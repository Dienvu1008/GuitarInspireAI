import 'dart:async';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart' show ExternalLibrary;

import 'document_service.dart';
import 'generated/api/document.dart';
import 'generated/api/layout.dart';
import 'generated/api/types.dart';
import 'generated/frb_generated.dart';

/// Loads the native Music Core. Call once before creating documents.
abstract final class CoreBridge {
  static bool _initialized = false;

  /// [libraryPath] overrides the default library lookup (used by tests that load
  /// `target/debug/mc_ffi.dll` directly).
  static Future<void> init({String? libraryPath}) async {
    if (_initialized) return;
    await RustLib.init(
      externalLibrary: libraryPath == null ? null : ExternalLibrary.open(libraryPath),
    );
    _initialized = true;
  }

  static DocumentFactory get documents => const _RustDocumentFactory();
}

final class _RustDocumentFactory implements DocumentFactory {
  const _RustDocumentFactory();

  @override
  DocumentService create({int bars = 4}) =>
      RustDocumentService._(DocumentHandle.create(bars: bars));

  @override
  Future<DocumentService> open(String path) async =>
      RustDocumentService._(await DocumentHandle.open(path: path));
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
  Future<OutcomeDto> execute(CommandDto command, {int? coalesceKey, int? expectedRevision}) =>
      _handle.execute(
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
  int voiceCount(int track, int bar) => _handle.voiceCount(track: track, bar: bar);

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
  void close() {
    _handle.unsubscribeEvents();
    unawaited(_source?.cancel());
    unawaited(_events?.close());
    _handle.dispose();
  }
}
