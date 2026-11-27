import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// PDF export (ADR-041 section 12). The app overrides it with `CoreBridge.pdf`; widget
/// tests use `FakePdfService`.
final pdfServiceProvider = Provider<PdfService>(
  (ref) => throw UnimplementedError('override pdfServiceProvider'),
);

/// The PDF title: the document's file name without its extension, or "Untitled"
/// (ADR-041 section 11.1, open point 2). Wording only.
String pdfTitle(String? location) {
  if (location == null) return 'Untitled';
  final name = location.split(RegExp(r'[\\/]')).last;
  final dot = name.lastIndexOf('.');
  return dot > 0 ? name.substring(0, dot) : name;
}

/// The options the person exporting chooses (section 12, choices 1 and 2).
typedef PdfOptions = ({PaperDto paper, SystemSpacingDto spacing});

/// Asks for the paper size and the spacing between systems; `null` if cancelled.
Future<PdfOptions?> askPdfOptions(BuildContext context) {
  var paper = PaperDto.a4;
  var spacing = SystemSpacingDto.spacious;
  return showDialog<PdfOptions>(
    context: context,
    builder: (context) => StatefulBuilder(
      builder: (context, setState) => AlertDialog(
        title: const Text('Export PDF'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('Paper'),
            const SizedBox(height: 4),
            SegmentedButton<PaperDto>(
              key: const ValueKey('pdf-paper'),
              segments: const [
                ButtonSegment(value: PaperDto.a4, label: Text('A4')),
                ButtonSegment(value: PaperDto.letter, label: Text('Letter')),
              ],
              selected: {paper},
              onSelectionChanged: (s) => setState(() => paper = s.single),
            ),
            const SizedBox(height: 12),
            const Text('Space between systems'),
            const SizedBox(height: 4),
            SegmentedButton<SystemSpacingDto>(
              key: const ValueKey('pdf-spacing'),
              segments: const [
                ButtonSegment(
                  value: SystemSpacingDto.spacious,
                  label: Text('Spacious'),
                ),
                ButtonSegment(
                  value: SystemSpacingDto.compact,
                  label: Text('Compact'),
                ),
              ],
              selected: {spacing},
              onSelectionChanged: (s) => setState(() => spacing = s.single),
            ),
            const SizedBox(height: 12),
            const Text('The shown tracks are exported as they are shown.'),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const ValueKey('pdf-export-confirm'),
            onPressed: () =>
                Navigator.of(context).pop((paper: paper, spacing: spacing)),
            child: const Text('Export'),
          ),
        ],
      ),
    ),
  );
}

/// The File menu item "Export PDF...". The composition root owns the dialogs and the
/// disk write (GR-11, FF-08); the core produces the bytes from what the view shows.
List<Widget> buildPdfMenuItems(BuildContext context, WidgetRef ref) {
  final messenger = ScaffoldMessenger.of(context);

  Future<void> export() async {
    final options = await askPdfOptions(context);
    if (options == null) return;
    final state = ref.read(editorProvider);
    final title = pdfTitle(state.summary.location);
    try {
      final result = await ref
          .read(pdfServiceProvider)
          .exportPdf(
            state.service,
            shown: state.shownStaves,
            paper: options.paper,
            spacing: options.spacing,
            title: title,
          );
      final path = await ref
          .read(fileDialogsProvider)
          .savePdf(result.bytes, suggestedName: '$title.pdf');
      if (path != null) {
        messenger.showSnackBar(
          SnackBar(content: Text('Exported ${result.pages} page(s) to PDF')),
        );
      }
    } on ApiError catch (e) {
      messenger.showSnackBar(
        SnackBar(content: Text('Export failed: ${e.rule}: ${e.message}')),
      );
    }
  }

  return [
    MenuItemButton(
      key: const ValueKey('export-pdf'),
      onPressed: export,
      child: const Text('Export PDF...'),
    ),
  ];
}
