// M3 acceptance flow against the real Rust core, in the built Windows app:
//   flutter test integration_test -d windows        (from app/)
// Covers: a 2-track song, switching the active TAB track, keyboard note entry,
// undo/redo, save, reopen identically, recovery after a lost session, and the Mixer
// panel (ADR-032 X14).
// Native dialogs and the per-user recovery folder are replaced by fakes that point
// into a temporary directory; everything else is the production wiring.
import 'dart:io';

import 'package:app_platform/app_platform.dart';
import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart' show LogicalKeyboardKey;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:guitar_inspire/src/musicxml_menu.dart';
import 'package:guitar_inspire/src/playback_wiring.dart';
import 'package:guitar_inspire/src/shell.dart';
import 'package:guitar_inspire/src/startup.dart';
import 'package:integration_test/integration_test.dart';
import 'package:playback_feature/playback_feature.dart';

final class _Dialogs implements FileDialogs {
  String? openPath;
  String? savePath;
  List<int>? midiBytes;
  String? midiSavePath;
  @override
  Future<String?> pickProjectToOpen() async => openPath;
  @override
  Future<String?> pickProjectToSave({required String suggestedName}) async =>
      savePath;
  @override
  Future<List<int>?> pickMidiToImport() async => midiBytes;
  String? musicXmlSavePath;
  @override
  Future<String?> saveMusicXml(
    List<int> bytes, {
    required String suggestedName,
  }) async {
    final path = musicXmlSavePath;
    if (path == null) return null;
    await File(path).writeAsBytes(bytes);
    return path;
  }

  @override
  Future<String?> saveMidi(
    List<int> bytes, {
    required String suggestedName,
  }) async => midiSavePath;
}

final class _Paths implements AppPaths {
  _Paths(this.dir);
  final String dir;
  @override
  Future<String> recoveryDirectory() async {
    await Directory(dir).create(recursive: true);
    return dir;
  }
}

