import 'document_service.dart';
import 'generated/api/layout.dart' show ShownTrackDto;
import 'generated/api/pdf.dart' as api;
import 'generated/api/pdf.dart'
    show PaperDto, PdfExportResult, SystemSpacingDto;
import 'rust_document_service.dart' show rustDocumentHandle;

/// PDF export (ADR-041 section 12). The Rust core paginates, lays out and paints
/// (GR-05); this facade only returns the file bytes. Implementations: [RustPdfService]
/// and `FakePdfService` for widget tests; both pass one shared contract.
abstract interface class PdfService {
  /// Export the [shown] staves of [document] (what the view shows) on [paper], with
  /// [spacing] between systems and [title] on page 1. Throws [ApiError] (rule
  /// `pdf-export`) when a character is missing from the bundled fonts.
  Future<PdfExportResult> exportPdf(
    DocumentService document, {
    required List<ShownTrackDto> shown,
    required PaperDto paper,
    required SystemSpacingDto spacing,
    required String title,
  });
}

/// [PdfService] over `mc-pdf` through the bridge.
final class RustPdfService implements PdfService {
  const RustPdfService();

  @override
  Future<PdfExportResult> exportPdf(
    DocumentService document, {
    required List<ShownTrackDto> shown,
    required PaperDto paper,
    required SystemSpacingDto spacing,
    required String title,
  }) => api.exportPdf(
    document: rustDocumentHandle(document),
    shown: shown,
    paper: paper,
    spacing: spacing,
    title: title,
  );
}
