// Guitar Pro import and export through the real Rust core (ADR-036 G13). No audio
// device is needed. Needs the native library:
//   cargo build -p mc-ffi
@TestOn('windows')
library;

import 'dart:io';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_test/flutter_test.dart';

import 'contract/guitar_pro_service_contract.dart';

String _library() {
  final root = Directory.current.parent.parent.path;
  return '$root${Platform.pathSeparator}target${Platform.pathSeparator}debug'
      '${Platform.pathSeparator}mc_ffi.dll';
}

void main() {
  setUpAll(() => CoreBridge.init(libraryPath: _library()));

  guitarProServiceContract(
    'rust',
    () => CoreBridge.guitarPro,
    () => CoreBridge.documents,
  );

  test('an exported file imports again with the same notes', () async {
    final imported = await CoreBridge.guitarPro.importGuitarPro(
      guitarProSample(),
    );
    final exported = await CoreBridge.guitarPro.exportGuitarPro(
      imported.document,
      'gp5',
    );
    final again = await CoreBridge.guitarPro.importGuitarPro(exported.bytes);
    expect(again.report.notesImported, imported.report.notesImported);
    expect(exported.report.notesWritten, imported.report.notesImported);
    imported.document.close();
    again.document.close();
  });
}
