import 'dart:async';

import 'document_service.dart';
import 'generated/api/document.dart' show DocumentHandle;
import 'generated/api/engine.dart';
import 'generated/api/midi.dart' as midi;
import 'generated/api/types.dart' show PositionDto;
import 'playback_service.dart';
import 'rust_document_service.dart'
    show rustDocumentHandle, rustServiceFromHandle;

/// Owns the one shared [EngineHandle] and hands out [RustPlaybackService]s bound to a
/// document. The engine opens the default output device and loads the bundled SoundFont
/// lazily on the first Play (Q10, ADR-026). Created by [RustPlaybackFactory.start].
final class RustPlaybackFactory implements PlaybackFactory {
  RustPlaybackFactory._(this._engine);

  final EngineHandle _engine;

  /// Starts the engine. [soundfontPath] is the bundled `GuitarSteel.sf2`; `null` leaves
  /// playback disabled until a path is given (a missing file is reported per command).
  static Future<RustPlaybackFactory> start({String? soundfontPath}) async {
    final engine = await EngineHandle.start(soundfontPath: soundfontPath);
    return RustPlaybackFactory._(engine);
  }

  @override
  PlaybackService forDocument(DocumentService document) =>
      RustPlaybackService._(_engine, rustDocumentHandle(document));
}

/// MIDI import and export over `mc-midi`, with no audio device (ADR-023).
final class RustMidiService implements MidiService {
  const RustMidiService();

  @override
  Future<MidiImport> importMidi(List<int> bytes) async {
    final result = await midi.importMidi(bytes: bytes);
    // Wrap the imported native handle as a Rust document service (same package).
    final service = rustServiceFromHandle(result.document);
    return MidiImport(document: service, report: result.report);
  }

  @override
  Future<midi.ExportResult> exportMidi(DocumentService document) =>
      midi.exportMidi(document: rustDocumentHandle(document));
}

/// [PlaybackService] backed by the shared Rust engine, bound to one document handle.
final class RustPlaybackService implements PlaybackService {
  RustPlaybackService._(this._engine, this._document);

  final EngineHandle _engine;
  final DocumentHandle _document;
  StreamController<PositionDto>? _positions;
  StreamSubscription<PositionDto>? _source;

  @override
  Future<void> play({int? fromMasterBar}) async {
    await _engine.play(document: _document, fromMasterBar: fromMasterBar);
  }

  @override
  Future<void> stop() async {
    await _engine.stop(document: _document);
  }

  @override
  Future<void> seek(int tick) async {
    await _engine.seek(document: _document, tick: tick);
  }

  @override
  Future<void> setMuted(int track, bool muted) async {
    await _engine.setMuted(document: _document, track: track, muted: muted);
  }

  @override
  Future<void> setTempoFactor(int hundredths) async {
    await _engine.setTempoFactor(document: _document, hundredths: hundredths);
  }

  @override
  Future<void> refreshAfterEdit() async {
    await _engine.refreshAfterEdit(document: _document);
  }

  @override
  PositionDto position() => _engine.position();

  @override
  String? get lastError => _engine.lastError();

  /// One Rust subscription shared by all Dart listeners. [close] unsubscribes on the
  /// Rust side (ADR-005 amendment 1).
  @override
  Stream<PositionDto> get positions {
    final existing = _positions;
    if (existing != null) return existing.stream;
    final controller = StreamController<PositionDto>.broadcast();
    _positions = controller;
    _source = _engine.positions().listen(controller.add);
    return controller.stream;
  }

  @override
  void close() {
    unawaited(_source?.cancel());
    unawaited(_positions?.close());
    // The engine is shared and owned by the factory; do not dispose it here.
  }
}
