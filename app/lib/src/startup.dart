import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'autosave.dart';
import 'shell.dart';

/// Offers crash recovery before showing the main shell, and runs autosave for the
/// app's lifetime (slice 3e).
class StartupGate extends ConsumerStatefulWidget {
  const StartupGate({
    super.key,
    this.autosavePoll = const Duration(seconds: 30),
  });

  /// How often the core is asked whether an autosave is due.
  final Duration autosavePoll;

  @override
  ConsumerState<StartupGate> createState() => _StartupGateState();
}

class _StartupGateState extends ConsumerState<StartupGate> {
  bool _ready = false;
  late final AutosaveTimer _autosave;

  // The recovery dialog needs a Navigator; this gate's own context is above the
  // MaterialApp it builds, so the dialog uses the loading app's navigator.
  final _navigator = GlobalKey<NavigatorState>();

  @override
  void initState() {
    super.initState();
    _autosave = AutosaveTimer(ref, pollInterval: widget.autosavePoll)..start();
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      final context = _navigator.currentContext;
      if (context != null) await checkRecovery(context, ref);
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
      return MaterialApp(
        navigatorKey: _navigator,
        debugShowCheckedModeBanner: false,
        home: const Scaffold(body: Center(child: CircularProgressIndicator())),
      );
    }
    return const GuitarInspireApp();
  }
}
