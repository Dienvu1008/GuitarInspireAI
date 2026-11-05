import 'dart:async';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// The playback factory (shared engine). The app overrides it with the Rust-backed
/// factory from `CoreBridge.startPlayback`; widget tests use `FakePlaybackFactory`.
final playbackFactoryProvider = Provider<PlaybackFactory>(
  (ref) => throw UnimplementedError('override playbackFactoryProvider'),
);

/// MIDI import and export (no audio device). Overridden with `CoreBridge.midi`; tests
/// use `FakeMidiService`.
final midiServiceProvider = Provider<MidiService>(
  (ref) => throw UnimplementedError('override midiServiceProvider'),
);

/// Binds a [PlaybackService] to the document the editor is showing. The app wires this
/// so the transport plays the active document; tests override it directly.
final playbackServiceProvider = Provider<PlaybackService>(
  (ref) => throw UnimplementedError('override playbackServiceProvider'),
);

/// What the transport bar and cursor show. Playback is session state (ADR-028): none of
/// this edits the document or is saved. [tick] and [playing] come from the engine's
/// position stream; [mutedTracks] and [tempoFactor] are the user's current settings.
///
/// For a smooth cursor the state also carries the two most recent position samples, each
/// stamped with the real monotonic time it arrived ([sampleTick]/[sampleAtMicros] and the
/// previous pair). The cursor extrapolates between frames from the measured tick velocity
/// of those samples, not from a wall clock in the widget, so an uneven frame interval
/// does not make it jump (ADR-005 amendment 5).
final class TransportState {
  const TransportState({
    required this.playing,
    required this.failed,
    required this.tick,
    required this.tempoFactor,
    required this.mutedTracks,
    required this.sampleTick,
    required this.sampleAtMicros,
    required this.prevSampleTick,
    required this.prevSampleAtMicros,
    this.message,
  });

  const TransportState.initial()
    : playing = false,
      failed = false,
      tick = 0,
      tempoFactor = TempoFactorSteps.normal,
      mutedTracks = const {},
      sampleTick = 0,
      sampleAtMicros = 0,
      prevSampleTick = 0,
      prevSampleAtMicros = 0,
      message = null;

  final bool playing;
  final bool failed;

  /// Current position in ticks of the timeline (ADR-005 amendment 2: an `int`).
  final int tick;

  /// User tempo factor in hundredths (100 = 1.0x), Q6.
  final int tempoFactor;

  /// Tracks the user has muted (by track id).
  final Set<int> mutedTracks;

  /// Latest position sample and the monotonic microsecond timestamp it arrived at.
  final int sampleTick;
  final int sampleAtMicros;

  /// The sample before [sampleTick], for the velocity estimate.
  final int prevSampleTick;
  final int prevSampleAtMicros;

  /// Last playback failure message (missing SoundFont, lost device), else `null`.
  final String? message;

  bool isMuted(int track) => mutedTracks.contains(track);

  String get tempoLabel => TempoFactorSteps.label(tempoFactor);

  /// Measured tick velocity (ticks per microsecond) from the last two samples, or 0 when
  /// not playing or the samples are degenerate. Used to extrapolate the cursor.
  double get tickVelocityPerMicro {
    if (!playing) return 0;
    final dt = sampleAtMicros - prevSampleAtMicros;
    if (dt <= 0) return 0;
    return (sampleTick - prevSampleTick) / dt;
  }

  /// Extrapolated tick at monotonic time [nowMicros], clamped so it never runs backwards
  /// before the last sample. When stopped it is just the last tick.
  int tickAt(int nowMicros) {
    if (!playing) return tick;
    final ahead = nowMicros - sampleAtMicros;
    if (ahead <= 0) return sampleTick;
    final extra = (tickVelocityPerMicro * ahead).round();
    return sampleTick + (extra > 0 ? extra : 0);
  }

  TransportState copyWith({
    bool? playing,
    bool? failed,
    int? tick,
    int? tempoFactor,
    Set<int>? mutedTracks,
    int? sampleTick,
    int? sampleAtMicros,
    int? prevSampleTick,
    int? prevSampleAtMicros,
    String? message,
    bool clearMessage = false,
  }) => TransportState(
    playing: playing ?? this.playing,
    failed: failed ?? this.failed,
    tick: tick ?? this.tick,
    tempoFactor: tempoFactor ?? this.tempoFactor,
    mutedTracks: mutedTracks ?? this.mutedTracks,
    sampleTick: sampleTick ?? this.sampleTick,
    sampleAtMicros: sampleAtMicros ?? this.sampleAtMicros,
    prevSampleTick: prevSampleTick ?? this.prevSampleTick,
    prevSampleAtMicros: prevSampleAtMicros ?? this.prevSampleAtMicros,
    message: clearMessage ? null : (message ?? this.message),
  );
}

