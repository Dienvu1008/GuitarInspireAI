import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:tools_feature/tools_feature.dart';

/// Wires the chord finder (ADR-029 T6/T7/T8) to the editor's active document and track.
/// Lives in the composition root because it joins two feature packages (FF-05 forbids a
/// feature importing another).
///
/// - `chordLookupProvider`: calls the active document's `chordVoicings` with the editor's
///   active track (its tuning + capo, T8).
/// - `insertVoicingProvider`: inserts a voicing at the cursor by issuing ordinary
///   `setNote` commands through the editor (T7, GR-05: no music rule in Dart). Each
///   string of the voicing becomes one note on the beat at the cursor; a beat is created
///   first if the cursor is on the append slot. `null` is not used here (insertion is
///   always available when a document is open).
List<Override> toolsOverrides() => [
  chordLookupProvider.overrideWith((ref) {
    final service = ref.watch(editorProvider.select((s) => s.service));
    final track = ref.watch(editorProvider.select((s) => s.activeTrack));
    return (String symbol, {int max = 5}) =>
        service.chordVoicings(track, symbol, max: max);
  }),
  insertVoicingProvider.overrideWith((ref) {
    return (VoicingDto voicing) => _insertAtCursor(ref, voicing);
  }),
];

Future<void> _insertAtCursor(Ref ref, VoicingDto voicing) async {
  final editor = ref.read(editorProvider.notifier);
  final s = ref.read(editorProvider);
  final track = s.activeTrack;
  final bar = s.cursor.bar;
  final voice = s.cursor.voice;

  // Ensure a beat exists at the cursor; if on the append slot, create one first.
  var beatIndex = s.cursor.beat;
  if (beatIndex == null) {
    final index = s.service.beats(track, bar, voice).length;
    final ok = await editor.execute(
      CommandDto.insertBeat(
        track: track,
        bar: bar,
        voice: voice,
        index: index,
        duration: const DurationDto(base: 2, dots: 0),
      ),
    );
    if (!ok) return;
    beatIndex = index;
    editor.moveCursor(
      EditorCursor(bar: bar, voice: voice, string: s.cursor.string, beat: index),
    );
  }

  final beats = ref.read(editorProvider).service.beats(track, bar, voice);
  if (beatIndex >= beats.length) return;
  final beatId = beats[beatIndex].id;

  // One setNote per fretted/open string of the voicing (string 1 first; -1 muted).
  for (var i = 0; i < voicing.frets.length; i++) {
    final fret = voicing.frets[i];
    if (fret < 0) continue; // muted string
    final string = i + 1; // 1-based
    await editor.execute(
      CommandDto.setNote(
        beat: BeatAddressDto(track: track, bar: bar, voice: voice, beat: beatId),
        string: string,
        fret: fret,
      ),
    );
  }
}
