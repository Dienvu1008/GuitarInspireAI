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
                  onTap: () =>
                      ref.read(editorProvider.notifier).selectTrack(t.id),
                ),
            ],
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
