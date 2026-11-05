import 'dart:async';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Starts a tuner bound to the active document (ADR-029). The app overrides it; widget
/// tests use a `FakeTunerFactory`. `null` means the tuner cannot be built here (wiring
/// bug); the controller surfaces it as an error, not a crash.
final tunerFactoryProvider = Provider<TunerFactory>(
  (ref) => throw UnimplementedError('override tunerFactoryProvider'),
);

/// The track the tuner references (its tuning + capo, T8). The app overrides it with the
/// editor's active track; when the user switches tracks or retunes, the next reading
/// follows with no restart.
final tunerTrackProvider = Provider<int>(
  (ref) => throw UnimplementedError('override tunerTrackProvider'),
);

/// What the tuner view shows. All derived from the latest core reading plus the UI
/// gates; no musical rule here (GR-05).
final class TunerState {
  const TunerState({
    required this.listening,
    required this.hasPitch,
    required this.hz,
    required this.clarity,
    required this.string,
    required this.cents,
    this.message,
  });

  const TunerState.off()
    : listening = false,
      hasPitch = false,
      hz = 0,
      clarity = 0,
      string = 0,
      cents = 0,
      message = null;

  /// True while the microphone is open.
  final bool listening;

  /// True when a confident pitch is present (above the clarity floor).
  final bool hasPitch;
  final double hz;
  final double clarity;

  /// Nearest open string (1-based), 0 when no pitch.
  final int string;

  /// Cents off the nearest string (positive is sharp).
  final double cents;

  /// Error or status message (no microphone, permission denied, device lost).
  final String? message;

  bool get inTune => hasPitch && TunerDisplay.inTune(cents);

  TunerState copyWith({
    bool? listening,
    bool? hasPitch,
    double? hz,
    double? clarity,
    int? string,
    double? cents,
    String? message,
    bool clearMessage = false,
  }) => TunerState(
    listening: listening ?? this.listening,
    hasPitch: hasPitch ?? this.hasPitch,
    hz: hz ?? this.hz,
    clarity: clarity ?? this.clarity,
    string: string ?? this.string,
    cents: cents ?? this.cents,
    message: clearMessage ? null : (message ?? this.message),
  );
}

final tunerProvider = NotifierProvider<TunerController, TunerState>(
  TunerController.new,
);

/// Owns the tuner lifecycle: opens the microphone on [enable], polls the core at ~30 Hz,
/// and releases it on [disable] or dispose (the mic is live only while enabled). No
/// musical rule here; mapping a frequency to a string/cents is done in the core.
final class TunerController extends Notifier<TunerState> {
  TunerService? _service;
  Timer? _poll;

  /// Poll interval; ~30 Hz. Exposed for tests to drive manually.
  static const pollInterval = Duration(milliseconds: 33);

  @override
  TunerState build() {
    // Release the microphone on dispose without touching state (setting state during
    // dispose is illegal in Riverpod). `disable()` is for user-driven stop.
    ref.onDispose(_release);
    return const TunerState.off();
  }

  void _release() {
    _poll?.cancel();
    _poll = null;
    _service?.close();
    _service = null;
  }

  /// Open the microphone and start polling. Safe to call when already enabled.
  Future<void> enable() async {
    if (_service != null) return;
    try {
      _service = await ref.read(tunerFactoryProvider).start();
      state = state.copyWith(listening: true, clearMessage: true);
      _poll = Timer.periodic(pollInterval, (_) => _tick());
    } on ApiError catch (e) {
      state = const TunerState.off().copyWith(message: _friendly(e));
    }
  }

  /// Stop and release the microphone (user-driven). Resets the view to "off".
  void disable() {
    _release();
    state = const TunerState.off();
  }

  /// One poll of the core reading. Public so tests can step it deterministically.
  void poll() => _tick();

  void _tick() {
    final service = _service;
    if (service == null) return;
    if (service.failed) {
      disable();
      state = const TunerState.off().copyWith(
        message: 'The microphone was lost. Reopen the tuner to try again.',
      );
      return;
    }
    final track = ref.read(tunerTrackProvider);
    final r = service.reading(track);
    final confident = r.hasPitch && r.clarity >= TunerDisplay.clarityFloor;
    state = state.copyWith(
      listening: true,
      hasPitch: confident,
      hz: r.hz,
      clarity: r.clarity,
      string: confident ? r.string : 0,
      cents: confident ? r.cents : 0,
      clearMessage: true,
    );
  }

  String _friendly(ApiError e) => switch (e.rule) {
    'tuner-no-device' => 'No microphone found. Connect one to use the tuner.',
    'tuner-permission' =>
      'Microphone access is blocked. Allow it in Windows privacy settings.',
    _ => 'The tuner could not start: ${e.message}',
  };
}
