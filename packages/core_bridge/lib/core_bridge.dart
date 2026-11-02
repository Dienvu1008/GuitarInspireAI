/// Public facade of the Music Core for feature packages (FF-05): services and the plain
/// DTO types. The generated entry points stay private; features never import
/// `src/generated` directly.
library;

export 'src/document_service.dart';
export 'src/generated/api/document.dart' show RecoveryEntryDto;
export 'src/generated/api/layout.dart' hide defaultMetrics;
export 'src/generated/api/types.dart';
export 'src/rust_document_service.dart' show CoreBridge, RustDocumentService;
