/// TAB editor: document controller, TAB view, track list, keyboard handling and
/// inspector. Imports only the `core_bridge` facade, `ui_kit` and `app_platform` (FF-05).
library;

export 'src/clipboard.dart' show BeatClipboard, beatClipboardProvider;
export 'src/document_controller.dart'
    show
        BeatSlot,
        CursorOverlayBuilder,
        DocumentController,
        EditorState,
        ReplaceGuard,
        maxShownTracksOnOpen,
        replaceGuardProvider,
        appPathsProvider,
        cursorOverlayProvider,
        documentFactoryProvider,
        editorProvider,
        fileDialogsProvider;
export 'src/editor_cursor.dart';
export 'src/inspector.dart' show Inspector;
export 'src/key_bindings.dart' show coalesceWindowMs;
export 'src/keyboard_handler.dart' show KeyboardHandler;
export 'src/tab_metrics.dart' show measureTabMetrics;
export 'src/tab_view.dart' show TabView;
export 'src/track_list.dart';
