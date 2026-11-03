import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/startup.dart';

/// Composition root (ADR-004): loads the native core and wires the real services into
/// the feature packages through provider overrides.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await CoreBridge.init();
  runApp(
    ProviderScope(
      overrides: [
        documentFactoryProvider.overrideWithValue(CoreBridge.documents),
      ],
      child: const StartupGate(),
    ),
  );
}
