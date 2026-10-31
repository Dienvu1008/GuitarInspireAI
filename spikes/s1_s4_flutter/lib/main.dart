// Throwaway benchmark app for spikes S1 (interop) and S4 (TAB rendering).
// Usage (profile build): s1_s4_flutter.exe <s1|s4> <output.json> [stream_seconds]
// Runs the scenario automatically, writes JSON results, then exits.
import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:s1_s4_flutter/src/rust/api/spike.dart';
import 'package:s1_s4_flutter/src/rust/api/tab.dart';
import 'package:s1_s4_flutter/src/rust/frb_generated.dart';

final clock = Stopwatch()..start();

/// Frame timings with the clock time (microseconds) they were received.
final frames = <(int, FrameTiming)>[];

Future<void> main(List<String> args) async {
  WidgetsFlutterBinding.ensureInitialized(); // required before addTimingsCallback
  await RustLib.init();
  final scenario = args.isNotEmpty ? args[0] : 's1';
  final out = args.length > 1 ? args[1] : 'result.json';
  final seconds = args.length > 2 ? int.parse(args[2]) : 600;
  logPath = '$out.log';
  log('args=$args scenario=$scenario');
  SchedulerBinding.instance.addTimingsCallback((ts) {
    final now = clock.elapsedMicroseconds;
    for (final t in ts) {
      frames.add((now, t));
    }
  });
  runApp(MaterialApp(
    debugShowCheckedModeBanner: false,
    home: scenario == 's4' ? TabSpike(out: out) : InteropSpike(out: out, seconds: seconds),
  ));
}

Map<String, num> stats(List<int> v) {
  if (v.isEmpty) return {'n': 0};
  final s = [...v]..sort();
  int p(double q) => s[((s.length - 1) * q).floor()];
  return {'n': s.length, 'p50': p(0.5), 'p99': p(0.99), 'max': s.last};
}

/// Frame stats for frames received after [fromUs]. Frame cost = max(build, raster).
Map<String, num> frameStats(int fromUs, [int? toUs]) {
  final sel = frames.where((f) => f.$1 >= fromUs && (toUs == null || f.$1 <= toUs)).map((f) => f.$2);
  final cost = sel.map((t) => math.max(t.buildDuration.inMicroseconds, t.rasterDuration.inMicroseconds)).toList();
  final st = stats(cost);
  return {...st, 'over_16ms': cost.where((c) => c > 16000).length, 'over_33ms': cost.where((c) => c > 33000).length};
}

String logPath = 'spike.log';

void log(String msg) => File(logPath).writeAsStringSync('${clock.elapsedMilliseconds} ms: $msg\n', mode: FileMode.append);

Future<void> finish(String out, Map<String, Object?> result) async {
  await File(out).writeAsString(const JsonEncoder.withIndent('  ').convert(result));
  log('finished');
  exit(0);
}

/// Runs a scenario; any error is written to the result file instead of hanging.
Future<void> guarded(String out, Future<void> Function() body) async {
  try {
    await body();
  } catch (e, st) {
    log('error: $e');
    await finish(out, {'error': '$e', 'stack': '$st'});
  }
}

// ---------------------------------------------------------------- S1

class InteropSpike extends StatefulWidget {
  const InteropSpike({super.key, required this.out, required this.seconds});
  final String out;
  final int seconds;
  @override
  State<InteropSpike> createState() => _InteropSpikeState();
}

