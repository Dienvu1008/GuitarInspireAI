import 'dart:io';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_test/flutter_test.dart';

/// An S10 fixture (CC0-1.0) written by TuxGuitar: one bar, song information set.
List<int> guitarProSample() => File(
  '${Directory.current.parent.parent.path}/crates/mc-guitarpro/tests/fixtures/'
  'tuxguitar/f11-text.gp5',
).readAsBytesSync();

/// Shared contract for [GuitarProService] implementations (ADR-036 G13). The fake and
/// the real core must agree on what the UI relies on: an import gives a document
/// whose bar count matches its report, with the file's version and its issues; an
/// unsupported file is an [ApiError] with rule `guitar-pro-import`; the export
/// version follows the extension; exporting changes nothing. The content itself is
/// checked in Rust (`mc-guitarpro` tests).
void guitarProServiceContract(
  String name,
  GuitarProService Function() service,
  DocumentFactory Function() documentFactory,
) {
  group('GuitarProService contract ($name)', () {
    test('imports a GP5 file into a document with its report', () async {
      final result = await service().importGuitarPro(guitarProSample());
      final summary = result.document.summary();
      expect(result.report.version, startsWith('FICHIER GUITAR PRO v5'));
      expect(summary.tracks, isNotEmpty);
      expect(summary.masterBars.length, result.report.bars);
      expect(result.report.tracksImported, summary.tracks.length);
      expect(result.report.issues, contains(startsWith('song information')));
      result.document.close();
    });

    test('refuses a file that is not Guitar Pro', () async {
      await expectLater(
        service().importGuitarPro('not a tab'.codeUnits),
        throwsA(
          isA<ApiError>().having((e) => e.rule, 'rule', 'guitar-pro-import'),
        ),
      );
    });

    test('knows the versions it writes', () {
      for (final ext in ['gp5', '.gp4', 'GP3']) {
        expect(service().exportExtensionSupported(ext), isTrue, reason: ext);
      }
      for (final ext in ['gpx', 'gp', 'mid', '']) {
        expect(service().exportExtensionSupported(ext), isFalse, reason: ext);
      }
    });

    test('exports the version of the extension and changes nothing', () async {
      final doc = documentFactory().create(bars: 3);
      final before = doc.summary();
      for (final (ext, version) in [
        ('gp5', 'v5.00'),
        ('gp4', 'v4.06'),
        ('gp3', 'v3.00'),
      ]) {
        final result = await service().exportGuitarPro(doc, ext);
        final header = String.fromCharCodes(result.bytes.skip(1).take(24));
        expect(header, 'FICHIER GUITAR PRO $version');
        expect(result.report.version, ext);
        expect(result.report.tracks, before.tracks.length);
        expect(result.report.bars, 3);
      }
      final after = doc.summary();
      expect(after.revision, before.revision);
      expect(after.dirty, before.dirty);
      await expectLater(
        service().exportGuitarPro(doc, 'gpx'),
        throwsA(
          isA<ApiError>().having((e) => e.rule, 'rule', 'guitar-pro-export'),
        ),
      );
      doc.close();
    });
  });
}
