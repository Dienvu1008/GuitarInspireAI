import 'document_service.dart';
import 'generated/api/musicxml.dart' as api;
import 'generated/api/musicxml.dart' show MusicXmlExportResult;
import 'rust_document_service.dart' show rustDocumentHandle;

/// MusicXML export (ADR-033). Independent of the audio engine. The Rust core decides
/// the content (GR-05); this facade only returns the file bytes and the report.
/// Implementations: [RustMusicXmlService] and `FakeMusicXmlService` for widget tests;
/// both pass one shared contract.
abstract interface class MusicXmlService {
  /// Export [document] as MusicXML 4.0 (uncompressed `.musicxml`). Every track is
  /// written, whatever is muted, soloed or hidden. Throws [ApiError] if the score
  /// cannot be written.
  Future<MusicXmlExportResult> exportMusicXml(DocumentService document);
}

/// [MusicXmlService] over `mc-musicxml` through the bridge.
final class RustMusicXmlService implements MusicXmlService {
  const RustMusicXmlService();

  @override
  Future<MusicXmlExportResult> exportMusicXml(DocumentService document) =>
      api.exportMusicxml(document: rustDocumentHandle(document));
}
