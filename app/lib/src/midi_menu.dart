import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:playback_feature/playback_feature.dart';

/// Import and Export submenu items for the File menu (ADR-023). The composition root
/// owns the file dialogs and disk I/O (GR-11, FF-08); the feature translates bytes
/// through the core. Import opens the result as the active document.
List<Widget> buildMidiMenuItems(BuildContext context, WidgetRef ref) {
  final messenger = ScaffoldMessenger.of(context);

  Future<void> importMidi() async {
    final dialogs = ref.read(fileDialogsProvider);
    final bytes = await dialogs.pickMidiToImport();
    if (bytes == null) return;
    final actions = ref.read(midiActionsProvider);
    try {
      final outcome = await actions.import(bytes);
      ref.read(editorProvider.notifier).replace(outcome.document);
      _snack(messenger, outcome.summary);
    } on ApiError catch (e) {
      _snack(messenger, 'Import failed: ${e.rule}: ${e.message}');
    }
  }

  Future<void> exportMidi() async {
    final service = ref.read(editorProvider).service;
    final actions = ref.read(midiActionsProvider);
    try {
      final outcome = await actions.export(service);
      final dialogs = ref.read(fileDialogsProvider);
      final path = await dialogs.saveMidi(
        outcome.bytes,
        suggestedName: 'Untitled.mid',
      );
      if (path != null) _snack(messenger, outcome.summary);
    } on ApiError catch (e) {
      _snack(messenger, 'Export failed: ${e.rule}: ${e.message}');
    }
  }

  return [
    MenuItemButton(
      onPressed: importMidi,
      child: const Text('Import MIDI...'),
    ),
    MenuItemButton(
      onPressed: exportMidi,
      child: const Text('Export MIDI...'),
    ),
  ];
}

void _snack(ScaffoldMessengerState messenger, String message) {
  messenger.showSnackBar(SnackBar(content: Text(message)));
}
