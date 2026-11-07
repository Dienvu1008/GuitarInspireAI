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
typedef CursorOverlayBuilder = Widget Function(
  double? Function(int tick) tickToX,
  double height,
);

final cursorOverlayProvider = Provider<CursorOverlayBuilder?>((ref) => null);

/// Asked before the current document is replaced (New, Open, MIDI import). Returns
/// false to keep it. The app overrides it with its "save changes?" prompt (ADR-030
/// section 3); the default allows the replacement.
typedef ReplaceGuard = Future<bool> Function();

final replaceGuardProvider = Provider<ReplaceGuard>(
  (ref) =>
      () async => true,
);

/// How many tracks are shown when a document opens (ADR-031 scope reduction). On the
/// reference machine the large score stays within the 33 ms edit budget with 16
/// shown tracks and not reliably with 20; tracks beyond this start hidden and can be
/// shown from the track list. Tracks added later are shown.
const int maxShownTracksOnOpen = 16;

/// Per-user app data paths (recovery directory). Overridden in tests.
final appPathsProvider = Provider<AppPaths>((ref) => const SystemAppPaths());

/// What the editor shows. `summary` is re-read after every document event; the core
/// remains the only owner of document state (ADR-002).
/// One end of a beat selection: bar, voice and beat index (ADR-030).
typedef BeatSlot = ({int bar, int voice, int index});

final class EditorState {
  const EditorState({
    required this.service,
    required this.summary,
    required this.activeTrack,
    required this.cursor,
    this.message,
    this.anchor,
    this.hiddenTracks = const {},
  });

  final DocumentService service;
  final DocumentSummaryDto summary;

  /// Id of the track shown in the TAB view (one active TAB track at a time, Q-17).
  final int activeTrack;
  final EditorCursor cursor;

  /// Last error or status message for the status bar.
  final String? message;

  /// Where a Shift selection started. Pure UI state: not in the document, not in
  /// undo (ADR-030 section 1).
  final BeatSlot? anchor;

  /// Tracks hidden from the TAB view (ADR-031). UI state only, not saved. When a
  /// document opens, the tracks after the first [maxShownTracksOnOpen] start hidden;
  /// the user changes it from the track list. Ids of deleted tracks are ignored.
  final Set<int> hiddenTracks;

  /// Track ids shown in the TAB view, in score order. The active track is always
  /// shown (ADR-031 5.3).
  List<int> get shownTracks => [
    for (final t in summary.tracks)
      if (t.id == activeTrack || !hiddenTracks.contains(t.id)) t.id,
  ];

  /// The selected run of beats from [anchor] to the cursor, in written order and
  /// inclusive, or null when nothing is selected (no anchor, cursor on the append
  /// slot, or anchor in another voice).
  (BeatSlot, BeatSlot)? get selection {
    final a = anchor;
    final b = cursor.beat;
    if (a == null || b == null || a.voice != cursor.voice) return null;
    final c = (bar: cursor.bar, voice: cursor.voice, index: b);
    final ordered = a.bar < c.bar || (a.bar == c.bar && a.index <= c.index);
    return ordered ? (a, c) : (c, a);
  }

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
    BeatSlot? anchor,
    bool clearAnchor = false,
    Set<int>? hiddenTracks,
  }) => EditorState(
    service: service,
    summary: summary ?? this.summary,
    activeTrack: activeTrack ?? this.activeTrack,
    cursor: cursor ?? this.cursor,
    message: clearMessage ? null : (message ?? this.message),
    anchor: clearAnchor ? null : (anchor ?? this.anchor),
    hiddenTracks: hiddenTracks ?? this.hiddenTracks,
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
      // ADR-031 scope reduction: on opening, at most [maxShownTracksOnOpen] tracks
      // are shown; the rest can be shown from the track list.
      hiddenTracks: {
        for (final t in summary.tracks.skip(maxShownTracksOnOpen)) t.id,
      },
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

  /// Asks [replaceGuardProvider] whether the current document may be replaced.
  Future<bool> confirmReplace() => ref.read(replaceGuardProvider)();

  Future<void> newDocument() async {
    if (!await confirmReplace()) return;
    _replace(ref.read(documentFactoryProvider).create(bars: 4));
  }

  /// Replaces the current document (used by recovery). Exposed for the recovery dialog.
  void replace(DocumentService service) => _replace(service);

  Future<bool> open() async {
    if (!await confirmReplace()) return false;
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

  /// Makes [track] the edited track. A hidden track becomes shown, since the active
  /// track always is (ADR-031 5.3).
  void selectTrack(int track) => state = state.copyWith(
    activeTrack: track,
    cursor: EditorCursor.start,
    clearAnchor: true,
    hiddenTracks: {...state.hiddenTracks}..remove(track),
  );

  /// Shows or hides [track] in the TAB view. The active track cannot be hidden.
  void toggleTrackShown(int track) =>
      setTrackShown(track, state.hiddenTracks.contains(track));

  /// Shows ([shown] true) or hides [track] in the TAB view. Hiding the active track
  /// does nothing: it is always shown (ADR-031 5.3).
  void setTrackShown(int track, bool shown) {
    if (!shown && track == state.activeTrack) return;
    final hidden = {...state.hiddenTracks};
    if (shown) {
      hidden.remove(track);
    } else {
      hidden.add(track);
    }
    state = state.copyWith(hiddenTracks: hidden);
  }

  /// Moves the cursor. With [extend] (Shift), the selection grows from where it
  /// started, or from the old cursor if that was on a beat; otherwise it is cleared.
  void moveCursor(EditorCursor cursor, {bool extend = false}) {
    if (!extend) {
      state = state.copyWith(cursor: cursor, clearAnchor: true);
      return;
    }
    final old = state.cursor;
    final oldBeat = old.beat;
    final anchor =
        state.anchor ??
        (oldBeat == null
            ? null
            : (bar: old.bar, voice: old.voice, index: oldBeat));
    state = state.copyWith(cursor: cursor, anchor: anchor);
  }

  /// Shows a status message (for example the result of a copy).
  void showMessage(String message) => state = state.copyWith(message: message);
}
