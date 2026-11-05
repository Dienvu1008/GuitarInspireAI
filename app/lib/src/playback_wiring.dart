import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:playback_feature/playback_feature.dart';

/// Whether the audio engine started (an output device was found). The transport bar is
/// shown only when true; MIDI import/export work regardless.
final playbackAvailableProvider = Provider<bool>((ref) => false);

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
];
