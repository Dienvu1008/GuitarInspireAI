import 'dart:convert';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_test/flutter_test.dart';

/// Shared contract for [MusicXmlService] implementations (ADR-033 E13). The fake and
/// the real core must agree on what the UI relies on: UTF-8 MusicXML 4.0 bytes, one
/// part per track named after it, one measure per master bar, and a report with those
/// counts. The content itself is checked in Rust (`mc-musicxml` tests).
void musicXmlServiceContract(
  String name,
  MusicXmlService Function() service,
  DocumentFactory Function() documentFactory,
) {
  group('MusicXmlService contract ($name)', () {
    late DocumentService doc;

    setUp(() => doc = documentFactory().create(bars: 3));
    tearDown(() => doc.close());

    test(
      'exports MusicXML 4.0 with one part per track and its measures',
      () async {
        final result = await service().exportMusicXml(doc);
        final text = utf8.decode(result.bytes);
        expect(text, startsWith('<?xml'));
        expect(text, contains('<score-partwise version="4.0">'));
        final tracks = doc.summary().tracks;
        expect(RegExp('<part id=').allMatches(text).length, tracks.length);
        for (final t in tracks) {
          expect(text, contains('<part-name>${t.name}</part-name>'));
        }
        expect(RegExp('<measure ').allMatches(text).length, 3 * tracks.length);
        expect(result.report.parts, tracks.length);
        expect(result.report.measures, 3);
      },
    );

    test('exporting does not change the document', () async {
      final before = doc.summary();
      await service().exportMusicXml(doc);
      final after = doc.summary();
      expect(after.revision, before.revision);
      expect(after.dirty, before.dirty);
    });
  });
}