/// Everything the user can see of the song, for "reopen identically".
List<Object> _content(EditorState s) => [
  for (final t in s.summary.tracks) ...[
    t.name,
    t.instrument.tuning.join(' '),
    for (var bar = 0; bar < s.summary.masterBars.length; bar++)
      for (final b in s.service.beats(t.id, bar, 0))
        '$bar:${b.duration.base}.${b.duration.dots}:'
            '${b.notes.map((n) => '${n.string}/${n.fret}').join(',')}',
  ],
];

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  late Directory tmp;

  setUpAll(() async {
    await CoreBridge.init();
    tmp = await Directory.systemTemp.createTemp('gi_acceptance');
  });
  tearDownAll(() => tmp.delete(recursive: true));

  Future<ProviderContainer> start(
    WidgetTester tester,
    _Dialogs dialogs,
    String recoveryDir,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          documentFactoryProvider.overrideWithValue(CoreBridge.documents),
          fileDialogsProvider.overrideWithValue(dialogs),
          appPathsProvider.overrideWithValue(_Paths(recoveryDir)),
          midiServiceProvider.overrideWithValue(CoreBridge.midi),
          musicXmlServiceProvider.overrideWithValue(CoreBridge.musicXml),
        ],
        // Autosave is triggered explicitly in the recovery test.
        child: const StartupGate(autosavePoll: Duration(hours: 1)),
      ),
    );
    await _until(tester, () => find.byType(Shell).evaluate().isNotEmpty);
    return ProviderScope.containerOf(tester.element(find.byType(Shell)));
  }

  testWidgets('M3: two tracks, keyboard entry, undo/redo, save, reopen', (
    tester,
  ) async {
    final dialogs = _Dialogs();
    final c = await start(tester, dialogs, '${tmp.path}/recovery-a');
    EditorState s() => c.read(editorProvider);
    List<int> frets(int track) => [
      for (final b in s().service.beats(track, 0, 0))
        for (final n in b.notes) n.fret,
    ];

    // A second track, from the track list.
    await tester.tap(find.byKey(const ValueKey('add-track')));
    await _until(tester, () => s().summary.tracks.length == 2);
    final guitar = s().summary.tracks[0].id;
    final second = s().summary.tracks[1].id;
    expect(s().activeTrack, guitar);

    // Keyboard entry on track 1: "12" on string 1, then "3" on string 2.
    final latencies = <int>[];
    final watch = Stopwatch()..start();
    await tester.sendKeyEvent(LogicalKeyboardKey.digit1);
    await _until(tester, () => frets(guitar).isNotEmpty);
    latencies.add(watch.elapsedMicroseconds);
    await tester.sendKeyEvent(LogicalKeyboardKey.digit2);
    await _until(tester, () => frets(guitar).firstOrNull == 12);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    watch.reset();
    await tester.sendKeyEvent(LogicalKeyboardKey.digit3);
    await _until(tester, () => frets(guitar).length == 2);
    latencies.add(watch.elapsedMicroseconds);
    expect(frets(guitar), [12, 3]);

    // Undo and redo, one note per step.
    await _ctrl(tester, LogicalKeyboardKey.keyZ);
    await _until(tester, () => frets(guitar).length == 1);
    expect(frets(guitar), [12]);
    await _ctrl(tester, LogicalKeyboardKey.keyY);
    await _until(tester, () => frets(guitar).length == 2);

    // Switch the active TAB track and enter a note there.
    // The track list entry (the stave label shows the name too, ADR-031).
    await tester.tap(
      find.descendant(
        of: find.byType(TrackList),
        matching: find.text('Track 2'),
      ),
    );
    await _until(tester, () => s().activeTrack == second);
    await tester.sendKeyEvent(LogicalKeyboardKey.digit5);
    await _until(tester, () => frets(second).isNotEmpty);
    expect(frets(second), [5]);
    expect(frets(guitar), [12, 3], reason: 'the other track is unchanged');

    // Save (first save asks for a location).
    final path = '${tmp.path}${Platform.pathSeparator}song.giproj';
    dialogs.savePath = path;
    await _ctrl(tester, LogicalKeyboardKey.keyS);
    await _until(tester, () => !s().summary.dirty);
    expect(find.text('song.giproj'), findsOneWidget);
    final saved = _content(s());

    // New document, then reopen the file: identical content.
    await _ctrl(tester, LogicalKeyboardKey.keyN);
    await _until(tester, () => s().summary.location == null);
    dialogs.openPath = path;
    await _ctrl(tester, LogicalKeyboardKey.keyO);
    await _until(tester, () => s().summary.location == path);
    expect(_content(s()), saved);
    expect(s().summary.dirty, isFalse);

    debugPrint('key_to_state_us_debug: $latencies');
    binding.reportData = {
      'key_to_state_us_debug': latencies,
      'note': 'debug build; the budget is gated by the edit_and_relayout bench',
    };
  });

  testWidgets('M3: recovery after a lost session', (tester) async {
    final recovery = '${tmp.path}/recovery-b';
    final dialogs = _Dialogs();
    final c = await start(tester, dialogs, recovery);
    EditorState s() => c.read(editorProvider);

    await tester.sendKeyEvent(LogicalKeyboardKey.digit7);
    await _until(tester, () => _hasNote(c));
    final before = _content(s());
    expect(before.last, '0:2.0:1/7');
    // What the autosave timer does when the core says an autosave is due.
    await s().service.writeAutosave(await _Paths(recovery).recoveryDirectory());

    // "Kill": drop the whole widget tree without saving, then start again.
    await tester.pumpWidget(const SizedBox());
    final c2 = await _startExpectingRecovery(tester, dialogs, recovery);
    final r = c2.read(editorProvider);
    expect(_content(r), before);
    expect(r.summary.dirty, isTrue, reason: 'recovered work is not saved yet');

    // Saving the recovered document clears its recovery files.
    dialogs.savePath = '${tmp.path}${Platform.pathSeparator}recovered.giproj';
    await _ctrl(tester, LogicalKeyboardKey.keyS);
    await _until(tester, () => !c2.read(editorProvider).summary.dirty);
    expect(CoreBridge.documents.recoveryEntries(recovery), isEmpty);
  });

  // ADR-032 X14: mute, solo, volume and pan from the Mixer panel, with the real core
  // and the real audio engine. Kept in this file: on Windows a second integration test
  // file has to relaunch the app, which fails to start in the same `flutter test` run.
  // The engine needs an output device; without one (a CI runner) the test is skipped,
  // and the local run is the evidence.
  testWidgets('X14: mute, solo, volume and pan from the Mixer panel', (
    tester,
  ) async {
    final PlaybackFactory playback;
    try {
      playback = await CoreBridge.startPlayback();
    } on ApiError catch (e) {
      markTestSkipped('no audio output device: ${e.message}');
      return;
    }
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          documentFactoryProvider.overrideWithValue(CoreBridge.documents),
          fileDialogsProvider.overrideWithValue(_Dialogs()),
          appPathsProvider.overrideWithValue(
            _Paths('${tmp.path}/recovery-x14'),
          ),
          ...playbackOverrides(playback, CoreBridge.midi),
        ],
        child: const StartupGate(autosavePoll: Duration(hours: 1)),
      ),
    );
    await _until(tester, () => find.byType(Shell).evaluate().isNotEmpty);
    final c = ProviderScope.containerOf(tester.element(find.byType(Shell)));
    expect(find.byType(MixerPanel), findsOneWidget, reason: 'open by default');

    final id = c.read(editorProvider).summary.tracks.first.id;
    TransportState s() => c.read(transportProvider);

    await tester.tap(find.byKey(ValueKey('mixer-mute-$id')));
    await _until(tester, () => s().isMuted(id));
    expect(s().message, isNull, reason: 'the core accepted the mute');

    await tester.tap(find.byKey(ValueKey('mixer-solo-$id')));
    await _until(tester, () => s().isSoloed(id));
    expect(s().message, isNull, reason: 'the core accepted the solo');

    await tester.tap(find.byKey(ValueKey('mixer-mute-$id')));
    await _until(tester, () => !s().isMuted(id));

    final volume = tester.widget<Slider>(
      find.byKey(ValueKey('mixer-volume-$id')),
    );
    // A burst, as a slider drag would send.
    for (final v in [90, 80, 70, 60, 50]) {
      volume.onChanged!(v.toDouble());
      await tester.pump(const Duration(milliseconds: 5));
    }
    final pan = tester.widget<Slider>(find.byKey(ValueKey('mixer-pan-$id')));
    pan.onChanged!(-30);
    await _until(
      tester,
      () => s().mixOf(id) == const TrackMixView(volume: 50, pan: -30),
    );
    expect(s().message, isNull, reason: 'the core accepted the mix');

    // The real core rejects values outside the ranges, with a typed error.
    final service = c.read(playbackServiceProvider);
    for (final (v, p) in [(128, 0), (256, 0), (-1, 0), (100, 64), (100, -64)]) {
      await expectLater(
        service.setTrackMix(id, volume: v, pan: p),
        throwsA(isA<ApiError>()),
        reason: 'volume $v pan $p',
      );
    }
    // Reset returns to the defaults.
    await tester.tap(find.byKey(ValueKey('mixer-reset-$id')));
    await _until(tester, () => s().mixes.isEmpty);
  });

  // ADR-033 E13: File > Export MusicXML... in the built app with the real core. The
  // fake dialog writes the bytes to disk; the file read back must equal what the core
  // produces for the same document and hold one part per track. (The core has no
  // MusicXML import, so the file is checked as written, not re-imported.)
  testWidgets('E13: export MusicXML from the File menu', (tester) async {
    final dialogs = _Dialogs()
      ..musicXmlSavePath = '${tmp.path}${Platform.pathSeparator}song.musicxml';
    final c = await start(tester, dialogs, '${tmp.path}/recovery-e13');
    EditorState s() => c.read(editorProvider);
    await tester.tap(find.byKey(const ValueKey('add-track')));
    await _until(tester, () => s().summary.tracks.length == 2);
    await tester.sendKeyEvent(LogicalKeyboardKey.digit5);
    await _until(tester, () => _hasNote(c));

    await tester.tap(find.text('File'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('export-musicxml')));
    await _until(tester, () => File(dialogs.musicXmlSavePath!).existsSync());
    await _until(
      tester,
      () => find.textContaining('to MusicXML').evaluate().isNotEmpty,
    );

    final written = await File(dialogs.musicXmlSavePath!).readAsBytes();
    final again = await CoreBridge.musicXml.exportMusicXml(s().service);
    expect(written, again.bytes, reason: 'the file is what the core produced');
    final text = String.fromCharCodes(written);
    expect(RegExp('<part id=').allMatches(text).length, 2);
    expect(again.report.notes, 1);
    expect(text, contains('<fret>5</fret>'));
  });
}

