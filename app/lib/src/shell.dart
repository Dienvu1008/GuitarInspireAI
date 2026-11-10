import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:playback_feature/playback_feature.dart';
import 'package:tools_feature/tools_feature.dart';
import 'package:ui_kit/ui_kit.dart';

import 'close_guard.dart';
import 'midi_menu.dart';
import 'musicxml_menu.dart';
import 'playback_wiring.dart';

class GuitarInspireApp extends StatelessWidget {
  const GuitarInspireApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
    navigatorKey: appNavigatorKey,
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
    // Hiding the tuner panel releases the microphone (privacy): mic is live only while
    // the panel is shown.
    ref.listen(tunerVisibleProvider, (_, visible) {
      if (!visible) ref.read(tunerProvider.notifier).disable();
    });
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
              _MenuBar(shortcuts: shortcuts, keys: _keys),
              const Divider(height: 1),
              if (ref.watch(playbackAvailableProvider)) ...[
                TransportBar(cursorMasterBar: s.cursor.bar),
                const Divider(height: 1),
              ],
              Expanded(
                child: Row(
                  children: [
                    const SizedBox(width: 220, child: TrackList()),
                    const VerticalDivider(width: 1),
                    const Expanded(child: TabView()),
                    const VerticalDivider(width: 1),
                    if (ref.watch(playbackAvailableProvider) &&
                        ref.watch(mixerVisibleProvider)) ...[
                      const SizedBox(
                        key: ValueKey('mixer-panel'),
                        width: 280,
                        child: MixerPanel(),
                      ),
                      const VerticalDivider(width: 1),
                    ],
                    if (ref.watch(tunerVisibleProvider)) ...[
                      const SizedBox(width: 240, child: TunerView()),
                      const VerticalDivider(width: 1),
                    ],
                    if (ref.watch(chordVisibleProvider)) ...[
                      const SizedBox(width: 280, child: ChordFinderView()),
                      const VerticalDivider(width: 1),
                    ],
                    const SizedBox(width: 220, child: Inspector()),
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
  const _MenuBar({required this.shortcuts, required this.keys});

  final Map<ShortcutActivator, VoidCallback> shortcuts;

  /// The editor's key handler, so menu edits run in the same order as keys.
  final KeyboardHandler keys;

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
                ...buildMusicXmlMenuItems(context, ref),
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
                const Divider(),
                // ADR-030: in-app beat clipboard (not the system clipboard).
                item(
                  'Cut',
                  const SingleActivator(LogicalKeyboardKey.keyX, control: true),
                  keys.cut,
                ),
                item(
                  'Copy',
                  const SingleActivator(LogicalKeyboardKey.keyC, control: true),
                  keys.copy,
                ),
                item(
                  'Paste',
                  const SingleActivator(LogicalKeyboardKey.keyV, control: true),
                  ref.watch(beatClipboardProvider) == null ? null : keys.paste,
                ),
              ],
              child: const Text('Edit'),
            ),
            SubmenuButton(
              menuChildren: [
                MenuItemButton(
                  key: const ValueKey('tools-mixer'),
                  trailingIcon: ref.watch(mixerVisibleProvider)
                      ? const Icon(Icons.check)
                      : null,
                  onPressed: ref.watch(playbackAvailableProvider)
                      ? () => ref.read(mixerVisibleProvider.notifier).toggle()
                      : null,
                  child: const Text('Mixer'),
                ),
                MenuItemButton(
                  key: const ValueKey('tools-tuner'),
                  trailingIcon: ref.watch(tunerVisibleProvider)
                      ? const Icon(Icons.check)
                      : null,
                  onPressed: () =>
                      ref.read(tunerVisibleProvider.notifier).toggle(),
                  child: const Text('Tuner'),
                ),
                MenuItemButton(
                  key: const ValueKey('tools-chord'),
                  trailingIcon: ref.watch(chordVisibleProvider)
                      ? const Icon(Icons.check)
                      : null,
                  onPressed: () =>
                      ref.read(chordVisibleProvider.notifier).toggle(),
                  child: const Text('Chord finder'),
                ),
                const Divider(),
                // ADR-034: apply a pattern of the session library (6a-3: the sample).
                MenuItemButton(
                  key: const ValueKey('tools-apply-pattern'),
                  onPressed: () => showApplyPatternDialog(context),
                  child: const Text('Apply pattern...'),
                ),
              ],
              child: const Text('Tools'),
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
