import 'generated/api/types.dart' show TunerReadingDto;

/// A running microphone tuner bound to one document (ADR-029). The audio side produces a
/// frequency; the reading is mapped to the document track's strings (its tuning and capo,
/// T8) in the Rust core, not in Dart (GR-05). Implementations: the real one backed by
/// `mc-audio` through `mc-ffi`, and `FakeTunerService` for widget tests. Both pass the
/// same contract.
///
/// The microphone is live only while the service exists; [close] releases it. Start can
/// fail with [ApiError] (no microphone, permission denied, device busy): the UI shows the
/// message and keeps the chord finder and fretboard working (T2).
abstract interface class TunerService {
  /// Latest reading for [track] (a track id in the bound document). Poll at the UI rate
  /// (~30 Hz). `hasPitch == false` means no confident pitch right now (show "—").
  TunerReadingDto reading(int track);

  /// True if the input device was lost after starting; the caller should [close] and
  /// show a message.
  bool get failed;

  /// Stop and release the microphone. The service must not be used afterwards.
  void close();
}

/// Starts a tuner bound to a document. Separate from playback: a tuner and playback do
/// not share state, and the tuner must work even when playback is unavailable.
abstract interface class TunerFactory {
  /// Open the default microphone and start detecting for [documentId]'s document. Throws
  /// [ApiError] with rule `tuner-no-device`, `tuner-permission` or `tuner` on failure.
  Future<TunerService> start();
}

/// Cents/Hz helpers and the "in tune" band for the UI, shared by views and tests.
abstract final class TunerDisplay {
  /// Within this many cents of a string is shown as "in tune".
  static const double inTuneCents = 5.0;

  static bool inTune(double cents) => cents.abs() <= inTuneCents;

  /// Clarity below this is treated as "no confident pitch" by the UI even if the core
  /// returned one (a second gate on top of the detector's; keeps the needle calm).
  static const double clarityFloor = 0.6;
}
