import 'dart:async';

import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Checks for recovery files on startup and shows a dialog if any exist.
Future<void> checkRecovery(BuildContext context, WidgetRef ref) async {
  final factory = ref.read(documentFactoryProvider);
  final paths = ref.read(appPathsProvider);
  late String dir;
  try {
    dir = await paths.recoveryDirectory();
  } catch (_) {
    return; // No app data directory available (test environment).
  }
  final entries = factory.recoveryEntries(dir);
  if (entries.isEmpty || !context.mounted) return;

  final choice = await showDialog<RecoveryEntryDto>(
    context: context,
    builder: (context) => RecoveryDialog(entries: entries),
  );
  if (choice == null) return;
  if (!context.mounted) return;

  // 'recover' opens the file; 'discard' removes it.
  final isRecover = choice.projectId != '__discard__';
  if (isRecover) {
    try {
      final service = await factory.openFromRecovery(choice.path);
      ref.read(editorProvider.notifier).replace(service);
    } catch (_) {
      // Recovery failed; discard the file.
    }
  }
  factory.discardRecovery(choice.path);
}

/// Periodically writes an autosave snapshot when the document is dirty.
class AutosaveTimer {
  AutosaveTimer(this._ref);

  final WidgetRef _ref;
  Timer? _timer;

  void start() {
    _timer?.cancel();
    _timer = Timer.periodic(const Duration(seconds: 30), (_) => _tick());
  }

  void stop() {
    _timer?.cancel();
    _timer = null;
  }

  void _tick() async {
    final state = _ref.read(editorProvider);
    if (!state.service.autosaveDue()) return;
    final paths = _ref.read(appPathsProvider);
    try {
      final dir = await paths.recoveryDirectory();
      await state.service.writeAutosave(dir);
    } catch (_) {
      // Autosave failure is non-fatal; the next tick will retry.
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
              'The following documents were not saved before the app '
              'closed. Recover them now?',
            ),
            const SizedBox(height: 12),
            for (final e in entries)
              ListTile(
                dense: true,
                leading: const Icon(Icons.restore),
                title: Text(e.path.split(RegExp(r'[\\/]')).last),
                subtitle: Text(
                  DateTime.fromMillisecondsSinceEpoch(e.timestamp.abs())
                      .toLocal()
                      .toString(),
                ),
                onTap: () => Navigator.of(context).pop(e),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(
            const RecoveryEntryDto(
              projectId: '__discard__',
              path: '',
              timestamp: 0,
            ),
          ),
          child: const Text('Discard all'),
        ),
      ],
    );
  }
}
