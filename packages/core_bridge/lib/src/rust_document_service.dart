import 'dart:async';

// The bridge's own Int64List (it differs from dart:typed_data's on the web).
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary, Int64List;

import 'document_service.dart';
import 'generated/api/document.dart';
import 'generated/api/layout.dart';
import 'generated/api/pattern.dart' as pattern_api;
import 'generated/api/pattern_grid.dart' as grid_api;
import 'generated/api/types.dart';
import 'generated/frb_generated.dart';
import 'musicxml_service.dart' show MusicXmlService, RustMusicXmlService;
import 'playback_service.dart' show MidiService, PlaybackFactory;
import 'rust_playback_service.dart' show RustMidiService, RustPlaybackFactory;
import 'rust_tuner_service.dart' show RustTunerFactory;
import 'tuner_service.dart' show TunerFactory;

/// Loads the native Music Core. Call once before creating documents.
abstract final class CoreBridge {
  static bool _initialized = false;

  /// [libraryPath] overrides the default library lookup (used by tests that load
  /// `target/debug/mc_ffi.dll` directly).
  static Future<void> init({String? libraryPath}) async {
    if (_initialized) return;
    await RustLib.init(
      externalLibrary: libraryPath == null
          ? null
          : ExternalLibrary.open(libraryPath),
    );
    _initialized = true;
  }

  static DocumentFactory get documents => const _RustDocumentFactory();

  /// Starts the shared audio engine and returns the playback factory (ADR-028). Call
  /// once after [init]; [soundfontPath] is the bundled SoundFont (4c). Throws [ApiError]
  /// with rule `audio` if there is no output device at all.
  static Future<PlaybackFactory> startPlayback({String? soundfontPath}) =>
      RustPlaybackFactory.start(soundfontPath: soundfontPath);

  /// MIDI import and export (no audio device needed, ADR-023).
  static MidiService get midi => const RustMidiService();

  /// MusicXML export (ADR-033); no audio device needed.
  static MusicXmlService get musicXml => const RustMusicXmlService();

  /// A tuner factory bound to [document] (ADR-029). Call `start()` on it to open the
  /// microphone; independent of the playback engine.
  static TunerFactory tunerFor(DocumentService document) =>
      RustTunerFactory.forDocument(document);
}

final class _RustDocumentFactory implements DocumentFactory {
  const _RustDocumentFactory();

  @override
  DocumentService create({int bars = 4}) =>
      RustDocumentService._(DocumentHandle.create(bars: bars));

  @override
  Future<DocumentService> open(String path) async =>
      RustDocumentService._(await DocumentHandle.open(path: path));

  @override
  List<RecoveryEntryDto> recoveryEntries(String dir) =>
      DocumentHandle.listRecoveryEntries(dir: dir);

  @override
  Future<DocumentService> openFromRecovery(
    String dir,
    String projectId,
  ) async => RustDocumentService._(
    await DocumentHandle.openFromRecovery(dir: dir, projectId: projectId),
  );

  @override
  void discardRecovery(String dir, String projectId) =>
      DocumentHandle.discardRecovery(dir: dir, projectId: projectId);

  @override
  bool beginSession(String dir) => DocumentHandle.beginSession(dir: dir);

  @override
  void endSession(String dir) => DocumentHandle.endSession(dir: dir);

  @override
  List<RecoveryEntryDto> offeredRecovery(String dir, {required bool unclean}) =>
      DocumentHandle.offeredRecoveryEntries(dir: dir, unclean: unclean);
}

/// [DocumentService] backed by the Rust session in `mc-ffi`.
final class RustDocumentService implements DocumentService {
  RustDocumentService._(this._handle);

  final DocumentHandle _handle;
  StreamController<EventDto>? _events;
  StreamSubscription<EventDto>? _source;

  @override
  DocumentSummaryDto summary() => _handle.summary();

  @override
  Future<OutcomeDto> execute(
    CommandDto command, {
    int? coalesceKey,
    int? expectedRevision,
  }) => _handle.execute(
    command: command,
    coalesceKey: coalesceKey,
    expectedRevision: expectedRevision,
  );

