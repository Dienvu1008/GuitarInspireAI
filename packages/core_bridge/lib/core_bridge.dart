/// Public facade of the Music Core for feature packages (FF-05): services and the plain
/// DTO types. The generated entry points stay private; features never import
/// `src/generated` directly.
library;

export 'src/document_service.dart';
export 'src/generated/api/document.dart' show RecoveryEntryDto;
export 'src/generated/api/guitarpro.dart'
    show
        GuitarProExportReportDto,
        GuitarProExportResult,
        GuitarProImportReportDto;
export 'src/generated/api/layout.dart' hide defaultMetrics;
export 'src/generated/api/midi.dart'
    show ExportReportDto, ExportResult, ImportReportDto;
export 'src/generated/api/musicxml.dart'
    show MusicXmlExportResult, MusicXmlReportDto;
export 'src/generated/api/pattern.dart'
    hide patterns, previewPattern, samplePattern;
export 'src/generated/api/pattern_grid.dart' hide gestures, patternGrid;
export 'src/generated/api/types.dart';
export 'src/guitar_pro_service.dart' show GuitarProImport, GuitarProService;
export 'src/musicxml_service.dart' show MusicXmlService;
export 'src/playback_service.dart';
export 'src/rust_document_service.dart' show CoreBridge, RustDocumentService;
export 'src/tuner_service.dart';
