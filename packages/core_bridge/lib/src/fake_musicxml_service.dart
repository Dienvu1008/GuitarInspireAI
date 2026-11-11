import 'dart:convert';
import 'dart:typed_data';

import 'document_service.dart';
import 'generated/api/musicxml.dart'
    show MusicXmlExportResult, MusicXmlReportDto;
import 'generated/api/types.dart' show ApiError;
import 'musicxml_service.dart';

/// In-memory stand-in for the MusicXML export, for widget tests. It returns a small
/// well-formed document with one part per track of the document and a report with
/// those counts. It does not model the content: the real core is tested in Rust.
final class FakeMusicXmlService implements MusicXmlService {
  /// Test hook: when set, the next export fails with this message.
  String? failNext;

  /// Number of exports requested.
  int exports = 0;

  @override
  Future<MusicXmlExportResult> exportMusicXml(DocumentService document) async {
    exports++;
    final fail = failNext;
    if (fail != null) {
      failNext = null;
      throw ApiError(rule: 'musicxml-export', message: fail);
    }
    final summary = document.summary();
    final tracks = summary.tracks;
    final bars = summary.masterBars.length;
    final buffer = StringBuffer()
      ..writeln('<?xml version="1.0" encoding="UTF-8" standalone="no"?>')
      ..writeln('<score-partwise version="4.0">')
      ..writeln('  <part-list>');
    for (final (i, t) in tracks.indexed) {
      buffer.writeln(
        '    <score-part id="P${i + 1}"><part-name>${_escape(t.name)}'
        '</part-name></score-part>',
      );
    }
    buffer.writeln('  </part-list>');
    for (var i = 0; i < tracks.length; i++) {
      buffer.writeln('  <part id="P${i + 1}">');
      for (var m = 1; m <= bars; m++) {
        buffer.writeln('    <measure number="$m"/>');
      }
      buffer.writeln('  </part>');
    }
    buffer.writeln('</score-partwise>');
    return MusicXmlExportResult(
      bytes: Uint8List.fromList(utf8.encode(buffer.toString())),
      report: MusicXmlReportDto(
        parts: tracks.length,
        measures: bars,
        notes: 0,
        paddedVoices: 0,
        overfullMeasures: 0,
        tupletsWithoutBracket: 0,
        tempoRounded: 0,
        charactersRemoved: 0,
        techniquesApproximated: 0,
      ),
    );
  }

  static String _escape(String s) => s
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;');
}
