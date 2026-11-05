import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/playback_wiring.dart';
import 'src/soundfont.dart';
import 'src/startup.dart';

/// Composition root (ADR-004): loads the native core, starts the audio engine, and
/// wires the real services into the feature packages through provider overrides.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await CoreBridge.init();

  // Start the shared audio engine. A missing output device is not fatal: the app runs
  // with playback disabled and the transport bar is hidden (ADR-028).
  PlaybackFactory? playback;
  try {
    playback = await CoreBridge.startPlayback(
      soundfontPath: await bundledSoundFontPath(),
    );
  } on ApiError {
    playback = null;
  }

  runApp(
    ProviderScope(
      overrides: [
        documentFactoryProvider.overrideWithValue(CoreBridge.documents),
        ...playbackOverrides(playback, CoreBridge.midi),
      ],
      child: const StartupGate(),
    ),
  );
}
