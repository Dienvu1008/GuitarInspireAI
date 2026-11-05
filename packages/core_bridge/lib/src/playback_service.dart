import 'generated/api/midi.dart' show ExportResult, ImportReportDto;
import 'generated/api/types.dart' show PlaybackStateDto, PositionDto;

import 'document_service.dart';

/// Plays a document and reports its position. All playback decisions (what to compile,
/// start bar, mute, swap on edit) are made by the Rust core (ADR-028); this interface
/// only forwards transport commands and surfaces state. Implementations: the real one
/// backed by `mc-audio` through `mc-ffi`, and `FakePlaybackService` for widget tests.
/// Both pass the same contract suite so the fake cannot drift.
///
/// A service is bound to one [DocumentService] at creation. Transport changes are
/// session state: they never edit the document, never bump its revision and are not
/// saved. They do arrive on the document's event stream as `PlaybackStateChanged`,
/// `MuteChanged` and `TempoFactorChanged` (ADR-028).
abstract interface class PlaybackService {
  /// Start playback. [fromMasterBar] is the bar containing the cursor (Q7); `null`
  /// resumes from the current position. Throws [ApiError] on an audio failure or a bar
  /// past the end.
  Future<void> play({int? fromMasterBar});

  Future<void> stop();

  /// Seek to a tick on the current timeline (click-to-seek, Q7).
  Future<void> seek(int tick);

  /// Mute or unmute a track. While playing, the core recompiles and swaps so the change
  /// is heard without a restart (Q8).
  Future<void> setMuted(int track, bool muted);

  /// Set the user tempo factor in hundredths (100 = 1.0x), 25..=200 in steps of 5 (Q6).
  Future<void> setTempoFactor(int hundredths);

  /// Recompile and swap after an edit while playing (ADR-028 item 5). A no-op when
  /// stopped. The controller calls this after a document command during playback.
  Future<void> refreshAfterEdit();

  /// The current position (one-off query).
  PositionDto position();

  /// Why playback is not working, if it is not (missing SoundFont, lost device);
  /// `null` when all is well.
  String? get lastError;

  /// A ~60 Hz stream of positions for the moving cursor. Rust owns the lifecycle
  /// (ADR-005 amendment 1): [close] releases the Rust sink.
  Stream<PositionDto> get positions;

  /// Releases the engine binding. The service must not be used afterwards. The shared
  /// engine itself is owned by the factory and outlives individual services.
  void close();
}

/// Creates playback services bound to a document. One factory owns one shared engine.
abstract interface class PlaybackFactory {
  /// A playback service for [document]. Many may share one underlying engine.
  PlaybackService forDocument(DocumentService document);
}

/// MIDI import and export (ADR-023). Independent of the audio engine (no device
/// needed), so it is a separate facade: the UI can import or export without playback
/// ever starting. Import builds a brand-new, unsaved document.
abstract interface class MidiService {
  /// Import a Standard MIDI File (its bytes) into a new, unsaved document. Returns the
  /// document and a report of what the import changed or dropped. Throws [ApiError] for
  /// a malformed or oversized file.
  Future<MidiImport> importMidi(List<int> bytes);

  /// Export [document] as a Standard MIDI File (type 1, 960 PPQ). All tracks are
  /// exported regardless of mute; the tempo factor is not applied (ADR-023 item 3).
  Future<ExportResult> exportMidi(DocumentService document);
}

/// A document produced by MIDI import, with the import report.
final class MidiImport {
  const MidiImport({required this.document, required this.report});

  final DocumentService document;
  final ImportReportDto report;
}

/// Tempo-factor steps shared by the UI (Q6): 0.25x..=2.0x in 0.05 (5-hundredth) steps.
abstract final class TempoFactorSteps {
  static const int min = 25;
  static const int max = 200;
  static const int step = 5;
  static const int normal = 100;

  /// Clamps and snaps [hundredths] to the allowed grid.
  static int snap(int hundredths) {
    final clamped = hundredths.clamp(min, max);
    final snapped = (clamped / step).round() * step;
    return snapped.clamp(min, max);
  }

  /// Human label like `1.0x` or `0.75x`.
  static String label(int hundredths) {
    final whole = hundredths ~/ 100;
    final frac = hundredths % 100;
    final text = frac == 0
        ? '$whole'
        : '$whole.${frac.toString().padLeft(2, '0').replaceAll(RegExp(r'0$'), '')}';
    return '${text}x';
  }
}

/// Convenience on [PlaybackStateDto] for the UI.
extension PlaybackStateView on PlaybackStateDto {
  bool get isPlaying => this == PlaybackStateDto.playing;
  bool get isFailed => this == PlaybackStateDto.failed;
}
