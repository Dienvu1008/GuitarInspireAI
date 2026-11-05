import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:playback_feature/playback_feature.dart';
import 'package:tools_feature/tools_feature.dart';

/// Whether the audio engine started (an output device was found). The transport bar is
/// shown only when true; MIDI import/export work regardless.
final playbackAvailableProvider = Provider<bool>((ref) => false);

/// Whether the tuner panel is shown (Tools menu toggle). The microphone opens when the
/// panel is shown and closes when it is hidden (privacy), driven by the panel's lifecycle.
final tunerVisibleProvider = NotifierProvider<TunerVisible, bool>(TunerVisible.new);

/// Boolean toggle for the tuner panel (a small Notifier instead of the legacy
/// StateProvider, Q-12: no code generation).
final class TunerVisible extends Notifier<bool> {
  @override
  bool build() => false;

  void toggle() => state = !state;
}

/// Binds the playback feature to the editor's active document (ADR-028). This lives in
/// the composition root because it joins two feature packages, which may not import each
/// other (FF-05).
///
/// [playbackServiceProvider] is derived from the shared engine factory and the document
/// the editor is showing. When the editor replaces its document (new, open, recover,
/// MIDI import), the provider rebuilds and a fresh service is bound; the old one is
/// closed. If starting the engine failed (no audio device), [playbackFactoryProvider]
/// is null and the override throws, which the shell guards by not showing transport.
List<Override> playbackOverrides(PlaybackFactory? factory, MidiService midi) => [
  playbackAvailableProvider.overrideWithValue(factory != null),
  if (factory != null) playbackFactoryProvider.overrideWithValue(factory),
  midiServiceProvider.overrideWithValue(midi),
  if (factory != null)
    playbackServiceProvider.overrideWith((ref) {
      // Rebuilds when the active document changes; close the previous service.
      final document = ref.watch(
        editorProvider.select((s) => s.service),
      );
      final service = factory.forDocument(document);
      ref.onDispose(service.close);
      return service;
    }),
  // The playback cursor overlay on the TAB (ADR-028). Only when the engine started.
  if (factory != null)
    cursorOverlayProvider.overrideWithValue(
      (tickToX, height) => PlaybackCursor(tickToX: tickToX, height: height),
    ),
];

/// Wires the tuner (ADR-029) to the editor's active document and track. The tuner is
/// independent of the playback engine: it works even when `playbackOverrides` had no
/// engine (no output device). The factory binds to the current document; the referenced
/// track follows the editor's active track (T8), so retuning or switching tracks changes
/// the reading with no restart.
List<Override> tunerOverrides() => [
  tunerFactoryProvider.overrideWith((ref) {
    final document = ref.watch(editorProvider.select((s) => s.service));
    return CoreBridge.tunerFor(document);
  }),
  tunerTrackProvider.overrideWith(
    (ref) => ref.watch(editorProvider.select((s) => s.activeTrack)),
  ),
];
