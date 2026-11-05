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
final class TransportState {
  const TransportState({
    required this.playing,
    required this.failed,
    required this.tick,
    required this.tempoFactor,
    required this.mutedTracks,
    this.message,
  });

  const TransportState.initial()
    : playing = false,
      failed = false,
      tick = 0,
      tempoFactor = TempoFactorSteps.normal,
      mutedTracks = const {},
      message = null;

  final bool playing;
  final bool failed;

  /// Current position in ticks of the timeline (ADR-005 amendment 2: an `int`).
  final int tick;

  /// User tempo factor in hundredths (100 = 1.0x), Q6.
  final int tempoFactor;

  /// Tracks the user has muted (by track id).
  final Set<int> mutedTracks;

  /// Last playback failure message (missing SoundFont, lost device), else `null`.
  final String? message;

  bool isMuted(int track) => mutedTracks.contains(track);

  String get tempoLabel => TempoFactorSteps.label(tempoFactor);

  TransportState copyWith({
    bool? playing,
    bool? failed,
    int? tick,
    int? tempoFactor,
    Set<int>? mutedTracks,
    String? message,
    bool clearMessage = false,
  }) => TransportState(
    playing: playing ?? this.playing,
    failed: failed ?? this.failed,
    tick: tick ?? this.tick,
    tempoFactor: tempoFactor ?? this.tempoFactor,
    mutedTracks: mutedTracks ?? this.mutedTracks,
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
    state = state.copyWith(
      tick: p.tick,
      playing: p.playing,
      failed: p.state.isFailed,
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
}
