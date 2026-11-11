// ADR-036 G13: the File menu items "Import Guitar Pro..." and "Export Guitar Pro...".
import 'package:app_platform/app_platform.dart';
import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:guitar_inspire/src/guitar_pro_menu.dart';
import 'package:guitar_inspire/src/shell.dart';

/// The version text a Guitar Pro 5 file starts with (enough for the fake service).
List<int> _gp5() => [
  24,
  ...'FICHIER GUITAR PRO v5.00'.codeUnits,
  ...List.filled(6, 0),
];

final class _Dialogs implements FileDialogs {
  List<int>? importBytes;
  String? savePath;
  String? suggested;
  String? writtenPath;
  List<int>? written;

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
  }) async => null;
  @override
  Future<List<int>?> pickGuitarProToImport() async => importBytes;
  @override
  Future<String?> pickGuitarProToSave({required String suggestedName}) async {
    suggested = suggestedName;
    return savePath;
  }

  @override
  Future<void> writeFile(String path, List<int> bytes) async {
    writtenPath = path;
    written = bytes;
  }
}

Future<void> _pump(
  WidgetTester tester,
  _Dialogs dialogs,
  FakeGuitarProService service,
) async {
  tester.view.physicalSize = const Size(1400, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        documentFactoryProvider.overrideWithValue(FakeDocumentFactory()),
        fileDialogsProvider.overrideWithValue(dialogs),
        guitarProServiceProvider.overrideWithValue(service),
      ],
      child: const GuitarInspireApp(),
    ),
  );
}

Future<void> _menu(WidgetTester tester, String key) async {
  await tester.tap(find.text('File'));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(ValueKey(key)));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('G13: import opens the file and shows the report', (
    tester,
  ) async {
    final dialogs = _Dialogs()..importBytes = _gp5();
    final service = FakeGuitarProService();
    await _pump(tester, dialogs, service);
    await _menu(tester, 'import-guitar-pro');

    expect(service.imports, 1);
    expect(find.textContaining('from Guitar Pro'), findsOneWidget);
    expect(find.textContaining('song information not kept'), findsOneWidget);
  });

  testWidgets('G13: a file that is not Guitar Pro shows the error', (
    tester,
  ) async {
    final dialogs = _Dialogs()..importBytes = 'hello'.codeUnits;
    await _pump(tester, dialogs, FakeGuitarProService());
    await _menu(tester, 'import-guitar-pro');

    expect(find.textContaining('Import failed: guitar-pro-import:'), findsOne);
  });

  testWidgets('cancelling the import dialog imports nothing', (tester) async {
    final service = FakeGuitarProService();
    await _pump(tester, _Dialogs(), service);
    await _menu(tester, 'import-guitar-pro');

    expect(service.imports, 0);
    expect(find.byType(SnackBar), findsNothing);
  });

  testWidgets('G13: export writes the version of the chosen extension', (
    tester,
  ) async {
    final dialogs = _Dialogs()..savePath = 'C:/out/song.gp4';
    final service = FakeGuitarProService();
    await _pump(tester, dialogs, service);
    await _menu(tester, 'export-guitar-pro');

    expect(dialogs.suggested, 'Untitled.gp5');
    expect(dialogs.writtenPath, 'C:/out/song.gp4');
    expect(
      String.fromCharCodes(dialogs.written!.skip(1).take(24)),
      'FICHIER GUITAR PRO v4.06',
    );
    expect(find.textContaining('to Guitar Pro (GP4)'), findsOneWidget);
  });

  testWidgets('G13: a failed export shows the error and writes nothing', (
    tester,
  ) async {
    final dialogs = _Dialogs()..savePath = 'C:/out/song.gp5';
    final service = FakeGuitarProService()..failNext = 'too many strings';
    await _pump(tester, dialogs, service);
    await _menu(tester, 'export-guitar-pro');

    expect(dialogs.written, isNull);
    expect(
      find.text('Export failed: guitar-pro-export: too many strings'),
      findsOneWidget,
    );
  });

  testWidgets('an extension that is not a version is refused by the core', (
    tester,
  ) async {
    final dialogs = _Dialogs()..savePath = 'C:/out/song.txt';
    await _pump(tester, dialogs, FakeGuitarProService());
    await _menu(tester, 'export-guitar-pro');

    expect(dialogs.written, isNull);
    expect(find.textContaining("unknown Guitar Pro extension 'txt'"), findsOne);
  });

  testWidgets('cancelling the save dialog exports nothing', (tester) async {
    final service = FakeGuitarProService();
    await _pump(tester, _Dialogs(), service);
    await _menu(tester, 'export-guitar-pro');

    expect(service.exports, 0);
    expect(find.byType(SnackBar), findsNothing);
  });

  test('the summaries show the counts and at most three issue lines', () {
    const r = GuitarProImportReportDto(
      version: 'FICHIER GUITAR PRO v5.00',
      tracksImported: 2,
      bars: 8,
      notesInFile: 40,
      notesImported: 38,
      notesDropped: 2,
      issues: ['a: 1', 'b: 2', 'c: 3', 'd: 4', 'e: 5'],
    );
    expect(
      guitarProImportSummary(r),
      'Imported 2 track(s), 8 bar(s), 38 of 40 note(s) from Guitar Pro '
      '(2 dropped); a: 1; b: 2; c: 3; and 2 more',
    );
    const e = GuitarProExportReportDto(
      version: 'gp3',
      tracks: 1,
      bars: 4,
      notesWritten: 10,
      notesDropped: 0,
      issues: [],
    );
    expect(
      guitarProExportSummary(e),
      'Exported 1 track(s), 4 bar(s), 10 note(s) to Guitar Pro (GP3)',
    );
  });
}
