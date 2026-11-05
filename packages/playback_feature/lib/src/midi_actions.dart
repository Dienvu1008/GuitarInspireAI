import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'transport_controller.dart';

/// MIDI import and export, independent of the audio engine (ADR-023). The UI reads and
/// writes the actual file (platform concern, GR-11); this controller only translates
/// bytes through the Rust core and reports what happened. No musical rules here.
final midiActionsProvider = Provider<MidiActions>(
  (ref) => MidiActions(ref.watch(midiServiceProvider)),
);

/// Result of an import: the new document service and a human-readable summary.
final class ImportOutcome {
  const ImportOutcome({required this.document, required this.summary});

  final DocumentService document;

  /// Short line for the status bar, for example `Imported 1 track, 42 notes`.
  final String summary;
}

/// Result of an export: the file bytes to write and a summary.
final class ExportOutcome {
  const ExportOutcome({required this.bytes, required this.summary});

  final List<int> bytes;
  final String summary;
}

final class MidiActions {
  const MidiActions(this._midi);

  final MidiService _midi;

  /// Import a Standard MIDI File (its [bytes]) into a new document. Throws [ApiError]
  /// for a malformed or oversized file.
  Future<ImportOutcome> import(List<int> bytes) async {
    final result = await _midi.importMidi(bytes);
    final r = result.report;
    final parts = <String>[
      'Imported ${r.tracksImported} track(s), ${r.notesImported} note(s)',
      if (r.quantizedPoints > 0) '${r.quantizedPoints} point(s) quantized',
      if (r.issues.isNotEmpty) '${r.issues.length} note(s) in the report',
    ];
    return ImportOutcome(
      document: result.document,
      summary: parts.join(', '),
    );
  }

  /// Export [document] as a Standard MIDI File. All tracks are written regardless of
  /// mute; the tempo factor is not applied (ADR-023 item 3).
  Future<ExportOutcome> export(DocumentService document) async {
    final result = await _midi.exportMidi(document);
    final r = result.report;
    final parts = <String>[
      'Exported ${r.tracks} track(s), ${r.notes} note(s)',
      if (r.roundedTicks > 0) '${r.roundedTicks} tick(s) rounded',
      if (r.tempoClamped > 0) '${r.tempoClamped} tempo(s) clamped',
    ];
    return ExportOutcome(bytes: result.bytes, summary: parts.join(', '));
  }
}
