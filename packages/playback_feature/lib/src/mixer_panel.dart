import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'transport_controller.dart';

/// One track as the Mixer panel lists it: its id and its name.
final class MixerTrack {
  const MixerTrack({required this.id, required this.name});

  final int id;
  final String name;

  @override
  bool operator ==(Object other) =>
      other is MixerTrack && other.id == id && other.name == name;

  @override
  int get hashCode => Object.hash(id, name);
}

/// The tracks the Mixer lists, in score order. The app overrides it from the editor's
/// track list; this package cannot import the editor package (FF-05).
final mixerTracksProvider = Provider<List<MixerTrack>>((ref) => const []);

/// The Mixer panel (ADR-032): per track a name, Mute (M), Solo (S), a volume slider and
/// a pan slider. It is the one place that shows mute and solo. Everything here is session
/// state: nothing is saved with the project. The panel sends integers to the core, which
/// owns the ranges, the defaults and the solo rule (GR-05).
class MixerPanel extends ConsumerWidget {
  const MixerPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tracks = ref.watch(mixerTracksProvider);
    return Semantics(
      container: true,
      label: 'Mixer',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 8, 12, 4),
            child: Text('Mixer', style: Theme.of(context).textTheme.titleSmall),
          ),
          const Divider(height: 1),
          Expanded(
            child: tracks.isEmpty
                ? const Center(child: Text('No tracks'))
                : ListView(
                    key: const ValueKey('mixer-list'),
                    children: [
                      for (final t in tracks)
                        _MixerRow(key: ValueKey('mixer-row-${t.id}'), track: t),
                    ],
                  ),
          ),
          const Divider(height: 1),
          const Padding(
            padding: EdgeInsets.fromLTRB(12, 4, 12, 8),
            child: Text(
              'Not saved with the project. Double-click a slider to reset it.',
              key: ValueKey('mixer-note'),
              style: TextStyle(fontSize: 11),
            ),
          ),
        ],
      ),
    );
  }
}

class _MixerRow extends ConsumerWidget {
  const _MixerRow({super.key, required this.track});

  final MixerTrack track;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final id = track.id;
    final muted = ref.watch(transportProvider.select((s) => s.isMuted(id)));
    final soloed = ref.watch(transportProvider.select((s) => s.isSoloed(id)));
    final mix = ref.watch(transportProvider.select((s) => s.mixOf(id)));
    final transport = ref.read(transportProvider.notifier);
    final name = track.name;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  name,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.bodyMedium,
                ),
              ),
              _ToggleButton(
                key: ValueKey('mixer-mute-$id'),
                label: 'M',
                tooltip: muted ? 'Unmute $name' : 'Mute $name',
                selected: muted,
                onPressed: () => transport.toggleMute(id),
              ),
              _ToggleButton(
                key: ValueKey('mixer-solo-$id'),
                label: 'S',
                tooltip: soloed ? 'Unsolo $name' : 'Solo $name',
                selected: soloed,
                onPressed: () => transport.toggleSolo(id),
              ),
              IconButton(
                key: ValueKey('mixer-reset-$id'),
                tooltip: 'Reset volume and pan of $name',
                iconSize: 18,
                visualDensity: VisualDensity.compact,
                icon: const Icon(Icons.restart_alt),
                onPressed: mix.isDefault ? null : () => transport.resetMix(id),
              ),
            ],
          ),
          _LabeledSlider(
            sliderKey: ValueKey('mixer-volume-$id'),
            label: 'Vol',
            semanticLabel: 'Volume of $name',
            value: mix.volume,
            min: MixRanges.volumeMin,
            max: MixRanges.volumeMax,
            onChanged: (v) =>
                transport.setTrackMix(id, volume: v, pan: mix.pan),
            onReset: () => transport.setTrackMix(
              id,
              volume: MixRanges.volumeDefault,
              pan: mix.pan,
            ),
          ),
          _LabeledSlider(
            sliderKey: ValueKey('mixer-pan-$id'),
            label: 'Pan',
            semanticLabel: 'Pan of $name',
            value: mix.pan,
            min: MixRanges.panMin,
            max: MixRanges.panMax,
            onChanged: (v) =>
                transport.setTrackMix(id, volume: mix.volume, pan: v),
            onReset: () => transport.setTrackMix(
              id,
              volume: mix.volume,
              pan: MixRanges.panDefault,
            ),
          ),
          const Divider(height: 8),
        ],
      ),
    );
  }
}

/// A small toggle with a letter (M or S). The state is shown by fill, not by colour
/// alone: a selected button is filled and announced as selected.
class _ToggleButton extends StatelessWidget {
  const _ToggleButton({
    super.key,
    required this.label,
    required this.tooltip,
    required this.selected,
    required this.onPressed,
  });

  final String label;
  final String tooltip;
  final bool selected;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) => Tooltip(
    message: tooltip,
    child: Semantics(
      button: true,
      toggled: selected,
      label: tooltip,
      excludeSemantics: true,
      child: selected
          ? FilledButton(
              style: _style,
              onPressed: onPressed,
              child: Text(label),
            )
          : OutlinedButton(
              style: _style,
              onPressed: onPressed,
              child: Text(label),
            ),
    ),
  );

  static final ButtonStyle _style = ButtonStyle(
    minimumSize: WidgetStateProperty.all(const Size(32, 28)),
    padding: WidgetStateProperty.all(EdgeInsets.zero),
    visualDensity: VisualDensity.compact,
  );
}

class _LabeledSlider extends StatelessWidget {
  const _LabeledSlider({
    required this.sliderKey,
    required this.label,
    required this.semanticLabel,
    required this.value,
    required this.min,
    required this.max,
    required this.onChanged,
    required this.onReset,
  });

  final Key sliderKey;
  final String label;
  final String semanticLabel;
  final int value;
  final int min;
  final int max;
  final ValueChanged<int> onChanged;
  final VoidCallback onReset;

  @override
  Widget build(BuildContext context) => Row(
    children: [
      SizedBox(
        width: 28,
        child: Text(label, style: const TextStyle(fontSize: 11)),
      ),
      Expanded(
        child: GestureDetector(
          onDoubleTap: onReset,
          child: Slider(
            key: sliderKey,
            value: value.toDouble(),
            min: min.toDouble(),
            max: max.toDouble(),
            divisions: max - min,
            label: '$value',
            semanticFormatterCallback: (v) => '$semanticLabel ${v.round()}',
            onChanged: (v) {
              final next = v.round();
              if (next != value) onChanged(next);
            },
          ),
        ),
      ),
      SizedBox(
        width: 30,
        child: Text(
          '$value',
          textAlign: TextAlign.end,
          style: const TextStyle(fontSize: 11),
        ),
      ),
    ],
  );
}
