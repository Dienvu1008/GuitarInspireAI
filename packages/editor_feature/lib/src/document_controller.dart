import 'dart:async';

import 'package:app_platform/app_platform.dart';
import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'editor_cursor.dart';

/// Creates and opens documents. The app overrides it with the Rust-backed factory;
/// widget tests use `FakeDocumentFactory`.
final documentFactoryProvider = Provider<DocumentFactory>(
  (ref) => throw UnimplementedError('override documentFactoryProvider'),
);

final fileDialogsProvider = Provider<FileDialogs>(
  (ref) => const SystemFileDialogs(),
);

/// Builds the playback cursor overlay for a TAB system, given a tick-to-x mapper and the
/// system height. The app overrides it with a `PlaybackCursor` from `playback_feature`;
/// by default there is no cursor (editor_feature does not depend on playback, FF-05).
typedef CursorOverlayBuilder =
    Widget Function(double? Function(int tick) tickToX, double height);

final cursorOverlayProvider = Provider<CursorOverlayBuilder?>((ref) => null);

/// Per-user app data paths (recovery directory). Overridden in tests.
final appPathsProvider = Provider<AppPaths>((ref) => const SystemAppPaths());

/// What the editor shows. `summary` is re-read after every document event; the core
/// remains the only owner of document state (ADR-002).
final class EditorState {
  const EditorState({
    required this.service,
    required this.summary,
    required this.activeTrack,
    required this.cursor,
    this.message,
  });

  final DocumentService service;
  final DocumentSummaryDto summary;

  /// Id of the track shown in the TAB view (one active TAB track at a time, Q-17).
  final int activeTrack;
  final EditorCursor cursor;

  /// Last error or status message for the status bar.
  final String? message;

  String get title {
    final location = summary.location;
    final name = location == null
        ? 'Untitled'
        : location.split(RegExp(r'[\\/]')).last;
    return '${summary.dirty ? '* ' : ''}$name';
  }

  EditorState copyWith({
    DocumentSummaryDto? summary,
    int? activeTrack,
    EditorCursor? cursor,
    String? message,
    bool clearMessage = false,
  }) => EditorState(
    service: service,
    summary: summary ?? this.summary,
    activeTrack: activeTrack ?? this.activeTrack,
    cursor: cursor ?? this.cursor,
    message: clearMessage ? null : (message ?? this.message),
  );
}

final editorProvider = NotifierProvider<DocumentController, EditorState>(
  DocumentController.new,
);

/// Document lifecycle and commands for the editor. No musical rules here: commands go
/// to the core, which validates them (GR-05).
final class DocumentController extends Notifier<EditorState> {
  StreamSubscription<EventDto>? _events;

  @override
  EditorState build() {
    ref.onDispose(() => _detach());
    return _attach(ref.read(documentFactoryProvider).create(bars: 4));
  }

  bool _refreshQueued = false;

  /// One command emits several events; they are folded into one refresh. The summary
  /// query is the expensive part on large scores, so it runs once per batch.
  void _queueRefresh() {
    if (_refreshQueued) return;
    _refreshQueued = true;
    scheduleMicrotask(_flushRefresh);
  }

  void _flushRefresh() {
    if (!_refreshQueued) return;
    _refreshQueued = false;
    if (_events != null) _refresh();
  }

  EditorState _attach(DocumentService service) {
    _events = service.events.listen((_) => _queueRefresh());
    final summary = service.summary();
    return EditorState(
      service: service,
      summary: summary,
      activeTrack: summary.tracks.first.id,
      cursor: EditorCursor.start,
    );
  }

  void _detach() {
    unawaited(_events?.cancel());
    _events = null;
  }

  void _replace(DocumentService service) {
    final old = state.service;
    _detach();
    state = _attach(service);
    old.close();
  }

  void _refresh() {
    final summary = state.service.summary();
    final old = state.summary;
    // Summary DTOs hold lists, so == is identity; compare what can change instead.
    // Every content change bumps the revision; the rest is save and undo state.
    if (summary.revision == old.revision &&
        summary.dirty == old.dirty &&
        summary.canUndo == old.canUndo &&
        summary.canRedo == old.canRedo &&
        summary.location == old.location) {
      return;
    }
    final stillThere = summary.tracks.any((t) => t.id == state.activeTrack);
    state = state.copyWith(
      summary: summary,
      activeTrack: stillThere ? state.activeTrack : summary.tracks.first.id,
    );
  }

  /// Runs an action and turns a rejection into a status message instead of a crash.
  Future<bool> _guard(Future<void> Function() action) async {
    try {
      await action();
      // Every change arrives as events; apply a refresh they queued now, so callers
      // see the new state when this returns, without a second summary query.
      _flushRefresh();
      state = state.copyWith(clearMessage: true);
      return true;
    } on ApiError catch (e) {
      state = state.copyWith(message: '${e.rule}: ${e.message}');
      return false;
    }
  }

  void newDocument() =>
      _replace(ref.read(documentFactoryProvider).create(bars: 4));

  /// Replaces the current document (used by recovery). Exposed for the recovery dialog.
  void replace(DocumentService service) => _replace(service);

  Future<bool> open() async {
    final path = await ref.read(fileDialogsProvider).pickProjectToOpen();
    if (path == null) return false;
    return openPath(path);
  }

  Future<bool> openPath(String path) => _guard(() async {
    _replace(await ref.read(documentFactoryProvider).open(path));
  });

  /// Saves; asks for a location when the document has none yet.
  Future<bool> save() async {
    if (state.summary.location == null) return saveAs();
    return _guard(() => state.service.save());
  }

  Future<bool> saveAs() async {
    final path = await ref
        .read(fileDialogsProvider)
        .pickProjectToSave(suggestedName: 'Untitled.$projectExtension');
    if (path == null) return false;
    return _guard(() => state.service.saveAs(path));
  }

  Future<bool> undo() => _guard(() => state.service.undo());

  Future<bool> redo() => _guard(() => state.service.redo());

  Future<bool> execute(CommandDto command, {int? coalesceKey}) =>
      _guard(() => state.service.execute(command, coalesceKey: coalesceKey));

  void selectTrack(int track) =>
      state = state.copyWith(activeTrack: track, cursor: EditorCursor.start);

  void moveCursor(EditorCursor cursor) =>
      state = state.copyWith(cursor: cursor);
}
