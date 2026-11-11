import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'document_controller.dart';

/// Inspector panel: properties of the selected beat and note. Phase 3: tie, dead,
/// let ring. ADR-035: accent, palm mute, string-percussive and the beat stroke. Every
/// control is a standard Material control, so Tab, Space and Enter work; there are no
/// shortcuts here. Each change sends one command; musical rules and what the flags
/// mean are owned by the core (GR-05).
class Inspector extends ConsumerWidget {
  const Inspector({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = ref.watch(editorProvider);
    final cursor = s.cursor;
    final beat = _currentBeat(s);
    if (beat == null) {
      return const Padding(
        padding: EdgeInsets.all(16),
        child: Text('No beat selected.'),
      );
    }
    final note = beat.notes.where((n) => n.string == cursor.string).firstOrNull;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(12, 10, 12, 4),
          child: Text(
            'Beat – bar ${cursor.bar + 1}, voice ${cursor.voice + 1}',
            style: Theme.of(context).textTheme.labelSmall,
          ),
        ),
        ListTile(
          dense: true,
          title: Text('Duration: ${_durationLabel(beat.duration)}'),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 4, 12, 8),
          child: Row(
            children: [
              const Text('Stroke'),
              const SizedBox(width: 12),
              Flexible(
                child: SegmentedButton<_Stroke>(
                  key: const ValueKey('inspector-stroke'),
                  showSelectedIcon: false,
                  segments: const [
                    ButtonSegment(
                      value: _Stroke.down,
                      label: Text('Down'),
                      tooltip: 'Down-stroke',
                    ),
                    ButtonSegment(
                      value: _Stroke.none,
                      label: Text('None'),
                      tooltip: 'No stroke',
                    ),
                    ButtonSegment(
                      value: _Stroke.up,
                      label: Text('Up'),
                      tooltip: 'Up-stroke',
                    ),
                  ],
                  selected: {_Stroke.of(beat.stroke)},
                  onSelectionChanged: (v) =>
                      _setStroke(ref, s, beat, v.single.dto),
                ),
              ),
            ],
          ),
        ),
        if (note != null) ...[
          const Divider(height: 1),
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 10, 12, 4),
            child: Text(
              'Note – fret ${note.fret}, string ${note.string}',
              style: Theme.of(context).textTheme.labelSmall,
            ),
          ),
          CheckboxListTile(
            dense: true,
            title: const Text('Tie'),
            value: note.tie,
            onChanged: (_) => _toggle(ref, s, beat, note, 'tie'),
          ),
          CheckboxListTile(
            dense: true,
            title: const Text('Dead note'),
            value: note.dead,
            onChanged: (_) => _toggle(ref, s, beat, note, 'dead'),
          ),
          CheckboxListTile(
            dense: true,
            title: const Text('Let ring'),
            value: note.letRing,
            onChanged: (_) => _toggle(ref, s, beat, note, 'letRing'),
          ),
          CheckboxListTile(
            dense: true,
            title: const Text('Accent'),
            value: note.accent,
            onChanged: (_) => _toggle(ref, s, beat, note, 'accent'),
          ),
          CheckboxListTile(
            dense: true,
            title: const Text('Palm mute'),
            value: note.palmMute,
            onChanged: (_) => _toggle(ref, s, beat, note, 'palmMute'),
          ),
          CheckboxListTile(
            dense: true,
            title: const Text('String-percussive'),
            subtitle: const Text('Struck string, keeps its fret'),
            value: note.stringPercussive,
            onChanged: (_) => _toggle(ref, s, beat, note, 'stringPercussive'),
          ),
        ],
      ],
    );
  }

  BeatDto? _currentBeat(EditorState s) {
    final beatIndex = s.cursor.beat;
    if (beatIndex == null) return null;
    final beats = s.service.beats(s.activeTrack, s.cursor.bar, s.cursor.voice);
    if (beatIndex >= beats.length) return null;
    return beats[beatIndex];
  }

  String _durationLabel(DurationDto d) {
    const names = [
      'whole',
      'half',
      'quarter',
      '8th',
      '16th',
      '32nd',
      '64th',
      '128th',
      '256th',
    ];
    final base = d.base < names.length ? names[d.base] : '1/${1 << d.base}';
    final dots = '.' * d.dots;
    final tuplet = d.tuplet != null
        ? ' (${d.tuplet!.actual}:${d.tuplet!.normal})'
        : '';
    return '$base$dots$tuplet';
  }

  void _toggle(
    WidgetRef ref,
    EditorState s,
    BeatDto beat,
    NoteDto note,
    String field,
  ) {
    final editor = ref.read(editorProvider.notifier);
    final address = _address(s, beat);
    if (field == 'tie') {
      editor.execute(
        CommandDto.setTie(beat: address, note: note.id, tie: !note.tie),
      );
    } else {
      editor.execute(
        CommandDto.setNoteEffects(
          beat: address,
          note: note.id,
          letRing: field == 'letRing' ? !note.letRing : note.letRing,
          dead: field == 'dead' ? !note.dead : note.dead,
          accent: field == 'accent' ? !note.accent : note.accent,
          palmMute: field == 'palmMute' ? !note.palmMute : note.palmMute,
          stringPercussive: field == 'stringPercussive'
              ? !note.stringPercussive
              : note.stringPercussive,
        ),
      );
    }
  }

  void _setStroke(WidgetRef ref, EditorState s, BeatDto beat, StrokeDto? to) {
    if (to == beat.stroke) return;
    ref
        .read(editorProvider.notifier)
        .execute(CommandDto.setBeatStroke(beat: _address(s, beat), stroke: to));
  }

  BeatAddressDto _address(EditorState s, BeatDto beat) => BeatAddressDto(
    track: s.activeTrack,
    bar: s.cursor.bar,
    voice: s.cursor.voice,
    beat: beat.id,
  );
}

/// The three stroke choices of the segmented control ("none" stands for no stroke,
/// which the DTO models as null).
enum _Stroke {
  down(StrokeDto.down),
  none(null),
  up(StrokeDto.up);

  const _Stroke(this.dto);
  final StrokeDto? dto;

  static _Stroke of(StrokeDto? s) => switch (s) {
    StrokeDto.down => down,
    StrokeDto.up => up,
    null => none,
  };
}
