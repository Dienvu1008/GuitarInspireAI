import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// MusicXML export (ADR-033). The app overrides it with `CoreBridge.musicXml`; widget
/// tests use `FakeMusicXmlService`.
final musicXmlServiceProvider = Provider<MusicXmlService>(
  (ref) => throw UnimplementedError('override musicXmlServiceProvider'),
);

/// The status line after an export: what was written and every approximation the
/// core reported, so the user knows no note was lost (ADR-033 choice 5.4). Wording
/// only; the counts come from the core.
String musicXmlSummary(MusicXmlReportDto r) => <String>[
  'Exported ${r.parts} track(s), ${r.measures} bar(s), ${r.notes} note(s) to MusicXML',
  if (r.overfullMeasures > 0)
    '${r.overfullMeasures} overfull bar(s) kept with all notes',
  if (r.paddedVoices > 0) '${r.paddedVoices} short voice(s) filled with rests',
  if (r.tupletsWithoutBracket > 0)
    '${r.tupletsWithoutBracket} tuplet note(s) without a bracket',
  if (r.tempoRounded > 0) '${r.tempoRounded} tempo(s) rounded',
  if (r.charactersRemoved > 0)
    '${r.charactersRemoved} character(s) removed from names',
].join(', ');

/// The File menu item "Export MusicXML...". The composition root owns the dialog and
/// the disk write (GR-11, FF-08); the core produces the bytes.
List<Widget> buildMusicXmlMenuItems(BuildContext context, WidgetRef ref) {
  final messenger = ScaffoldMessenger.of(context);

  Future<void> export() async {
    final document = ref.read(editorProvider).service;
    try {
      final result = await ref
          .read(musicXmlServiceProvider)
          .exportMusicXml(document);
      final path = await ref
          .read(fileDialogsProvider)
          .saveMusicXml(result.bytes, suggestedName: 'Untitled.musicxml');
      if (path != null) {
        messenger.showSnackBar(
          SnackBar(content: Text(musicXmlSummary(result.report))),
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
      key: const ValueKey('export-musicxml'),
      onPressed: export,
      child: const Text('Export MusicXML...'),
    ),
  ];
}