final transportProvider =
    NotifierProvider<TransportController, TransportState>(
      TransportController.new,
    );

/// Drives the playback engine for the active document. No musical rules here: it only
/// forwards transport commands and mirrors engine position and state (GR-05, ADR-028).
final class TransportController extends Notifier<TransportState> {
  PlaybackService? _service;
  StreamSubscription<PositionDto>? _positions;

  /// Monotonic clock for stamping position samples, in microseconds. A real stopwatch by
  /// default; tests override it to feed controlled timestamps. It measures elapsed real
  /// time at the moment a sample arrives, never a wall clock read inside a widget build.
  int Function() nowMicros = _defaultClock;

  static final Stopwatch _stopwatch = Stopwatch()..start();
  static int _defaultClock() => _stopwatch.elapsedMicroseconds;

  @override
  TransportState build() {
    final service = ref.watch(playbackServiceProvider);
    _service = service;
    _positions = service.positions.listen(_onPosition);
    ref.onDispose(() {
      unawaited(_positions?.cancel());
    });
    return const TransportState.initial();
  }

  void _onPosition(PositionDto p) {
    final now = nowMicros();
    state = state.copyWith(
      tick: p.tick,
      playing: p.playing,
      failed: p.state.isFailed,
      // Shift the newest sample into "previous" so the cursor can measure velocity.
      prevSampleTick: state.sampleTick,
      prevSampleAtMicros: state.sampleAtMicros,
      sampleTick: p.tick,
      sampleAtMicros: now,
      message: p.state.isFailed ? (_service?.lastError ?? state.message) : null,
      clearMessage: !p.state.isFailed && state.message != null,
    );
  }

  PlaybackService get _s =>
      _service ?? (throw StateError('no playback service'));

  Future<bool> _guard(Future<void> Function() action) async {
    try {
      await action();
      state = state.copyWith(clearMessage: true);
      return true;
    } on ApiError catch (e) {
      state = state.copyWith(message: '${e.rule}: ${e.message}');
      return false;
    }
  }

  /// Toggle play/stop. [fromMasterBar] is the bar containing the cursor (Q7); ignored
  /// when stopping.
  Future<void> toggle({int? fromMasterBar}) async {
    if (state.playing) {
      await stop();
    } else {
      await play(fromMasterBar: fromMasterBar);
    }
  }

  Future<bool> play({int? fromMasterBar}) =>
      _guard(() => _s.play(fromMasterBar: fromMasterBar));

  Future<bool> stop() => _guard(() => _s.stop());

  Future<bool> seek(int tick) => _guard(() => _s.seek(tick));

  Future<bool> setMuted(int track, bool muted) async {
    final next = Set<int>.of(state.mutedTracks);
    if (muted) {
      next.add(track);
    } else {
      next.remove(track);
    }
    final ok = await _guard(() => _s.setMuted(track, muted));
    if (ok) state = state.copyWith(mutedTracks: next);
    return ok;
  }

  Future<bool> toggleMute(int track) =>
      setMuted(track, !state.isMuted(track));

  /// Set the tempo factor in hundredths; snapped to the Q6 grid.
  Future<bool> setTempoFactor(int hundredths) async {
    final snapped = TempoFactorSteps.snap(hundredths);
    final ok = await _guard(() => _s.setTempoFactor(snapped));
    if (ok) state = state.copyWith(tempoFactor: snapped);
    return ok;
  }

  /// Step the tempo factor by one grid step (for the +/- buttons).
  Future<bool> nudgeTempo({required bool up}) => setTempoFactor(
    state.tempoFactor + (up ? TempoFactorSteps.step : -TempoFactorSteps.step),
  );

  /// Recompile and swap after an edit while playing (ADR-028 item 5). The app calls
  /// this after a document command when the transport is running.
  Future<void> refreshAfterEdit() async {
    if (state.playing) await _guard(() => _s.refreshAfterEdit());
  }

  /// The tick to draw the cursor at right now: the last sample extrapolated to the
  /// current monotonic time by the measured velocity (ADR-028, ADR-005 amendment 5). The
  /// cursor widget calls this every frame between position updates for smooth motion.
  int displayTick() => state.tickAt(nowMicros());
}
