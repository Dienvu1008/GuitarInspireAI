// MusicXML export through the real Rust core (ADR-033 E13). No audio device is
// needed. Needs the native library:
//   cargo build -p mc-ffi
@TestOn('windows')
library;

import 'dart:io';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_test/flutter_test.dart';

import 'contract/musicxml_service_contract.dart';

String _library() {
  final root = Directory.current.parent.parent.path;
  return '$root${Platform.pathSeparator}target${Platform.pathSeparator}debug'
      '${Platform.pathSeparator}mc_ffi.dll';
}

void main() {
  setUpAll(() => CoreBridge.init(libraryPath: _library()));

  musicXmlServiceContract(
    'rust',
    () => CoreBridge.musicXml,
    () => CoreBridge.documents,
  );

  test('the export reports the notes of the document', () async {
    final doc = CoreBridge.documents.create(bars: 2);
    final track = doc.summary().tracks.single.id;
    await doc.execute(
      CommandDto.insertBeat(
        track: track,
        bar: 0,
        voice: 0,
        index: 0,
        duration: const DurationDto(base: 2, dots: 0),
      ),
    );
    final beat = doc.beats(track, 0, 0).single.id;
    await doc.execute(
      CommandDto.setNote(
        beat: BeatAddressDto(track: track, bar: 0, voice: 0, beat: beat),
        string: 1,
        fret: 5,
      ),
    );
    final result = await CoreBridge.musicXml.exportMusicXml(doc);
    expect(result.report.notes, 1);
    // The one quarter note in a 4/4 bar is padded with a rest.
    expect(result.report.paddedVoices, 1);
    doc.close();
  });
}
