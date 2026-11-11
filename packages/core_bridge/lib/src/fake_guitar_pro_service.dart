import 'dart:typed_data';

import 'document_service.dart';
import 'fake_document_service.dart';
import 'generated/api/guitarpro.dart'
    show
        GuitarProExportReportDto,
        GuitarProExportResult,
        GuitarProImportReportDto;
import 'generated/api/types.dart' show ApiError;
import 'guitar_pro_service.dart';

/// In-memory stand-in for Guitar Pro import and export, for widget tests. Import of
/// any bytes that start with the Guitar Pro version text gives a 4-bar fake document
/// and a report with one issue; other bytes fail as the core does. Export returns
/// bytes that start with the version text of the chosen extension. It does not model
/// the content: the real core is tested in Rust.
final class FakeGuitarProService implements GuitarProService {
  /// Test hook: when set, the next import or export fails with this message.
  String? failNext;

  int imports = 0;
  int exports = 0;

  static const _versions = {'gp3': 'v3.00', 'gp4': 'v4.06', 'gp5': 'v5.00'};

  static String? _key(String extension) {
    final key = extension.toLowerCase().replaceFirst(RegExp(r'^\.'), '');
    return _versions.containsKey(key) ? key : null;
  }

  ApiError? _failure(String rule) {
    final fail = failNext;
    if (fail == null) return null;
    failNext = null;
    return ApiError(rule: rule, message: fail);
  }

  @override
  Future<GuitarProImport> importGuitarPro(List<int> bytes) async {
    imports++;
    final failure = _failure('guitar-pro-import');
    if (failure != null) throw failure;
    final text = String.fromCharCodes(bytes.skip(1).take(24));
    if (!text.startsWith('FICHIER GUITAR PRO v')) {
      throw ApiError(
        rule: 'guitar-pro-import',
        message: 'unsupported file: not a Guitar Pro file',
      );
    }
    return GuitarProImport(
      document: FakeDocumentService(bars: 4),
      report: GuitarProImportReportDto(
        version: text,
        tracksImported: 1,
        bars: 4,
        notesInFile: 0,
        notesImported: 0,
        notesDropped: 0,
        issues: const ['song information not kept: 1 (first in bar 1)'],
      ),
    );
  }

  @override
  bool exportExtensionSupported(String extension) => _key(extension) != null;

  @override
  Future<GuitarProExportResult> exportGuitarPro(
    DocumentService document,
    String extension,
  ) async {
    exports++;
    final failure = _failure('guitar-pro-export');
    if (failure != null) throw failure;
    final key = _key(extension);
    if (key == null) {
      throw ApiError(
        rule: 'guitar-pro-export',
        message:
            "unknown Guitar Pro extension '$extension'; use gp5, gp4 or gp3",
      );
    }
    final header = 'FICHIER GUITAR PRO ${_versions[key]}';
    final bytes = Uint8List(31)
      ..[0] = header.length
      ..setRange(1, 1 + header.length, header.codeUnits);
    final summary = document.summary();
    return GuitarProExportResult(
      bytes: bytes,
      report: GuitarProExportReportDto(
        version: key,
        tracks: summary.tracks.length,
        bars: summary.masterBars.length,
        notesWritten: 0,
        notesDropped: 0,
        issues: const [],
      ),
    );
  }
}
