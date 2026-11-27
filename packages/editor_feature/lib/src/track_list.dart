import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'document_controller.dart';

/// Track list: shows every track, switches the active TAB track (Q-17) and adds
/// tracks.
class TrackList extends ConsumerWidget {
  const TrackList({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = ref.watch(editorProvider);
    final tracks = s.summary.tracks;
    return Column(
      children: [
        Expanded(
          child: ListView(
            children: [
              for (final (i, t) in tracks.indexed)
                ListTile(
                  key: ValueKey('track-${t.id}'),
                  dense: true,
                  selected: t.id == s.activeTrack,
                  leading: Text('${i + 1}'),
                  title: Text(t.name),
                  subtitle: Text(
                    '${t.instrument.tuning.reversed.map((p) => p.replaceAll(RegExp(r'-?\d+$'), '')).join()}'
                    '${t.instrument.capo > 0 ? ', capo ${t.instrument.capo}' : ''}',
                  ),
                  // ADR-031: show or hide the track's stave. The active track is
                  // always shown, so its button is disabled.
                  trailing: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      // ADR-041 section 9: standard notation above the TAB.
                      _NotationButton(
                        track: t.id,
                        name: t.name,
                        on: !s.notationOff.contains(t.id),
                      ),
                      _ShowButton(
                        track: t.id,
                        name: t.name,
                        shown: s.shownTracks.contains(t.id),
                        active: t.id == s.activeTrack,
                      ),
                    ],
                  ),
                  onTap: () =>
                      ref.read(editorProvider.notifier).selectTrack(t.id),
                ),
            ],
          ),
        ),
        // ADR-031: say so when tracks are hidden, since a long score opens with only
        // the first few shown.
        if (tracks.length > s.shownTracks.length)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
            child: Text(
              key: const ValueKey('hidden-tracks-note'),
              '${tracks.length - s.shownTracks.length} of ${tracks.length} '
              'tracks hidden. Use the eye buttons to show them.',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ),
        Padding(
          padding: const EdgeInsets.all(8),
          child: TextButton.icon(
            key: const ValueKey('add-track'),
            icon: const Icon(Icons.add),
            label: const Text('Add track'),
            onPressed: () => _addTrack(ref, s),
          ),
        ),
      ],
    );
  }

  /// Adds a track with the same instrument as the active one. The instrument is
  /// copied, not chosen here: Dart makes no instrument decisions (GR-05).
  static Future<void> _addTrack(WidgetRef ref, EditorState s) async {
    final active = s.summary.tracks.firstWhere((t) => t.id == s.activeTrack);
    await ref
        .read(editorProvider.notifier)
        .execute(
          CommandDto.addTrack(
            name: 'Track ${s.summary.tracks.length + 1}',
            instrument: active.instrument,
          ),
        );
  }
}

/// Toggle for one track's standard notation staff (ADR-041 section 9).
class _NotationButton extends ConsumerWidget {
  const _NotationButton({
    required this.track,
    required this.name,
    required this.on,
  });

  final int track;
  final String name;
  final bool on;

  @override
  Widget build(BuildContext context, WidgetRef ref) => IconButton(
    key: ValueKey('notation-track-$track'),
    tooltip: on ? 'Hide notation of $name' : 'Show notation of $name',
    iconSize: 18,
    visualDensity: VisualDensity.compact,
    isSelected: on,
    icon: const Icon(Icons.music_note_outlined),
    selectedIcon: const Icon(Icons.music_note),
    onPressed: () => ref.read(editorProvider.notifier).toggleNotation(track),
  );
}

/// Eye toggle for one track's stave in the TAB view (ADR-031).
class _ShowButton extends ConsumerWidget {
  const _ShowButton({
    required this.track,
    required this.name,
    required this.shown,
    required this.active,
  });

  final int track;
  final String name;
  final bool shown;
  final bool active;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tooltip = active
        ? 'The track being edited is always shown'
        : (shown ? 'Hide $name' : 'Show $name');
    return IconButton(
      key: ValueKey('show-track-$track'),
      tooltip: tooltip,
      iconSize: 18,
      visualDensity: VisualDensity.compact,
      icon: Icon(shown ? Icons.visibility : Icons.visibility_off),
      onPressed: active
          ? null
          : () => ref.read(editorProvider.notifier).toggleTrackShown(track),
    );
  }
}
