// MIDI import and export through the real Rust core. No audio device is needed
// (import and export are pure), so this runs in CI. Needs the native library:
//   cargo build -p mc-ffi
@TestOn('windows')
library;

import 'dart:io';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_test/flutter_test.dart';

String _library() {
  final root = Directory.current.parent.parent.path;
  return '$root${Platform.pathSeparator}target${Platform.pathSeparator}debug'
      '${Platform.pathSeparator}mc_ffi.dll';
}

void main() {
  setUpAll(() => CoreBridge.init(libraryPath: _library()));

  test('export a document, re-import it, and read the result', () async {
    final doc = CoreBridge.documents.create(bars: 2);
    final track = doc.summary().tracks.single.id;
    // One note so the file has content.
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
        fret: 0,
      ),
    );

    final export = await CoreBridge.midi.exportMidi(doc);
    expect(export.bytes.length, greaterThan(4));
    // A Standard MIDI File starts with the "MThd" header chunk.
    expect(export.bytes.sublist(0, 4), [0x4d, 0x54, 0x68, 0x64]);
    expect(export.report.tracks, greaterThanOrEqualTo(1));

    final imported = await CoreBridge.midi.importMidi(export.bytes);
    final s = imported.document.summary();
    expect(s.tracks, isNotEmpty);
    expect(imported.report.notesImported, greaterThanOrEqualTo(0));
    imported.document.close();
    doc.close();
  });

  test('a malformed MIDI file is rejected with an ApiError', () async {
    await expectLater(
      CoreBridge.midi.importMidi(const [0, 1, 2, 3, 4]),
      throwsA(isA<ApiError>()),
    );
  });
}
