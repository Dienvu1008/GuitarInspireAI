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

  test(
    'autosave, recover after a lost session, save clears recovery',
    () async {
      await CoreBridge.init(libraryPath: _library());
      final factory = CoreBridge.documents;
      final dir = await Directory.systemTemp.createTemp('core_bridge_recovery');
      final rec = '${dir.path}${Platform.pathSeparator}recovery';
      final d = factory.create(bars: 1);
      await d.execute(
        CommandDto.insertBeat(
          track: d.summary().tracks.single.id,
          bar: 0,
          voice: 0,
          index: 0,
          duration: const DurationDto(base: 3, dots: 0),
        ),
      );
      await d.writeAutosave(rec);
      d.close(); // lost session: never saved

      final entries = factory.recoveryEntries(rec);
      expect(entries, hasLength(1));
      expect(entries.single.source, isNull);
      final r = await factory.openFromRecovery(rec, entries.single.projectId);
      final s = r.summary();
      expect(s.dirty, isTrue);
      expect(r.beats(s.tracks.single.id, 0, 0).single.duration.base, 3);

      await r.saveAs('${dir.path}${Platform.pathSeparator}recovered.giproj');
      expect(factory.recoveryEntries(rec), isEmpty);
      r.close();
      await dir.delete(recursive: true);
    },
  );
}
