import 'package:app_platform/app_platform.dart';
import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:guitar_inspire/src/pdf_menu.dart';
import 'package:guitar_inspire/src/shell.dart';

final class _Dialogs implements FileDialogs {
  String? savePath = 'C:/out/song.pdf';
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
  }) async => null;
  @override
  Future<String?> savePdf(
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
  FakePdfService service,
) async {
  tester.view.physicalSize = const Size(1400, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        documentFactoryProvider.overrideWithValue(FakeDocumentFactory()),
        fileDialogsProvider.overrideWithValue(dialogs),
        pdfServiceProvider.overrideWithValue(service),
      ],
      child: const GuitarInspireApp(),
    ),
  );
}

Future<void> _open(WidgetTester tester) async {
  await tester.tap(find.text('File'));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const ValueKey('export-pdf')));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('Export PDF with the defaults writes what the view shows', (
    tester,
  ) async {
    final dialogs = _Dialogs();
    final service = FakePdfService();
    await _pump(tester, dialogs, service);
    await _open(tester);
    await tester.tap(find.byKey(const ValueKey('pdf-export-confirm')));
    await tester.pumpAndSettle();

    final r = service.requests.single;
    expect(r.paper, PaperDto.a4);
    expect(r.spacing, SystemSpacingDto.spacious);
    expect(r.title, 'Untitled');
    final container = ProviderScope.containerOf(
      tester.element(find.byType(GuitarInspireApp)),
    );
    expect(r.shown, container.read(editorProvider).shownStaves);
    expect(r.shown, isNotEmpty);
    expect(dialogs.suggested, 'Untitled.pdf');
    expect(String.fromCharCodes(dialogs.written!.take(5)), '%PDF-');
    expect(find.text('Exported 1 page(s) to PDF'), findsOneWidget);
  });

  testWidgets('the chosen paper and spacing are passed to the core', (
    tester,
  ) async {
    final dialogs = _Dialogs();
    final service = FakePdfService();
    await _pump(tester, dialogs, service);
    await _open(tester);
    await tester.tap(find.text('Letter'));
    await tester.tap(find.text('Compact'));
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('pdf-export-confirm')));
    await tester.pumpAndSettle();

    final r = service.requests.single;
    expect(r.paper, PaperDto.letter);
    expect(r.spacing, SystemSpacingDto.compact);
  });

  testWidgets('cancelling the options exports nothing', (tester) async {
    final dialogs = _Dialogs();
    final service = FakePdfService();
    await _pump(tester, dialogs, service);
    await _open(tester);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();

    expect(service.requests, isEmpty);
    expect(dialogs.written, isNull);
  });

  testWidgets('a failed export shows the error and writes nothing', (
    tester,
  ) async {
    final dialogs = _Dialogs();
    final service = FakePdfService()..failNext = 'missing glyph';
    await _pump(tester, dialogs, service);
    await _open(tester);
    await tester.tap(find.byKey(const ValueKey('pdf-export-confirm')));
    await tester.pumpAndSettle();

    expect(dialogs.written, isNull);
    expect(
      find.text('Export failed: pdf-export: missing glyph'),
      findsOneWidget,
    );
  });

  test('the title is the file name without its extension', () {
    expect(pdfTitle(null), 'Untitled');
    expect(pdfTitle(r'C:\songs\Bài hát.giproj'), 'Bài hát');
    expect(pdfTitle('/home/u/My.Song.giproj'), 'My.Song');
    expect(pdfTitle('/home/u/plain'), 'plain');
  });
}
