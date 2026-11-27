// PDF export through the real Rust core (ADR-041 section 12). No audio device is
// needed. Needs the native library:
//   cargo build -p mc-ffi
@TestOn('windows')
library;

import 'dart:io';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_test/flutter_test.dart';

import 'contract/pdf_service_contract.dart';

String _library() {
  final root = Directory.current.parent.parent.path;
  return '$root${Platform.pathSeparator}target${Platform.pathSeparator}debug'
      '${Platform.pathSeparator}mc_ffi.dll';
}

void main() {
  setUpAll(() => CoreBridge.init(libraryPath: _library()));

  pdfServiceContract('rust', () => CoreBridge.pdf, () => CoreBridge.documents);
}
