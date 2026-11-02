import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/autosave.dart';
import 'src/shell.dart';

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
      child: const _StartupGate(),
    ),
  );
}

/// Checks for crash-recovery files before showing the main shell (slice 3e).
class _StartupGate extends ConsumerStatefulWidget {
  const _StartupGate();

  @override
  ConsumerState<_StartupGate> createState() => _StartupGateState();
}

class _StartupGateState extends ConsumerState<_StartupGate> {
  bool _ready = false;
  late final AutosaveTimer _autosave;

  @override
  void initState() {
    super.initState();
    _autosave = AutosaveTimer(ref)..start();
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      await checkRecovery(context, ref);
      if (mounted) setState(() => _ready = true);
    });
  }

  @override
  void dispose() {
    _autosave.stop();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (!_ready) {
      return const MaterialApp(
        home: Scaffold(body: Center(child: CircularProgressIndicator())),
      );
    }
    return const GuitarInspireApp();
  }
}
