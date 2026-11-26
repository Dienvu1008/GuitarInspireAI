import 'dart:convert';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_test/flutter_test.dart';

/// Shared contract for [PdfService] implementations (ADR-041 section 12). The fake and
/// the real core must agree on what the UI relies on: PDF bytes, at least one page, the
/// embedded subset names (Bravura only when notation is shown), and a `pdf-export`
/// error naming a character the fonts lack. The content is checked in Rust (`mc-pdf`).
void pdfServiceContract(
  String name,
  PdfService Function() service,
  DocumentFactory Function() documentFactory,
) {
  group('PdfService contract ($name)', () {
    late DocumentService doc;

    setUp(() => doc = documentFactory().create(bars: 3));
    tearDown(() => doc.close());

    List<ShownTrackDto> shown({required bool notation}) => [
      for (final t in doc.summary().tracks)
        ShownTrackDto(track: t.id, notation: notation, tab: true),
    ];

    test('exports PDF bytes with the embedded font subsets', () async {
      final r = await service().exportPdf(
        doc,
        shown: shown(notation: true),
        paper: PaperDto.a4,
        spacing: SystemSpacingDto.spacious,
        title: 'Song',
      );
      expect(ascii.decode(r.bytes.sublist(0, 5)), '%PDF-');
      expect(r.pages, greaterThanOrEqualTo(1));
      expect(r.fonts.length, 2);
      expect(r.fonts.where((f) => f.endsWith('+Bravura')).length, 1);
      expect(r.fonts.where((f) => f.endsWith('+NotoSansMono')).length, 1);
      for (final f in r.fonts) {
        expect(f, matches(RegExp(r'^[A-Z]{6}\+')));
      }
    });

    test('TAB only embeds the text font only', () async {
      final r = await service().exportPdf(
        doc,
        shown: shown(notation: false),
        paper: PaperDto.letter,
        spacing: SystemSpacingDto.compact,
        title: 'Song',
      );
      expect(r.fonts.single, endsWith('+NotoSansMono'));
    });

    test('a character missing from the fonts is a pdf-export error', () async {
      await expectLater(
        service().exportPdf(
          doc,
          shown: shown(notation: true),
          paper: PaperDto.a4,
          spacing: SystemSpacingDto.spacious,
          title: 'Song 曲',
        ),
        throwsA(
          isA<ApiError>()
              .having((e) => e.rule, 'rule', 'pdf-export')
              .having((e) => e.message, 'message', contains('U+66F2')),
        ),
      );
    });

    test('exporting does not change the document', () async {
      final before = doc.summary();
      await service().exportPdf(
        doc,
        shown: shown(notation: true),
        paper: PaperDto.a4,
        spacing: SystemSpacingDto.spacious,
        title: 'Song',
      );
      expect(doc.summary().revision, before.revision);
      expect(doc.summary().dirty, before.dirty);
    });
  });
}
