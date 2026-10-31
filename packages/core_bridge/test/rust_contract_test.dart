// Runs the contract against the real Rust core. Needs the native library built first:
//   cargo build -p mc-ffi
// The test loads target/debug/mc_ffi.dll (Windows) from the workspace root.
@TestOn('windows')
library;

import 'dart:io';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_test/flutter_test.dart';

import 'contract/document_service_contract.dart';
import 'fake_contract_test.dart' show testMetrics;

String _library() {
  final root = Directory.current.parent.parent.path;
  return '$root${Platform.pathSeparator}target${Platform.pathSeparator}debug'
      '${Platform.pathSeparator}mc_ffi.dll';
}

void main() {
  documentServiceContract('rust', () async {
    await CoreBridge.init(libraryPath: _library());
    return CoreBridge.documents;
  }, testMetrics);

  test('save as, reopen and save again through the real core', () async {
    await CoreBridge.init(libraryPath: _library());
    final dir = await Directory.systemTemp.createTemp('core_bridge_test');
    final path = '${dir.path}${Platform.pathSeparator}song.giproj';
    final d = CoreBridge.documents.create(bars: 1);
    final track = d.summary().tracks.single.id;
    await d.execute(
      CommandDto.insertBeat(
        track: track,
        bar: 0,
        voice: 0,
        index: 0,
        duration: const DurationDto(base: 2, dots: 0),
      ),
    );
    await d.saveAs(path);
    expect(d.summary().dirty, isFalse);
    d.close();
    final reopened = await CoreBridge.documents.open(path);
    expect(reopened.beats(reopened.summary().tracks.single.id, 0, 0).length, 1);
    await reopened.save();
    reopened.close();
    await dir.delete(recursive: true);
  });
}
