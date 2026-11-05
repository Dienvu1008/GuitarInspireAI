import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'transport_controller.dart';

/// Play/Stop button and tempo-factor control (Q6, Q7). It reads the cursor bar from
/// [cursorMasterBar] so Play starts at the bar containing the cursor. No musical rules
/// here; it only sends transport commands (GR-05).
class TransportBar extends ConsumerWidget {
  const TransportBar({super.key, this.cursorMasterBar});

  /// Master bar the editor cursor is in; `null` resumes from the current position.
  final int? cursorMasterBar;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = ref.watch(transportProvider);
    final transport = ref.read(transportProvider.notifier);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      child: Row(
        children: [
          IconButton(
            key: const ValueKey('transport-play-stop'),
            tooltip: s.playing ? 'Stop' : 'Play',
            icon: Icon(s.playing ? Icons.stop : Icons.play_arrow),
            onPressed: () => transport.toggle(fromMasterBar: cursorMasterBar),
          ),
          const SizedBox(width: 16),
          const Text('Tempo'),
          IconButton(
            key: const ValueKey('tempo-down'),
            tooltip: 'Slower',
            icon: const Icon(Icons.remove),
            onPressed: s.tempoFactor > TempoFactorSteps.min
                ? () => transport.nudgeTempo(up: false)
                : null,
          ),
          SizedBox(
            width: 52,
            child: Text(
              s.tempoLabel,
              key: const ValueKey('tempo-label'),
              textAlign: TextAlign.center,
            ),
          ),
          IconButton(
            key: const ValueKey('tempo-up'),
            tooltip: 'Faster',
            icon: const Icon(Icons.add),
            onPressed: s.tempoFactor < TempoFactorSteps.max
                ? () => transport.nudgeTempo(up: true)
                : null,
          ),
          const SizedBox(width: 16),
          Text('Tick ${s.tick}', key: const ValueKey('transport-tick')),
          const Spacer(),
          if (s.failed && s.message != null)
            Flexible(
              child: Text(
                s.message!,
                key: const ValueKey('transport-message'),
                overflow: TextOverflow.ellipsis,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ),
        ],
      ),
    );
  }
}
