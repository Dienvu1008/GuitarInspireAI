import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Guitar Pro import and export (ADR-036). The app overrides it with
/// `CoreBridge.guitarPro`; widget tests use `FakeGuitarProService`.
///
/// "Guitar Pro" is a trademark of Arobas Music; the app is not affiliated with it and
/// only names the format to describe compatibility (ADR-036 section 2).
final guitarProServiceProvider = Provider<GuitarProService>(
  (ref) => throw UnimplementedError('override guitarProServiceProvider'),
);

/// Most issue lines shown in the status line; the rest are counted.
const _shownIssues = 3;

String _issues(List<String> issues) {
  if (issues.isEmpty) return '';
  final shown = issues.take(_shownIssues).join('; ');
  final more = issues.length - _shownIssues;
  return more > 0 ? '; $shown; and $more more' : '; $shown';
}

/// The status line after an import: what was kept and what the core reported as
/// changed or dropped. Wording only; the counts and issue lines come from the core.
String guitarProImportSummary(GuitarProImportReportDto r) =>
    'Imported ${r.tracksImported} track(s), ${r.bars} bar(s), '
    '${r.notesImported} of ${r.notesInFile} note(s) from Guitar Pro'
    '${r.notesDropped > 0 ? ' (${r.notesDropped} dropped)' : ''}'
    '${_issues(r.issues)}';

/// The status line after an export.
String guitarProExportSummary(GuitarProExportReportDto r) =>
    'Exported ${r.tracks} track(s), ${r.bars} bar(s), ${r.notesWritten} note(s) '
    'to Guitar Pro (${r.version.toUpperCase()})'
    '${r.notesDropped > 0 ? ' (${r.notesDropped} dropped)' : ''}'
    '${_issues(r.issues)}';

/// The File menu items "Import Guitar Pro..." and "Export Guitar Pro...". The
/// composition root owns the dialogs and disk I/O (GR-11, FF-08); the core reads and
/// produces the bytes and decides the version from the chosen extension.
List<Widget> buildGuitarProMenuItems(BuildContext context, WidgetRef ref) {
  final messenger = ScaffoldMessenger.of(context);
  void snack(String text) =>
      messenger.showSnackBar(SnackBar(content: Text(text)));

  Future<void> import() async {
    // Import replaces the current document, so it asks like Open (ADR-030).
    if (!await ref.read(editorProvider.notifier).confirmReplace()) return;
    final bytes = await ref.read(fileDialogsProvider).pickGuitarProToImport();
    if (bytes == null) return;
    try {
      final result = await ref
          .read(guitarProServiceProvider)
          .importGuitarPro(bytes);
      ref.read(editorProvider.notifier).replace(result.document);
      snack(guitarProImportSummary(result.report));
    } on ApiError catch (e) {
      snack('Import failed: ${e.rule}: ${e.message}');
    }
  }

  Future<void> export() async {
    final document = ref.read(editorProvider).service;
    final dialogs = ref.read(fileDialogsProvider);
    final path = await dialogs.pickGuitarProToSave(
      suggestedName: 'Untitled.gp5',
    );
    if (path == null) return;
    final name = path.split(RegExp(r'[\\/]')).last;
    final dot = name.lastIndexOf('.');
    final extension = dot < 0 ? '' : name.substring(dot + 1);
    try {
      final result = await ref
          .read(guitarProServiceProvider)
          .exportGuitarPro(document, extension);
      await dialogs.writeFile(path, result.bytes);
      snack(guitarProExportSummary(result.report));
    } on ApiError catch (e) {
      snack('Export failed: ${e.rule}: ${e.message}');
    }
  }

  return [
    MenuItemButton(
      key: const ValueKey('import-guitar-pro'),
      onPressed: import,
      child: const Text('Import Guitar Pro...'),
    ),
    MenuItemButton(
      key: const ValueKey('export-guitar-pro'),
      onPressed: export,
      child: const Text('Export Guitar Pro...'),
    ),
  ];
}
