import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'transport_controller.dart';

/// A mute toggle for one track (Q3: mute is session state, no solo). Shown next to each
/// track in the track list. Reflects the transport's muted set and toggles it.
class MuteButton extends ConsumerWidget {
  const MuteButton({super.key, required this.track});

  final int track;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final muted = ref.watch(
      transportProvider.select((s) => s.isMuted(track)),
    );
    final transport = ref.read(transportProvider.notifier);
    return IconButton(
      key: ValueKey('mute-$track'),
      tooltip: muted ? 'Unmute' : 'Mute',
      isSelected: muted,
      icon: Icon(muted ? Icons.volume_off : Icons.volume_up),
      onPressed: () => transport.toggleMute(track),
    );
  }
}
