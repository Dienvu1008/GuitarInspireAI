import 'dart:async';

import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Offers crash recovery on startup when autosaves exist (project-format section 12).
///
/// [context] must be below a `Navigator`. Recovering opens the newest autosave as an
/// unsaved document; its recovery files stay until the next successful save, so a
/// second crash before saving loses nothing. "Discard all" removes every entry.
Future<void> checkRecovery(BuildContext context, WidgetRef ref) async {
  final factory = ref.read(documentFactoryProvider);
  final String dir;
  try {
    dir = await ref.read(appPathsProvider).recoveryDirectory();
  } catch (_) {
    return; // No app data directory available (test environment).
  }
  final entries = factory.recoveryEntries(dir);
  if (entries.isEmpty || !context.mounted) return;

  final choice = await showDialog<RecoveryChoice>(
    context: context,
    barrierDismissible: false,
    builder: (context) => RecoveryDialog(entries: entries),
  );
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
          await showDialog<void>(
            context: context,
            builder: (context) => AlertDialog(
              title: const Text('Recovery failed'),
              content: Text('${e.rule}: ${e.message}'),
              actions: [
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text('OK'),
                ),
              ],
            ),
          );
        }
      }
    case DiscardAll():
      for (final e in entries) {
        factory.discardRecovery(dir, e.projectId);
      }
  }
}

/// What the user chose in the [RecoveryDialog].
sealed class RecoveryChoice {
  const RecoveryChoice();
}

final class RecoverEntry extends RecoveryChoice {
  const RecoverEntry(this.entry);
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
              'These documents had unsaved changes when the app last closed. '
              'Choose one to recover it.',
            ),
            const SizedBox(height: 12),
            for (final e in entries)
              ListTile(
                key: ValueKey('recover-${e.projectId}'),
                dense: true,
                leading: const Icon(Icons.restore),
                title: Text(
                  e.source?.split(RegExp(r'[\\/]')).last ?? 'Untitled',
                ),
                subtitle: Text(
                  DateTime.fromMillisecondsSinceEpoch(e.timestamp.toInt())
                      .toLocal()
                      .toString(),
                ),
                onTap: () => Navigator.of(context).pop(RecoverEntry(e)),
              ),
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