  @override
  Future<OutcomeDto> undo() => _handle.undo();

  @override
  Future<OutcomeDto> redo() => _handle.redo();

  @override
  Future<OutcomeDto> save() => _handle.save();

  @override
  Future<OutcomeDto> saveAs(String path) => _handle.saveAs(path: path);

  @override
  List<BeatDto> beats(int track, int bar, int voice) =>
      _handle.beats(track: track, bar: bar, voice: voice);

  @override
  int voiceCount(int track, int bar) =>
      _handle.voiceCount(track: track, bar: bar);

  @override
  List<PlayedBarDto> playedBars() => _handle.playedBars();

  @override
  ChordResultDto chordVoicings(int track, String symbol, {int max = 5}) =>
      _handle.chordVoicings(track: track, symbol: symbol, max: max);

  @override
  List<pattern_api.PatternInfoDto> patterns() =>
      pattern_api.patterns(document: _handle);

  @override
  int samplePattern() => pattern_api.samplePattern(document: _handle);

  @override
  pattern_api.PatternPreviewDto previewPattern(
    pattern_api.ApplyPatternDto args,
  ) => pattern_api.previewPattern(document: _handle, args: args);

  @override
  grid_api.GridViewDto patternGrid(int pattern, List<int> snaps) =>
      grid_api.patternGrid(document: _handle, pattern: pattern, snaps: snaps);

  @override
  List<grid_api.GestureInfoDto> gestures() =>
      grid_api.gestures(document: _handle);

  @override
  List<ClipBeatDto> copyBeats(
    int track,
    int voice, {
    required int fromBar,
    required int fromIndex,
    required int toBar,
    required int toIndex,
  }) => _handle.copyBeats(
    track: track,
    voice: voice,
    fromBar: fromBar,
    fromIndex: fromIndex,
    toBar: toBar,
    toIndex: toIndex,
  );

  @override
  ScoreOutlineDto scoreOutline(List<int> tracks, MetricsDto metrics) => _handle
      .scoreOutline(tracks: Int64List.fromList(tracks), metrics: metrics);

  @override
  List<ScoreSystemDto> scoreSystems(
    List<int> tracks,
    MetricsDto metrics,
    int first,
    int count,
  ) => _handle.scoreSystems(
    tracks: Int64List.fromList(tracks),
    metrics: metrics,
    first: first,
    count: count,
  );

  @override
  TabLayoutDto tabLayout(int track, MetricsDto metrics) =>
      _handle.tabLayout(track: track, metrics: metrics);

  /// One Rust subscription shared by all Dart listeners. Rust owns the stream
  /// lifecycle (ADR-005 amendment 1): [close] unsubscribes on the Rust side instead of
  /// waiting for `cancel()`.
  @override
  Stream<EventDto> get events {
    final existing = _events;
    if (existing != null) return existing.stream;
    final controller = StreamController<EventDto>.broadcast();
    _events = controller;
    _source = _handle.subscribeEvents().listen(controller.add);
    return controller.stream;
  }

  @override
  bool autosaveDue() => _handle.autosaveDue();

  @override
  Future<void> writeAutosave(String dir) => _handle.writeAutosave(dir: dir);

  @override
  void discardAutosaves() => _handle.discardAutosaves();

  @override
  void close() {
    _handle.unsubscribeEvents();
    unawaited(_source?.cancel());
    unawaited(_events?.close());
    _handle.dispose();
  }
}

/// The native handle behind a [RustDocumentService], for the playback service in this
/// library (ADR-028: the engine drives transport against the document handle). Throws if
/// the service is not Rust-backed (for example a fake), which is a wiring bug.
DocumentHandle rustDocumentHandle(DocumentService service) {
  if (service is! RustDocumentService) {
    throw StateError(
      'the Rust engine needs a RustDocumentService, got ${service.runtimeType}',
    );
  }
  return service._handle;
}

/// Wraps a native handle (for example from MIDI import) as a [DocumentService].
DocumentService rustServiceFromHandle(DocumentHandle handle) =>
    RustDocumentService._(handle);
