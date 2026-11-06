// Edit feedback benchmark in the real app (performance-requirements: edit command +
// visible projection, p99 <= 16 ms on the typical score, <= 33 ms on the large one).
// Complements the Rust `edit_and_relayout` bench with what it cannot see: FFI
// transfer, the Dart refresh, widget rebuild, TAB painting and rasterization.
// Run in profile mode on the reference machine:
//
//   flutter build windows --profile -t tool/edit_latency.dart
//   build\windows\x64\runner\Profile\guitar_inspire.exe <out.json> [typical|large]
//
// The app builds the fixture shape of mc-score (`fixtures::TYPICAL` / `LARGE`),
// warms up, then times 200 note edits in the visible part of the active track.
import 'dart:convert';
import 'dart:io';

import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:guitar_inspire/src/shell.dart';

typedef _Shape = ({
  int tracks,
  int bars,
  int beats,
  int notes,
  int durationBase,
  int budgetUs,
});

// Same shapes as mc-score fixtures (crates/mc-score/src/fixtures.rs).
const _Shape _typical = (
  tracks: 4,
  bars: 150,
  beats: 8,
  notes: 2,
  durationBase: 3, // eighths fill 4/4
  budgetUs: 16000,
);
const _Shape _large = (
  tracks: 20,
  bars: 300,
  beats: 16,
  notes: 3,
  durationBase: 4, // sixteenths fill 4/4
  budgetUs: 33000,
);

const _warmup = 30;
const _samples = 200;

/// Scroll phase (ADR-031 P4): frames and pixels per frame.
const _scrollFrames = 300;
const _scrollStep = 40.0;

/// The scrollable of the TAB view (the list keyed `tab-systems`), found in the tree.
ScrollableState? _tabScrollable() {
  ScrollableState? found;
  void visit(Element e, bool inside) {
    if (found != null) return;
    final here = inside || e.widget.key == const ValueKey('tab-systems');
    if (here && e is StatefulElement && e.state is ScrollableState) {
      found = e.state as ScrollableState;
      return;
    }
    e.visitChildElements((c) => visit(c, here));
  }

  final root = WidgetsBinding.instance.rootElement;
  if (root != null) visit(root, false);
  return found;
}

