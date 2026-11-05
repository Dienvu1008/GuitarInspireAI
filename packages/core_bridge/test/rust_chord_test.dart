// Chord finder through the real Rust core (ADR-029 T6). No audio device needed. Needs
// the native library: cargo build -p mc-ffi.
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

  test('C on a standard-tuned track returns the open shape first', () {
    final doc = CoreBridge.documents.create(bars: 1);
    final track = doc.summary().tracks.single.id;
    final result = doc.chordVoicings(track, 'C', max: 5);
    expect(result.symbol, 'C');
    expect(result.tones, containsAll(['C', 'E', 'G']));
    expect(result.voicings, isNotEmpty);
    // The Guitar context orders open C (x32010) first.
    expect(result.voicings.first.chartText, 'x32010');
    // frets are per string, string 1 first: [0,1,0,2,3,-1].
    expect(result.voicings.first.frets, [0, 1, 0, 2, 3, -1]);
    doc.close();
  });

  test('a bad chord symbol is a typed ApiError', () {
    final doc = CoreBridge.documents.create(bars: 1);
    final track = doc.summary().tracks.single.id;
    expect(
      () => doc.chordVoicings(track, 'zzz', max: 5),
      throwsA(isA<ApiError>()),
    );
    doc.close();
  });
}