class _InteropSpikeState extends State<InteropSpike> with SingleTickerProviderStateMixin {
  late final AnimationController spin = AnimationController(vsync: this, duration: const Duration(seconds: 2))..repeat();
  final position = ValueNotifier<int>(0);
  String status = 'starting';

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => guarded(widget.out, run));
  }

  Future<List<int>> timeCalls(int n, Future<void> Function() call) async {
    final out = <int>[];
    final sw = Stopwatch();
    for (var i = 0; i < n; i++) {
      sw
        ..reset()
        ..start();
      await call();
      out.add(sw.elapsedMicroseconds);
    }
    return out;
  }

  Future<void> run() async {
    final r = <String, Object?>{};
    final doc = DocumentHandle();
    final events = <EventDto>[];
    final sub = doc.events().listen(events.add);
    log('s1: subscribed');

    await timeCalls(500, () => doc.execute(cmd: const CommandKind.noop())); // warm-up
    log('s1: warm-up done');
    r['execute_threadpool_us'] = stats(await timeCalls(10000, () => doc.execute(cmd: const CommandKind.noop())));
    log('s1: threadpool done');
    r['execute_async_rust_us'] = stats(await timeCalls(10000, () => doc.executeAsync(cmd: const CommandKind.noop())));
    log('s1: async rust done');
    final sync = <int>[];
    final sw = Stopwatch();
    for (var i = 0; i < 10000; i++) {
      sw
        ..reset()
        ..start();
      doc.revisionSync();
      sync.add(sw.elapsedMicroseconds);
    }
    r['sync_query_us'] = stats(sync);
    log('s1: sync done');

    final o = await doc.execute(cmd: const CommandKind.insertNote(stringIndex: 6, fret: 3));
    log('s1: insertNote done');
    await doc.execute(cmd: const CommandKind.setTempo(bpmNum: 120, bpmDen: 1));
    log('s1: setTempo done');
    await Future<void>.delayed(const Duration(milliseconds: 300));
    log('s1: delay done, events ${events.length}');
    final seqs = events.map((e) => e.seq.toInt()).toList();
    var ordered = true;
    for (var i = 1; i < seqs.length; i++) {
      if (seqs[i] != seqs[i - 1] + 1) ordered = false;
    }
    r['mirrored_payload_enum_ok'] = o.revision > BigInt.zero;
    r['events'] = {'expected': o.revision.toInt(), 'received': events.length, 'gapless_in_order': ordered, 'first_seq': seqs.first, 'last_seq': seqs.last};
    // Lifecycle probe: does cancelling the Dart subscription complete while Rust still holds the sink?
    var cancelCompleted = true;
    await sub.cancel().timeout(const Duration(seconds: 3), onTimeout: () => cancelCompleted = false);
    r['dart_cancel_completes_while_rust_holds_sink'] = cancelCompleted;
    final eventsAfterCancel = events.length;
    await doc.execute(cmd: const CommandKind.noop());
    await Future<void>.delayed(const Duration(milliseconds: 200));
    r['events_delivered_after_cancel'] = events.length - eventsAfterCancel;

    log('s1: events checked: ${r['events']}');
    setState(() => status = 'streaming ${widget.seconds}s at 60 Hz');
    final arrivals = <int>[];
    final done = Completer<void>();
    final streamStart = clock.elapsedMicroseconds;
    final psub = positionStream(hz: 60, seconds: widget.seconds).listen((p) {
      arrivals.add(clock.elapsedMicroseconds);
      position.value = p.tick.toInt();
    }, onDone: done.complete);
    await done.future;
    await psub.cancel();
    final gaps = <int>[for (var i = 1; i < arrivals.length; i++) arrivals[i] - arrivals[i - 1]];
    final spanS = (arrivals.last - arrivals.first) / 1e6;
    r['position_stream'] = {
      'seconds': widget.seconds,
      'received': arrivals.length,
      'expected': 60 * widget.seconds,
      'avg_rate_hz': (arrivals.length - 1) / spanS,
      'gap_us': stats(gaps),
      'gaps_over_50ms': gaps.where((g) => g > 50000).length,
    };
    await Future<void>.delayed(const Duration(milliseconds: 1500)); // flush frame timings
    r['ui_frames_during_stream_after_2s_warmup'] = frameStats(streamStart + 2000000);
    await finish(widget.out, r);
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        body: Center(
          child: Column(mainAxisSize: MainAxisSize.min, children: [
            RotationTransition(turns: spin, child: Container(width: 80, height: 80, color: Colors.teal)),
            const SizedBox(height: 16),
            ValueListenableBuilder<int>(valueListenable: position, builder: (_, v, _) => Text('tick $v')),
            Text(status),
          ]),
        ),
      );
}

// ---------------------------------------------------------------- S4

class TabSpike extends StatefulWidget {
  const TabSpike({super.key, required this.out});
  final String out;
  @override
  State<TabSpike> createState() => _TabSpikeState();
}

class _TabSpikeState extends State<TabSpike> {
  static const bars = 500;
  static const barsPerSystem = 4;
  final score = TabScore.generate(bars: bars, seed: BigInt.one);
  final controller = ScrollController();
  final cache = <int, SystemLayout>{};
  bool useCache = true;
  final callUs = <int>[];
  final rustUs = <int>[];
  final payloadBytes = <int>[];
  late final List<TextPainter> digits;
  late final double digitWidth;
  LayoutMetrics? metrics;

