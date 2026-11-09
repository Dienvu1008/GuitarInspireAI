import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/misc.dart' show Override, ProviderListenable;

/// Navigator of the main window, so prompts can be shown from the exit handler and
/// from the editor's New / Open, which have no widget context of their own.
final appNavigatorKey = GlobalKey<NavigatorState>();

/// Reads a provider; both `Ref.read` and `WidgetRef.read` fit.
typedef Reader = T Function<T>(ProviderListenable<T> provider);

enum CloseChoice { save, discard, cancel }

/// The "save changes?" prompt (ADR-030 section 3), asked before the document is
/// replaced (New, Open, MIDI import) or the app exits. Returns true to go ahead.
///
/// - No unsaved work (the core decides; an untouched new document has none): go ahead.
/// - An autosave is written first, so if the system ends the session while the prompt
///   is open, the work is in the recovery folder and the session marker is still there:
///   the next start offers it (C9).
/// - Save: saves (asking for a location if needed); goes ahead only if it was saved.
///   A successful save removes the document's autosaves.
/// - Don't save: removes the document's autosaves, then goes ahead.
/// - Cancel, Esc, or no window to ask in: keeps everything as it is.
Future<bool> confirmUnsavedWork(Reader read) async {
  final state = read(editorProvider);
  if (!state.summary.unsavedWork) return true;
  final service = state.service;
  try {
    final dir = await read(appPathsProvider).recoveryDirectory();
    await service.writeAutosave(dir);
  } catch (_) {
    // Not fatal: the prompt still protects the work.
  }
  final context = appNavigatorKey.currentContext;
  if (context == null || !context.mounted) return false;
  final choice = await showDialog<CloseChoice>(
    context: context,
    builder: (context) => _SaveChangesDialog(title: state.title),
  );
  switch (choice) {
    case CloseChoice.save:
      return read(editorProvider.notifier).save();
    case CloseChoice.discard:
      service.discardAutosaves();
      return true;
    case CloseChoice.cancel || null:
      return false;
  }
}

/// Provider overrides that install the prompt for New / Open / MIDI import.
List<Override> closeOverrides() => [
  replaceGuardProvider.overrideWith(
    (ref) =>
        () => confirmUnsavedWork(ref.read),
  ),
];

class _SaveChangesDialog extends StatelessWidget {
  const _SaveChangesDialog({required this.title});

  final String title;

  @override
  Widget build(BuildContext context) {
    final name = title.startsWith('* ') ? title.substring(2) : title;
    void pick(CloseChoice c) => Navigator.of(context).pop(c);
    return AlertDialog(
      title: const Text('Save changes?'),
      content: Text('"$name" has changes that are not saved.'),
      actions: [
        TextButton(
          key: const ValueKey('close-cancel'),
          onPressed: () => pick(CloseChoice.cancel),
          child: const Text('Cancel'),
        ),
        TextButton(
          key: const ValueKey('close-discard'),
          onPressed: () => pick(CloseChoice.discard),
          child: const Text("Don't save"),
        ),
        FilledButton(
          key: const ValueKey('close-save'),
          autofocus: true,
          onPressed: () => pick(CloseChoice.save),
          child: const Text('Save'),
        ),
      ],
    );
  }
}