Future<ProviderContainer> _startExpectingRecovery(
  WidgetTester tester,
  _Dialogs dialogs,
  String recoveryDir,
) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        documentFactoryProvider.overrideWithValue(CoreBridge.documents),
        fileDialogsProvider.overrideWithValue(dialogs),
        appPathsProvider.overrideWithValue(_Paths(recoveryDir)),
        midiServiceProvider.overrideWithValue(CoreBridge.midi),
      ],
      child: const StartupGate(autosavePoll: Duration(hours: 1)),
    ),
  );
  await _until(
    tester,
    () => find.text('Recover documents').evaluate().isNotEmpty,
  );
  // ADR-030: each entry has explicit Restore / Open original / Discard buttons.
  await tester.tap(find.widgetWithText(FilledButton, 'Restore').first);
  await _until(tester, () => find.byType(Shell).evaluate().isNotEmpty);
  final c = ProviderScope.containerOf(tester.element(find.byType(Shell)));
  await _until(tester, () => _hasNote(c));
  return c;
}

Future<void> _ctrl(WidgetTester tester, LogicalKeyboardKey key) async {
  await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
  await tester.sendKeyEvent(key);
  await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
}

/// Pumps frames until [done] holds. Core calls complete on worker threads, so the
/// test waits on observable state instead of a fixed number of frames.
Future<void> _until(WidgetTester tester, bool Function() done) async {
  final deadline = DateTime.now().add(const Duration(seconds: 15));
  while (!done()) {
    if (DateTime.now().isAfter(deadline)) {
      fail('timed out waiting for the editor');
    }
    await tester.pump(const Duration(milliseconds: 16));
  }
}

/// True when the first beat of bar 1 of the active track has a note.
bool _hasNote(ProviderContainer c) {
  final s = c.read(editorProvider);
  final beats = s.service.beats(s.activeTrack, 0, 0);
  return beats.isNotEmpty && beats.first.notes.isNotEmpty;
}