  @override
  void initState() {
    super.initState();
    const style = TextStyle(fontSize: 12, color: Colors.black, fontFeatures: [FontFeature.tabularFigures()]);
    digits = [for (var f = 0; f < 25; f++) TextPainter(text: TextSpan(text: '$f', style: style), textDirection: TextDirection.ltr)..layout()];
    digitWidth = digits[0].width; // metrics measured once in Dart, passed to Rust
    WidgetsBinding.instance.addPostFrameCallback((_) => guarded(widget.out, run));
  }

  SystemLayout layoutOf(int i) {
    if (useCache && cache.containsKey(i)) return cache[i]!;
    final sw = Stopwatch()..start();
    final l = score.layoutSystem(index: i, metrics: metrics!);
    callUs.add(sw.elapsedMicroseconds);
    rustUs.add(l.layoutUs);
    payloadBytes.add(l.lines.lengthInBytes + l.glyphPos.lengthInBytes + l.glyphFret.lengthInBytes);
    if (useCache) cache[i] = l;
    return l;
  }

  Future<void> scroll(double to) =>
      controller.animateTo(to, duration: const Duration(seconds: 20), curve: Curves.linear);

  Future<void> run() async {
    final r = <String, Object?>{
      'bars': bars,
      'notes': score.notesTotal(),
      'systems': score.systemCount(barsPerSystem: barsPerSystem),
      'window': '${View.of(context).physicalSize}',
    };
    log('s4 start ${r['window']}');
    await Future<void>.delayed(const Duration(seconds: 2)); // warm-up excluded
    log('warm-up done, frames so far ${frames.length}, max extent ${controller.position.maxScrollExtent}');
    final t1 = clock.elapsedMicroseconds;
    await scroll(controller.position.maxScrollExtent);
    final t2 = clock.elapsedMicroseconds;
    log('pass 1 done, frames ${frames.length}');
    r['pass1_down_layout_on_first_visit'] = frameStats(t1, t2);
    r['layout_call_us_with_ffi'] = stats(callUs);
    r['layout_rust_us'] = stats(rustUs);
    r['payload_bytes_per_system'] = stats(payloadBytes);
    setState(() {
      useCache = false; // pass 2: every build re-runs Rust layout and crosses FFI
      cache.clear();
    });
    callUs.clear();
    await Future<void>.delayed(const Duration(milliseconds: 500));
    final t3 = clock.elapsedMicroseconds;
    await scroll(0);
    final t4 = clock.elapsedMicroseconds;
    await Future<void>.delayed(const Duration(milliseconds: 1500));
    r['pass2_up_no_cache'] = frameStats(t3, t4);
    r['pass2_layout_call_us_with_ffi'] = stats(callUs);
    await finish(widget.out, r);
  }

  @override
  Widget build(BuildContext context) {
    final width = MediaQuery.sizeOf(context).width;
    metrics ??= LayoutMetrics(width: width, stringSpacing: 14, digitWidth: digitWidth, barsPerSystem: barsPerSystem, margin: 16);
    final systemHeight = 16 * 2 + 5 * 14 + 14 * 1.5;
    return Scaffold(
      body: ListView.builder(
        controller: controller,
        itemExtent: systemHeight,
        itemCount: score.systemCount(barsPerSystem: barsPerSystem),
        itemBuilder: (_, i) => RepaintBoundary(
          child: CustomPaint(size: Size(width, systemHeight), painter: SystemPainter(layoutOf(i), digits)),
        ),
      ),
    );
  }
}

class SystemPainter extends CustomPainter {
  SystemPainter(this.layout, this.digits);
  final SystemLayout layout;
  final List<TextPainter> digits;
  static final line = Paint()
    ..color = Colors.black54
    ..strokeWidth = 1;
  static final erase = Paint()..color = Colors.white;

  @override
  void paint(Canvas canvas, Size size) {
    final l = layout.lines;
    for (var i = 0; i + 3 < l.length; i += 4) {
      canvas.drawLine(Offset(l[i], l[i + 1]), Offset(l[i + 2], l[i + 3]), line);
    }
    final p = layout.glyphPos;
    final f = layout.glyphFret;
    for (var g = 0; g < f.length; g++) {
      final tp = digits[f[g]];
      final o = Offset(p[2 * g] - tp.width / 2, p[2 * g + 1] - tp.height / 2);
      canvas.drawRect(o & Size(tp.width, tp.height), erase);
      tp.paint(canvas, o);
    }
  }

  @override
  bool shouldRepaint(SystemPainter old) => !identical(old.layout, layout);
}
