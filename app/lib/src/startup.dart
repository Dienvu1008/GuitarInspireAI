import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'dart:ui' show AppExitResponse;

import 'package:core_bridge/core_bridge.dart' show ApiError;
import 'package:editor_feature/editor_feature.dart';

import 'autosave.dart';
import 'close_guard.dart';
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

  /// Window close (and other cancelable exit requests) ask before quitting.
  late final AppLifecycleListener _lifecycle = AppLifecycleListener(
    onExitRequested: onExitRequested,
  );

  /// ADR-030 section 3: ask about unsaved work; on a clean exit end the session, so
  /// the next start does not offer recovery. Public for tests.
  Future<AppExitResponse> onExitRequested() async {
    if (!await confirmUnsavedWork(ref.read)) return AppExitResponse.cancel;
    _autosave.stop();
    try {
      final dir = await ref.read(appPathsProvider).recoveryDirectory();
      ref.read(documentFactoryProvider).endSession(dir);
    } on ApiError {
      // The marker stays: the next start offers recovery, which is the safe side.
    } catch (_) {
      // No app data directory (test environment).
    }
    return AppExitResponse.exit;
  }

  @override
  void initState() {
    super.initState();
    _lifecycle; // create the listener now
    _autosave = AutosaveTimer(ref, pollInterval: widget.autosavePoll)..start();
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      final context = _navigator.currentContext;
      if (context != null) await checkRecovery(context, ref);
      if (mounted) setState(() => _ready = true);
    });
  }

  @override
  void dispose() {
    _lifecycle.dispose();
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
