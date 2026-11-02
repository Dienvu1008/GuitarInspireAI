import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'document_controller.dart';

/// Inspector panel: properties of the selected beat and note (Phase 3 minimal
/// scope: tie, dead, let-ring). Musical rules are applied by the core.
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
    final cursor = s.cursor;
    final address = BeatAddressDto(
      track: s.activeTrack,
      bar: cursor.bar,
      voice: cursor.voice,
      beat: beat.id,
    );
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
        ),
      );
    }
  }
}
