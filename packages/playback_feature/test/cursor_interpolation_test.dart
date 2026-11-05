import 'dart:async';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:playback_feature/playback_feature.dart';

/// A playback service whose position stream the test drives by hand, so it can place two
/// samples at exact ticks and let the controller stamp them with a controlled clock.
class _ManualService implements PlaybackService {
  final _positions = StreamController<PositionDto>.broadcast();

  void emit(int tick, {bool playing = true}) {
    _positions.add(
      PositionDto(
        tick: tick,
        playing: playing,
        state: playing ? PlaybackStateDto.playing : PlaybackStateDto.stopped,
      ),
    );
  }

  @override
  Stream<PositionDto> get positions => _positions.stream;
  @override
  Future<void> play({int? fromMasterBar}) async {}
  @override
  Future<void> stop() async {}
  @override
  Future<void> seek(int tick) async {}
  @override
  Future<void> setMuted(int track, bool muted) async {}
  @override
  Future<void> setTempoFactor(int hundredths) async {}
  @override
  Future<void> refreshAfterEdit() async {}
  @override
  PositionDto position() =>
      const PositionDto(tick: 0, playing: false, state: PlaybackStateDto.stopped);
  @override
  String? get lastError => null;
  @override
  void close() => unawaited(_positions.close());
}

void main() {
  test('the cursor tick is extrapolated between position frames', () async {
    final service = _ManualService();
    final container = ProviderContainer(
      overrides: [playbackServiceProvider.overrideWithValue(service)],
    );
    addTearDown(container.dispose);
    addTearDown(service.close);

    final controller = container.read(transportProvider.notifier);
    // Controlled monotonic clock in microseconds.
    var now = 0;
    controller.nowMicros = () => now;

    // Two samples 16 ms apart, tick advancing 480 per frame (so 30 ticks/ms).
    now = 1_000_000;
    service.emit(0);
    await pumpEventQueue();
    now = 1_016_000;
    service.emit(480);
    await pumpEventQueue();

    final state = container.read(transportProvider);
    expect(state.playing, isTrue);
    expect(state.sampleTick, 480);

    // Right at the last sample: exactly 480.
    now = 1_016_000;
    expect(controller.displayTick(), 480);

    // Halfway to the next expected frame (8 ms later): velocity is 480/16ms, so +240.
    now = 1_024_000;
    final mid = controller.displayTick();
    expect(mid, inInclusiveRange(700, 740),
        reason: 'interpolated ~720, not a step to 480 or 960');

    // A full frame later (16 ms): ~ +480 again, smooth continuation.
    now = 1_032_000;
    final full = controller.displayTick();
    expect(full, inInclusiveRange(940, 980));

    // The cursor never runs backwards before the last sample.
    now = 1_015_000; // earlier than the last sample (clock skew guard)
    expect(controller.displayTick(), 480);
  });

  test('when stopped the cursor holds the last tick (no extrapolation)', () async {
    final service = _ManualService();
    final container = ProviderContainer(
      overrides: [playbackServiceProvider.overrideWithValue(service)],
    );
    addTearDown(container.dispose);
    addTearDown(service.close);
    final controller = container.read(transportProvider.notifier);
    var now = 0;
    controller.nowMicros = () => now;

    now = 500_000;
    service.emit(1000, playing: true);
    await pumpEventQueue();
    now = 516_000;
    service.emit(1000, playing: false); // stopped at 1000
    await pumpEventQueue();

    now = 600_000; // much later
    expect(controller.displayTick(), 1000, reason: 'held, not extrapolated');
  });
}
