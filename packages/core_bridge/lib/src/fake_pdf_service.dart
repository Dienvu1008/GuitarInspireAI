import 'dart:convert';
import 'dart:typed_data';

import 'document_service.dart';
import 'generated/api/layout.dart' show ShownTrackDto;
import 'generated/api/pdf.dart'
    show PaperDto, PdfExportResult, SystemSpacingDto;
import 'generated/api/types.dart' show ApiError;
import 'pdf_service.dart';

/// In-memory stand-in for the PDF export, for widget tests. It returns a minimal PDF
/// header and the font subset names the real export reports, and it rejects titles
/// with characters outside Latin, Greek and Cyrillic (as the bundled text font does
/// for CJK). It does not model pages or content: the real core is tested in Rust.
final class FakePdfService implements PdfService {
  /// Test hook: when set, the next export fails with this message.
  String? failNext;

  /// The arguments of every export requested, in order.
  final List<
    ({
      List<ShownTrackDto> shown,
      PaperDto paper,
      SystemSpacingDto spacing,
      String title,
    })
  >
  requests = [];

  @override
  Future<PdfExportResult> exportPdf(
    DocumentService document, {
    required List<ShownTrackDto> shown,
    required PaperDto paper,
    required SystemSpacingDto spacing,
    required String title,
  }) async {
    requests.add((shown: shown, paper: paper, spacing: spacing, title: title));
    final fail = failNext;
    if (fail != null) {
      failNext = null;
      throw ApiError(rule: 'pdf-export', message: fail);
    }
    for (final rune in title.runes) {
      if (rune >= 0x2E80) {
        throw ApiError(
          rule: 'pdf-export',
          message:
              "the character '${String.fromCharCode(rune)}' (U+"
              "${rune.toRadixString(16).toUpperCase().padLeft(4, '0')}) is not in "
              'the font NotoSansMono',
        );
      }
    }
    return PdfExportResult(
      bytes: Uint8List.fromList(utf8.encode('%PDF-1.7\n%%EOF\n')),
      pages: 1,
      fonts: [
        if (shown.any((s) => s.notation)) 'AAAAAA+Bravura',
        'AAAAAA+NotoSansMono',
      ],
    );
  }
}
