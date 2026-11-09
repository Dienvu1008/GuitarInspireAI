import 'dart:async';

import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Starts the app session and offers crash recovery (project-format section 12,
/// ADR-030 section 2).
///
/// The session marker tells whether the last session ended cleanly; which autosaves
/// to offer is decided by the core (only after an unclean session, only those newer
/// than their source). [context] must be below a `Navigator`.
///
/// - Restore opens the autosave as an unsaved document; its files stay until the
///   next successful save, so a second crash before saving loses nothing.
/// - Open original opens the saved file and keeps the autosave, so nothing is lost.
/// - Discard removes that entry and shows the rest; Discard all removes every entry.
Future<void> checkRecovery(BuildContext context, WidgetRef ref) async {
  final factory = ref.read(documentFactoryProvider);
  final String dir;
  try {
    dir = await ref.read(appPathsProvider).recoveryDirectory();
  } catch (_) {
    return; // No app data directory available (test environment).
  }
  bool unclean;
  try {
    unclean = factory.beginSession(dir);
  } on ApiError {
    // The marker could not be written or read. Offering recovery is the safe side.
    unclean = true;
  }
  var entries = factory.offeredRecovery(dir, unclean: unclean);
  while (entries.isNotEmpty) {
    if (!context.mounted) return;
    final choice = await showDialog<RecoveryChoice>(
      context: context,
      barrierDismissible: false,
      builder: (context) => RecoveryDialog(entries: entries),
    );
    if (!context.mounted) return;
    switch (choice) {
      case null:
        return;
      case RecoverEntry(:final entry):
        try {
          final service = await factory.openFromRecovery(dir, entry.projectId);
          ref.read(editorProvider.notifier).replace(service);
        } on ApiError catch (e) {
          // Keep the files: the user can retry on the next start.
          if (context.mounted) {
            await _showFailure(
              context,
              'Recovery failed',
              '${e.rule}: ${e.message}',
            );
          }
        }
        return;
      case OpenOriginal(:final entry):
        final source = entry.source;
        if (source == null) return;
        // The autosave is kept on purpose: the user has not decided to drop it.
        final ok = await ref.read(editorProvider.notifier).openPath(source);
        if (!ok && context.mounted) {
          await _showFailure(
            context,
            'Could not open the original',
            ref.read(editorProvider).message ?? source,
          );
        }
        return;
      case DiscardEntry(:final entry):
        factory.discardRecovery(dir, entry.projectId);
        entries = factory.offeredRecovery(dir, unclean: unclean);
      case DiscardAll():
        for (final e in entries) {
          factory.discardRecovery(dir, e.projectId);
        }
        return;
    }
  }
}

Future<void> _showFailure(BuildContext context, String title, String message) =>
    showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(title),
        content: Text(message),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('OK'),
          ),
        ],
      ),
    );

/// What the user chose in the [RecoveryDialog].
sealed class RecoveryChoice {
  const RecoveryChoice();
}

final class RecoverEntry extends RecoveryChoice {
  const RecoverEntry(this.entry);
  final RecoveryEntryDto entry;
}

final class OpenOriginal extends RecoveryChoice {
  const OpenOriginal(this.entry);
  final RecoveryEntryDto entry;
}

final class DiscardEntry extends RecoveryChoice {
  const DiscardEntry(this.entry);
  final RecoveryEntryDto entry;
}

final class DiscardAll extends RecoveryChoice {
  const DiscardAll();
}

/// Writes an autosave snapshot whenever the core says one is due (dirty and the
/// autosave interval has passed). The timer only polls; the policy is in the core.
class AutosaveTimer {
  AutosaveTimer(this._ref, {this.pollInterval = const Duration(seconds: 30)});

  final WidgetRef _ref;
  final Duration pollInterval;
  Timer? _timer;

  void start() {
    _timer?.cancel();
    _timer = Timer.periodic(pollInterval, (_) => unawaited(tick()));
  }

  void stop() {
    _timer?.cancel();
    _timer = null;
  }

  /// One poll; public for tests.
  Future<void> tick() async {
    final state = _ref.read(editorProvider);
    if (!state.service.autosaveDue()) return;
    try {
      final dir = await _ref.read(appPathsProvider).recoveryDirectory();
      await state.service.writeAutosave(dir);
    } catch (_) {
      // Autosave failure is non-fatal; the next poll retries.
    }
  }
}

class RecoveryDialog extends StatelessWidget {
  const RecoveryDialog({super.key, required this.entries});

  final List<RecoveryEntryDto> entries;

  Widget _entry(BuildContext context, RecoveryEntryDto e) {
    final name = e.source?.split(RegExp(r'[\\/]')).last ?? 'Untitled';
    final when = DateTime.fromMillisecondsSinceEpoch(e.timestamp.toInt())
        .toLocal()
        .toString();
    void pick(RecoveryChoice c) => Navigator.of(context).pop(c);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          ListTile(
            dense: true,
            contentPadding: EdgeInsets.zero,
            leading: const Icon(Icons.restore),
            title: Text(name),
            subtitle: Text('Autosaved $when'),
          ),
          Wrap(
            spacing: 8,
            children: [
              FilledButton(
                key: ValueKey('recover-${e.projectId}'),
                onPressed: () => pick(RecoverEntry(e)),
                child: const Text('Restore'),
              ),
              // Disabled for an untitled document: there is no original to open.
              OutlinedButton(
                key: ValueKey('original-${e.projectId}'),
                onPressed: e.source == null
                    ? null
                    : () => pick(OpenOriginal(e)),
                child: const Text('Open original'),
              ),
              TextButton(
                key: ValueKey('discard-${e.projectId}'),
                onPressed: () => pick(DiscardEntry(e)),
                child: const Text('Discard'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Recover documents'),
      content: SizedBox(
        width: 400,
        child: ListView(
          shrinkWrap: true,
          children: [
            const Text(
              'The app did not close normally last time. These documents have '
              'changes newer than their saved file.',
            ),
            const SizedBox(height: 12),
            for (final e in entries) _entry(context, e),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(const DiscardAll()),
          child: const Text('Discard all'),
        ),
      ],
    );
  }
}
