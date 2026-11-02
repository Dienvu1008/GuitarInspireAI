/// TAB editor: document controller, TAB view, track list. Imports only the
/// `core_bridge` facade, `ui_kit` and `platform` (FF-05).
library;

export 'src/document_controller.dart'
    show
        DocumentController,
        EditorState,
        documentFactoryProvider,
        editorProvider,
        fileDialogsProvider;
export 'src/editor_cursor.dart';
export 'src/tab_metrics.dart' show measureTabMetrics;
export 'src/tab_view.dart' show TabView;
export 'src/track_list.dart';
