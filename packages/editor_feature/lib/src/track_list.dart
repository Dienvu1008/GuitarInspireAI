import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'document_controller.dart';

/// Track list: shows every track and switches the active TAB track (Q-17).
class TrackList extends ConsumerWidget {
  const TrackList({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = ref.watch(editorProvider);
    final tracks = s.summary.tracks;
    return ListView(
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
            onTap: () => ref.read(editorProvider.notifier).selectTrack(t.id),
          ),
      ],
    );
  }
}
