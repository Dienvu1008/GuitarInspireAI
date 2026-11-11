import 'document_service.dart';
import 'generated/api/guitarpro.dart' as api;
import 'generated/api/guitarpro.dart'
    show GuitarProExportResult, GuitarProImportReportDto;
import 'rust_document_service.dart'
    show rustDocumentHandle, rustServiceFromHandle;

/// Guitar Pro import and export (ADR-036). Independent of the audio engine. The Rust
/// core decides the content and the version (GR-05); this facade only passes bytes
/// and returns the reports. Implementations: [RustGuitarProService] and
/// `FakeGuitarProService` for widget tests; both pass one shared contract.
abstract interface class GuitarProService {
  /// Import a GP3, GP4 or GP5 (5.00) file into a new, unsaved document. The version is
  /// recognised from the content. Throws [ApiError] (rule `guitar-pro-import`) for an
  /// unsupported, unreadable or oversized file.
  Future<GuitarProImport> importGuitarPro(List<int> bytes);

  /// Whether the core writes the version named by [extension] (`gp5`, `.gp4` ...).
  bool exportExtensionSupported(String extension);

  /// Export [document] in the version named by [extension]. Every track is written,
  /// whatever is muted. Throws [ApiError] (rule `guitar-pro-export`) if the score
  /// cannot be written or the extension is not a supported version.
  Future<GuitarProExportResult> exportGuitarPro(
    DocumentService document,
    String extension,
  );
}

/// A document produced by Guitar Pro import, with the import report.
final class GuitarProImport {
  const GuitarProImport({required this.document, required this.report});

  final DocumentService document;
  final GuitarProImportReportDto report;
}

/// [GuitarProService] over `mc-guitarpro` through the bridge.
final class RustGuitarProService implements GuitarProService {
  const RustGuitarProService();

  @override
  Future<GuitarProImport> importGuitarPro(List<int> bytes) async {
    final result = await api.importGuitarPro(bytes: bytes);
    return GuitarProImport(
      document: rustServiceFromHandle(result.document),
      report: result.report,
    );
  }

  @override
  bool exportExtensionSupported(String extension) =>
      api.guitarProExportExtensionSupported(extension_: extension);

  @override
  Future<GuitarProExportResult> exportGuitarPro(
    DocumentService document,
    String extension,
  ) => api.exportGuitarPro(
    document: rustDocumentHandle(document),
    extension_: extension,
  );
}
