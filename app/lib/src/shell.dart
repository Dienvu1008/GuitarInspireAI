import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:playback_feature/playback_feature.dart';
import 'package:ui_kit/ui_kit.dart';

import 'midi_menu.dart';
import 'playback_wiring.dart';

class GuitarInspireApp extends StatelessWidget {
  const GuitarInspireApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'GuitarInspireAI',
    theme: AppTheme.light(),
    darkTheme: AppTheme.dark(),
    debugShowCheckedModeBanner: false,
    home: const Shell(),
  );
}

/// Desktop MVP layout with fixed panels (cross-platform-ui section 3): menu bar,
/// track list, TAB editor, inspector and status bar.
class Shell extends ConsumerStatefulWidget {
  const Shell({super.key});

  @override
  ConsumerState<Shell> createState() => _ShellState();
}

class _ShellState extends ConsumerState<Shell> {
  // One handler for the shell's lifetime: it holds multi-digit fret entry state and
  // runs key actions in order (Q-15).
  late final KeyboardHandler _keys = KeyboardHandler(ref);

  @override
  void dispose() {
    _keys.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final s = ref.watch(editorProvider);
    final editor = ref.read(editorProvider.notifier);
    // While playing, a score edit (revision bump) recompiles and swaps the timeline so
    // the change is heard without a restart (ADR-028 item 5). Only active when the
    // engine started.
    if (ref.watch(playbackAvailableProvider)) {
      ref.listen(editorProvider.select((e) => e.summary.revision), (_, _) {
        ref.read(transportProvider.notifier).refreshAfterEdit();
      });
    }
    final shortcuts = <ShortcutActivator, VoidCallback>{
      const SingleActivator(LogicalKeyboardKey.keyN, control: true):
          editor.newDocument,
      const SingleActivator(LogicalKeyboardKey.keyO, control: true):
          editor.open,
      const SingleActivator(LogicalKeyboardKey.keyS, control: true):
          editor.save,
      const SingleActivator(
        LogicalKeyboardKey.keyS,
        control: true,
        shift: true,
      ): editor.saveAs,
      const SingleActivator(LogicalKeyboardKey.keyZ, control: true):
          editor.undo,
      const SingleActivator(LogicalKeyboardKey.keyY, control: true):
          editor.redo,
    };
    return CallbackShortcuts(
      bindings: shortcuts,
      child: Focus(
        autofocus: true,
        onKeyEvent: (_, event) => _keys.handleKey(event),
        child: Scaffold(
          body: Column(
            children: [
              _MenuBar(shortcuts: shortcuts),
              const Divider(height: 1),
              if (ref.watch(playbackAvailableProvider)) ...[
                TransportBar(cursorMasterBar: s.cursor.bar),
                const Divider(height: 1),
              ],
              const Expanded(
                child: Row(
                  children: [
                    SizedBox(width: 220, child: TrackList()),
                    VerticalDivider(width: 1),
                    Expanded(child: TabView()),
                    VerticalDivider(width: 1),
                    SizedBox(width: 220, child: Inspector()),
                  ],
                ),
              ),
              const Divider(height: 1),
              _StatusBar(state: s),
            ],
          ),
        ),
      ),
    );
  }
}

class _MenuBar extends ConsumerWidget {
  const _MenuBar({required this.shortcuts});

  final Map<ShortcutActivator, VoidCallback> shortcuts;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = ref.watch(editorProvider);
    final editor = ref.read(editorProvider.notifier);
    MenuItemButton item(
      String label,
      SingleActivator key,
      VoidCallback? onPressed,
    ) =>
        MenuItemButton(shortcut: key, onPressed: onPressed, child: Text(label));
    return Row(
      children: [
        MenuBar(
          children: [
            SubmenuButton(
              menuChildren: [
                item(
                  'New',
                  const SingleActivator(LogicalKeyboardKey.keyN, control: true),
                  editor.newDocument,
                ),
                item(
                  'Open...',
                  const SingleActivator(LogicalKeyboardKey.keyO, control: true),
                  editor.open,
                ),
                item(
                  'Save',
                  const SingleActivator(LogicalKeyboardKey.keyS, control: true),
                  editor.save,
                ),
                item(
                  'Save As...',
                  const SingleActivator(
                    LogicalKeyboardKey.keyS,
                    control: true,
                    shift: true,
                  ),
                  editor.saveAs,
                ),
                const Divider(),
                ...buildMidiMenuItems(context, ref),
              ],
              child: const Text('File'),
            ),
            SubmenuButton(
              menuChildren: [
                item(
                  'Undo',
                  const SingleActivator(LogicalKeyboardKey.keyZ, control: true),
                  s.summary.canUndo ? editor.undo : null,
                ),
                item(
                  'Redo',
                  const SingleActivator(LogicalKeyboardKey.keyY, control: true),
                  s.summary.canRedo ? editor.redo : null,
                ),
              ],
              child: const Text('Edit'),
            ),
          ],
        ),
        const SizedBox(width: 16),
        Text(s.title, key: const ValueKey('document-title')),
      ],
    );
  }
}

class _StatusBar extends StatelessWidget {
  const _StatusBar({required this.state});

  final EditorState state;

  @override
  Widget build(BuildContext context) {
    final diagnostics = state.summary.diagnostics.length;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: Row(
        children: [
          Text('Cursor: ${state.cursor}'),
          const SizedBox(width: 24),
          Text('Revision ${state.summary.revision}'),
          const SizedBox(width: 24),
          if (diagnostics > 0)
            Text(
              '$diagnostics bar warning(s)',
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          const Spacer(),
          if (state.message != null)
            Text(
              state.message!,
              key: const ValueKey('status-message'),
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
        ],
      ),
    );
  }
}
