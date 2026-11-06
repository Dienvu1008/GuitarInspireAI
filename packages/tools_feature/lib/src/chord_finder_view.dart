import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'chord_controller.dart';
import 'fretboard_view.dart';

/// Chord finder: a symbol box and the top voicings as fretboard diagrams (ADR-029 T6).
/// Selecting a voicing can insert it into the TAB at the cursor (T7) when insertion is
/// wired. No musical rule here (GR-05): the voicings come from the core.
class ChordFinderView extends ConsumerStatefulWidget {
  const ChordFinderView({super.key});

  @override
  ConsumerState<ChordFinderView> createState() => _ChordFinderViewState();
}

class _ChordFinderViewState extends ConsumerState<ChordFinderView> {
  final _field = TextEditingController();

  @override
  void dispose() {
    _field.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final s = ref.watch(chordProvider);
    final chord = ref.read(chordProvider.notifier);
    final canInsert = ref.watch(insertVoicingProvider) != null;
    final scheme = Theme.of(context).colorScheme;

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Text(
            'Chord finder',
            style: TextStyle(fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 12),
          TextField(
            key: const ValueKey('chord-input'),
            controller: _field,
            decoration: const InputDecoration(
              labelText: 'Chord (e.g. C, Am7, G/B)',
              isDense: true,
              border: OutlineInputBorder(),
            ),
            onSubmitted: chord.find,
            onChanged: (v) {
              if (v.trim().isEmpty) chord.find('');
            },
          ),
          const SizedBox(height: 8),
          FilledButton(
            key: const ValueKey('chord-find'),
            onPressed: () => chord.find(_field.text),
            child: const Text('Find voicings'),
          ),
          if (s.message != null) ...[
            const SizedBox(height: 8),
            Text(
              s.message!,
              key: const ValueKey('chord-message'),
              style: TextStyle(color: scheme.error),
            ),
          ],
          const SizedBox(height: 12),
          if (s.result != null)
            Expanded(
              child: _VoicingList(
                result: s.result!,
                canInsert: canInsert,
                onInsert: (v) async {
                  final ok = await chord.insert(v);
                  if (ok && context.mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(content: Text('Inserted ${v.chartText}')),
                    );
                  }
                },
              ),
            ),
        ],
      ),
    );
  }
}

class _VoicingList extends StatelessWidget {
  const _VoicingList({
    required this.result,
    required this.canInsert,
    required this.onInsert,
  });

  final ChordResultDto result;
  final bool canInsert;
  final Future<void> Function(VoicingDto) onInsert;

  @override
  Widget build(BuildContext context) {
    if (result.voicings.isEmpty) {
      return Text('No voicings for ${result.symbol}.');
    }
    return ListView.separated(
      key: const ValueKey('voicing-list'),
      itemCount: result.voicings.length,
      separatorBuilder: (_, _) => const Divider(),
      itemBuilder: (context, i) {
        final v = result.voicings[i];
        return Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            SizedBox(
              width: 120,
              height: 150,
              child: FretboardView(voicing: v),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    v.chartText,
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                  Text('${v.fingers} finger(s)'),
                  const SizedBox(height: 4),
                  OutlinedButton(
                    key: ValueKey('insert-$i'),
                    onPressed:
                        canInsert ? () => onInsert(v) : null,
                    child: const Text('Insert at cursor'),
                  ),
                ],
              ),
            ),
          ],
        );
      },
    );
  }
}