Future<void> main(List<String> args) async {
  final shapeName = args.length > 1 ? args[1] : 'typical';
  final shape = shapeName == 'large' ? _large : _typical;
  // Optional: how many tracks are shown (the first N); default every track.
  final showCount = args.length > 2 ? int.parse(args[2]) : shape.tracks;

  WidgetsFlutterBinding.ensureInitialized();
  await CoreBridge.init();
  final container = ProviderContainer(
    overrides: [
      documentFactoryProvider.overrideWithValue(CoreBridge.documents),
    ],
  );
  runApp(
    UncontrolledProviderScope(
      container: container,
      child: const GuitarInspireApp(),
    ),
  );
  await WidgetsBinding.instance.endOfFrame;

  final setup = Stopwatch()..start();
  final doc = await _score(shape);
  final setupMs = setup.elapsedMilliseconds;

  final track = doc.summary().tracks.first.id;
  final beats = doc.beats(track, 1, 0); // bar 2: visible without scrolling
  CommandDto noteEdit(int i) => CommandDto.setNote(
    beat: BeatAddressDto(
      track: track,
      bar: 1,
      voice: 0,
      beat: beats[i % beats.length].id,
    ),
    string: 1 + i % 6,
    fret: i % 20,
  );

  // Parts, measured before the document is shown, so no frame work interleaves:
  // command round trip, summary query and layout queries. Every track is shown, as
  // in the app by default (ADR-031): outline plus the systems in view (three).
  final command = <int>[];
  final summary = <int>[];
  final layout = <int>[];
  final scoreLayout = <int>[];
  final metrics = measureTabMetrics(width: 1200);
  final shown = [for (final t in doc.summary().tracks) t.id]
      .take(showCount)
      .toList();
  for (var i = 0; i < _warmup + 100; i++) {
    final w = Stopwatch()..start();
    await doc.execute(noteEdit(i));
    final c = w.elapsedMicroseconds;
    w.reset();
    doc.summary();
    final s = w.elapsedMicroseconds;
    w.reset();
    doc.tabLayout(track, metrics);
    final l = w.elapsedMicroseconds;
    w.reset();
    doc.scoreOutline(shown, metrics);
    doc.scoreSystems(shown, metrics, 0, 3);
    final m = w.elapsedMicroseconds;
    if (i >= _warmup) {
      command.add(c);
      summary.add(s);
      layout.add(l);
      scoreLayout.add(m);
    }
  }

  container.read(editorProvider.notifier).replace(doc);
  // Hide the tracks beyond the first N, as a user would from the track list.
  for (final t in doc.summary().tracks.skip(showCount)) {
    container.read(editorProvider.notifier).toggleTrackShown(t.id);
  }
  await WidgetsBinding.instance.endOfFrame;
  final editor = container.read(editorProvider.notifier);

  final timings = <FrameTiming>[];
  final edits = <int>[];
  Future<void> edit(int i) async {
    final watch = Stopwatch()..start();
    await editor.execute(noteEdit(i));
    await WidgetsBinding.instance.endOfFrame;
    if (i >= _warmup) edits.add(watch.elapsedMicroseconds);
  }

  for (var i = 0; i < _warmup; i++) {
    await edit(i);
  }
  SchedulerBinding.instance.addTimingsCallback(timings.addAll);
  for (var i = _warmup; i < _warmup + _samples; i++) {
    await edit(i);
  }
  // Frame timings are delivered in batches; give the last batch time to arrive.
  await Future<void>.delayed(const Duration(milliseconds: 500));
  SchedulerBinding.instance.removeTimingsCallback(timings.addAll);

  // ADR-031 P4: scroll the TAB view continuously (every track shown), one step per
  // frame, wrapping to the top at the end, and time every frame.
  final scrollTimings = <FrameTiming>[];
  final scrollable = _tabScrollable();
  if (scrollable != null) {
    final position = scrollable.position;
    SchedulerBinding.instance.addTimingsCallback(scrollTimings.addAll);
    for (var i = 0; i < _scrollFrames; i++) {
      final next = position.pixels + _scrollStep;
      position.jumpTo(next > position.maxScrollExtent ? 0 : next);
      await WidgetsBinding.instance.endOfFrame;
    }
    await Future<void>.delayed(const Duration(milliseconds: 500));
    SchedulerBinding.instance.removeTimingsCallback(scrollTimings.addAll);
  }
  final scrollCompute = [
    for (final t in scrollTimings)
      t.buildDuration.inMicroseconds + t.rasterDuration.inMicroseconds,
  ];

  final raster = [for (final t in timings) t.rasterDuration.inMicroseconds];
  final build = [for (final t in timings) t.buildDuration.inMicroseconds];
  int p(List<int> v, double q) => _percentile(v, q);
  final result = {
    'shape':
        '$shapeName: ${shape.tracks} tracks x ${shape.bars} bars x '
        '${shape.beats} beats x ${shape.notes} notes',
    'setup_ms': setupMs,
    'samples': edits.length,
    'edit_to_frame_us': _stats(edits),
    'command_round_trip_us': _stats(command),
    'summary_query_us': _stats(summary),
    'tab_layout_query_us': _stats(layout),
    // ADR-031: outline of every track plus three systems in view (FFI included).
    'score_layout_in_view_query_us': _stats(scoreLayout),
    'shown_tracks': shown.length,
    // ADR-031 P4: frames while scrolling, build plus raster per frame.
    'scroll_frames': scrollCompute.length,
    'scroll_frame_compute_us': _stats(scrollCompute),
    'scroll_frames_over_33ms': scrollCompute.where((us) => us > 33000).length,
    'frame_build_us': _stats(build),
    'frame_raster_us': _stats(raster),
    // edit_to_frame includes waiting for the next vsync (each edit starts right after
    // a frame), so it is about one refresh interval by construction. The budget is
    // about compute: command + frame build + raster. Summing the three p99 values is
    // a pessimistic bound, since the slowest of each rarely coincide.
    'p99_compute_bound_us': p(command, 0.99) + p(build, 0.99) + p(raster, 0.99),
    'p50_compute_us': p(command, 0.50) + p(build, 0.50) + p(raster, 0.50),
    // Edits whose feedback missed the next 60 Hz frame (over 1.25 intervals).
    'missed_next_frame': edits.where((us) => us > 20800).length,
    'budget_us': shape.budgetUs,
  };
  final json = const JsonEncoder.withIndent('  ').convert(result);
  if (args.isNotEmpty) {
    await File(args.first).writeAsString(json);
  }
  stdout.writeln(json);
  exit(0);
}

/// Builds the fixture directly on the service (no UI refresh per command).
Future<DocumentService> _score(_Shape shape) async {
  final doc = CoreBridge.documents.create(bars: shape.bars);
  final first = doc.summary().tracks.single;
  for (var t = 1; t < shape.tracks; t++) {
    await doc.execute(
      CommandDto.addTrack(name: 'Track ${t + 1}', instrument: first.instrument),
    );
  }
  for (final track in doc.summary().tracks) {
    for (var bar = 0; bar < shape.bars; bar++) {
      for (var b = 0; b < shape.beats; b++) {
        await doc.execute(
          CommandDto.insertBeat(
            track: track.id,
            bar: bar,
            voice: 0,
            index: b,
            duration: DurationDto(base: shape.durationBase, dots: 0),
          ),
        );
      }
      for (final (b, beat) in doc.beats(track.id, bar, 0).indexed) {
        final address = BeatAddressDto(
          track: track.id,
          bar: bar,
          voice: 0,
          beat: beat.id,
        );
        // Notes on distinct strings: 1-2, 3-4, 5-6 pattern per beat.
        for (var n = 0; n < shape.notes; n++) {
          await doc.execute(
            CommandDto.setNote(
              beat: address,
              string: 1 + n * 2 + b % 2,
              fret: (b + bar + n) % 15,
            ),
          );
        }
      }
    }
  }
  return doc;
}

Map<String, int> _stats(List<int> v) => {
  'p50': _percentile(v, 0.50),
  'p90': _percentile(v, 0.90),
  'p99': _percentile(v, 0.99),
  'max': v.isEmpty ? 0 : v.reduce((a, b) => a > b ? a : b),
};

int _percentile(List<int> values, double p) {
  if (values.isEmpty) return 0;
  final sorted = [...values]..sort();
  final index = ((sorted.length - 1) * p).round();
  return sorted[index];
}
