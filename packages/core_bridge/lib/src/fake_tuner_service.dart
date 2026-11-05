import 'generated/api/types.dart' show TunerReadingDto;
import 'tuner_service.dart';

/// In-memory tuner for widget tests: a scripted sequence of readings, no microphone.
/// Tests push readings and the view polls `reading`.
final class FakeTunerService implements TunerService {
  FakeTunerService({this.startFails});

  /// If set, [FakeTunerFactory.start] throws this instead of returning a service (to
  /// test the no-device / permission-denied path, T2).
  final Object? startFails;

  TunerReadingDto _next = const TunerReadingDto(
    hasPitch: false,
    hz: 0,
    clarity: 0,
    string: 0,
    cents: 0,
    targetHz: 0,
    updates: 0,
  );
  bool _failed = false;
  bool _closed = false;
  int _updates = 0;

  /// Test hook: set the reading the next `reading()` call returns.
  void push({
    required bool hasPitch,
    double hz = 0,
    double clarity = 0,
    int string = 0,
    double cents = 0,
    double targetHz = 0,
  }) {
    _updates++;
    _next = TunerReadingDto(
      hasPitch: hasPitch,
      hz: hz,
      clarity: clarity,
      string: string,
      cents: cents,
      targetHz: targetHz,
      updates: _updates,
    );
  }

  /// Test hook: simulate the input device being lost.
  void fail() => _failed = true;

  @override
  TunerReadingDto reading(int track) => _next;

  @override
  bool get failed => _closed || _failed;

  @override
  void close() => _closed = true;
}

/// Fake factory: returns a [FakeTunerService], or throws if `startFails` is set.
final class FakeTunerFactory implements TunerFactory {
  FakeTunerFactory({this.service});

  /// The service to hand out; a fresh one if null. If its `startFails` is set, [start]
  /// throws it.
  final FakeTunerService? service;

  @override
  Future<TunerService> start() async {
    final s = service ?? FakeTunerService();
    if (s.startFails != null) {
      throw s.startFails!;
    }
    return s;
  }
}
