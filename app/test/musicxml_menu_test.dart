import 'dart:convert';

import 'package:app_platform/app_platform.dart';
import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:guitar_inspire/src/musicxml_menu.dart';
import 'package:guitar_inspire/src/shell.dart';

final class _Dialogs implements FileDialogs {
  String? savePath = 'C:/out/song.musicxml';
  List<int>? written;
  String? suggested;

  @override
  Future<String?> pickProjectToOpen() async => null;
  @override
  Future<String?> pickProjectToSave({required String suggestedName}) async =>
      null;
  @override
  Future<List<int>?> pickMidiToImport() async => null;
  @override
  Future<String?> saveMidi(
    List<int> bytes, {
    required String suggestedName,
  }) async => null;
  @override
  Future<String?> saveMusicXml(
    List<int> bytes, {
    required String suggestedName,
  }) async {
    suggested = suggestedName;
    if (savePath == null) return null;
    written = bytes;
    return savePath;
  }

  @override
  Future<List<int>?> pickGuitarProToImport() async => null;
  @override
  Future<String?> pickGuitarProToSave({required String suggestedName}) async =>
      null;
  @override
  Future<void> writeFile(String path, List<int> bytes) async {}
}

Future<void> _pump(
  WidgetTester tester,
  _Dialogs dialogs,
  FakeMusicXmlService service,
) async {
  tester.view.physicalSize = const Size(1400, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        documentFactoryProvider.overrideWithValue(FakeDocumentFactory()),
        fileDialogsProvider.overrideWithValue(dialogs),
        musicXmlServiceProvider.overrideWithValue(service),
      ],
      child: const GuitarInspireApp(),
    ),
  );
}

Future<void> _export(WidgetTester tester) async {
  await tester.tap(find.text('File'));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const ValueKey('export-musicxml')));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('E13: Export MusicXML writes the file and shows the report', (
    tester,
  ) async {
    final dialogs = _Dialogs();
    final service = FakeMusicXmlService();
    await _pump(tester, dialogs, service);
    await _export(tester);

    expect(service.exports, 1);
    expect(dialogs.suggested, 'Untitled.musicxml');
    expect(utf8.decode(dialogs.written!), startsWith('<?xml'));
    expect(find.textContaining('to MusicXML'), findsOneWidget);
  });

  testWidgets('E13: a failed export shows the error and writes nothing', (
    tester,
  ) async {
    final dialogs = _Dialogs();
    final service = FakeMusicXmlService()..failNext = 'too fine';
    await _pump(tester, dialogs, service);
    await _export(tester);

    expect(dialogs.written, isNull);
    expect(
      find.text('Export failed: musicxml-export: too fine'),
      findsOneWidget,
    );
  });

  testWidgets('cancelling the save dialog shows nothing', (tester) async {
    final dialogs = _Dialogs()..savePath = null;
    final service = FakeMusicXmlService();
    await _pump(tester, dialogs, service);
    await _export(tester);

    expect(service.exports, 1);
    expect(find.textContaining('to MusicXML'), findsNothing);
  });

  test('the summary names every approximation the core reported', () {
    const r = MusicXmlReportDto(
      parts: 2,
      measures: 5,
      notes: 10,
      paddedVoices: 1,
      overfullMeasures: 1,
      tupletsWithoutBracket: 3,
      tempoRounded: 1,
      charactersRemoved: 2,
      techniquesApproximated: 4,
    );
    final s = musicXmlSummary(r);
    expect(s, startsWith('Exported 2 track(s), 5 bar(s), 10 note(s)'));
    for (final part in [
      '1 overfull bar(s) kept with all notes',
      '1 short voice(s) filled with rests',
      '3 tuplet note(s) without a bracket',
      '1 tempo(s) rounded',
      '2 character(s) removed',
      '4 technique(s) written as text',
    ]) {
      expect(s, contains(part));
    }
  });
}
