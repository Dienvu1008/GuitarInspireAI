import 'dart:math' as math;

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'document_controller.dart';

/// Opens the minimal "Apply pattern" dialog (ADR-034, slice 6a-3) for the active track.
Future<void> showApplyPatternDialog(BuildContext context) => showDialog<void>(
  context: context,
  builder: (_) => const ApplyPatternDialog(),
);

/// The minimal apply dialog (ADR-034 section 7): pattern, chord and voicing, bar
/// range; then the preview, a fit choice when the core needs one, and a confirmation
/// before content is overwritten. It decides nothing about the music: the voicings
/// come from the chord finder, the preview, the fit need and the overwritten bars from
/// the core (GR-05). Applying is one `CommandDto.applyPattern`.
class ApplyPatternDialog extends ConsumerStatefulWidget {
  const ApplyPatternDialog({super.key});

  @override
  ConsumerState<ApplyPatternDialog> createState() => _ApplyPatternDialogState();
}

class _ApplyPatternDialogState extends ConsumerState<ApplyPatternDialog> {
  late final DocumentService _service;
  late final int _track;
  late final int _barCount;
  late final List<PatternInfoDto> _patterns;

  /// The chosen pattern; null when the library has none (6a-5: the library is the
  /// document's, so the sample may be gone).
  int? _pattern;
  final _chord = TextEditingController(text: 'C');
  final _first = TextEditingController();
  final _last = TextEditingController();
  ChordResultDto? _voicings;
  int _voicing = 0;
  FitDto? _fit;
  bool _needsFit = false;
  PatternReportDto? _report;

  /// The document revision the shown preview was computed at (from the core). Apply
  /// is sent with it, so a confirmation never applies to content that changed after
  /// the preview.
  int? _previewRevision;
  String? _error;

  @override
  void initState() {
    super.initState();
    final s = ref.read(editorProvider);
    _service = s.service;
    _track = s.activeTrack;
    _barCount = s.summary.masterBars.length;
    _patterns = _service.patterns();
    final sample = _service.samplePattern();
    _pattern = _patterns.any((p) => p.id == sample)
        ? sample
        : _patterns.isEmpty
        ? null
        : _patterns.first.id;
    _first.text = '${s.cursor.bar + 1}';
    _last.text = '${math.min(s.cursor.bar + 4, _barCount)}';
  }

  @override
  void dispose() {
    _chord.dispose();
    _first.dispose();
    _last.dispose();
    super.dispose();
  }

  /// Any input change invalidates the preview (and a range change the fit choice).
  void _changed({bool range = false}) => setState(() {
    _report = null;
    _previewRevision = null;
    _needsFit = false;
    _error = null;
    if (range) _fit = null;
  });

  void _findVoicings() {
    _changed();
    try {
      final v = _service.chordVoicings(_track, _chord.text.trim());
      setState(() {
        _voicings = v;
        _voicing = 0;
        if (v.voicings.isEmpty) _error = 'No voicing found for ${v.symbol}.';
      });
    } on ApiError catch (e) {
      setState(() {
        _voicings = null;
        _error = e.message;
      });
    }
  }

  /// The arguments, or null with an error shown when an input is missing or not a bar
  /// number. Bars are 1-based in the dialog and 0-based in the command.
  ApplyPatternDto? _args({bool overwrite = false}) {
    final pattern = _pattern;
    if (pattern == null) {
      setState(
        () => _error =
            'The library has no pattern. Make one in the pattern editor.',
      );
      return null;
    }
    final v = _voicings;
    if (v == null || v.voicings.isEmpty) {
      setState(() => _error = 'Find the voicings of a chord first.');
      return null;
    }
    final first = int.tryParse(_first.text.trim());
    final last = int.tryParse(_last.text.trim());
    if (first == null ||
        last == null ||
        first < 1 ||
        last < first ||
        last > _barCount) {
      setState(
        () => _error = 'Enter a first and last bar between 1 and $_barCount.',
      );
      return null;
    }
    return ApplyPatternDto(
      track: _track,
      first: first - 1,
      last: last - 1,
      pattern: pattern,
      frets: v.voicings[_voicing].frets,
      chord: v.symbol,
      fit: _fit,
      overwrite: overwrite,
    );
  }

  void _preview() {
    final a = _args();
    if (a == null) return;
    try {
      final p = _service.previewPattern(a);
      setState(() {
        _needsFit = p.needsFit;
        _report = p.report;
        _previewRevision = p.revision;
        _error = null;
      });
    } on ApiError catch (e) {
      setState(() {
        _report = null;
        _previewRevision = null;
        _error = e.message;
      });
    }
  }

  Future<void> _apply() async {
    final report = _report;
    final revision = _previewRevision;
    if (report == null || revision == null) return;
    final overwrite = report.overwritten.isNotEmpty;
    if (overwrite) {
      final bars = report.overwritten.map((b) => b + 1).join(', ');
      final yes = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: const Text('Overwrite existing content?'),
          content: Text(
            'Bars $bars already contain beats in the voices the pattern writes, '
            'including rests. Replace them?',
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              key: const ValueKey('apply-pattern-overwrite'),
              onPressed: () => Navigator.pop(context, true),
              child: const Text('Overwrite'),
            ),
          ],
        ),
      );
      if (yes != true) return;
    }
    final a = _args(overwrite: overwrite);
    if (a == null) return;
    final rejection = await ref
        .read(editorProvider.notifier)
        .executeAt(
          CommandDto.applyPattern(args: a),
          expectedRevision: revision,
        );
    if (!mounted) return;
    if (rejection == null) {
      Navigator.pop(context);
    } else if (rejection.rule == 'E-01') {
      // The score changed after the preview: the confirmation was for old content.
      // Drop the preview; the user previews and confirms again. Never resend.
      setState(() {
        _report = null;
        _previewRevision = null;
        _needsFit = false;
        _error =
            'The score changed after the preview. Preview again, and confirm the '
            'overwrite again if it is still needed.';
      });
    } else {
      setState(() => _error = rejection.message);
    }
  }

  String _summary(PatternReportDto r) {
    final parts = <String>[
      '${r.repetitions} repetition(s), ${r.notes} note(s) on bars '
          '${r.first + 1} to ${r.last + 1}',
      if (r.voices == 2) 'voices 1 and 2' else 'voice 1',
      if (r.overwritten.isNotEmpty)
        'replaces the content of bars ${r.overwritten.map((b) => b + 1).join(', ')}',
      if (r.dropped.isNotEmpty)
        '${r.dropped.length} step(s) dropped at the end',
      if (r.shortened.isNotEmpty)
        '${r.shortened.length} step(s) shortened at the end',
    ];
    return '${parts.join('; ')}.';
  }

  @override
  Widget build(BuildContext context) {
    final v = _voicings;
    final report = _report;
    return AlertDialog(
      title: const Text('Apply pattern'),
      content: SizedBox(
        width: 440,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (_patterns.isEmpty)
                const Text(
                  'The library has no pattern. Make one in the pattern editor.',
                  key: ValueKey('apply-pattern-empty'),
                )
              else
                DropdownButtonFormField<int>(
                  key: const ValueKey('apply-pattern-pattern'),
                  initialValue: _pattern,
                  decoration: const InputDecoration(labelText: 'Pattern'),
                  items: [
                    for (final p in _patterns)
                      DropdownMenuItem(value: p.id, child: Text(p.name)),
                  ],
                  onChanged: (id) {
                    if (id == null) return;
                    _pattern = id;
                    // A fit chosen for another pattern must not carry over.
                    _changed(range: true);
                  },
                ),
              const SizedBox(height: 8),
              Row(
                children: [
                  Expanded(
                    child: TextField(
                      key: const ValueKey('apply-pattern-chord'),
                      controller: _chord,
                      decoration: const InputDecoration(labelText: 'Chord'),
                      onChanged: (_) => setState(() {
                        _voicings = null;
                        _report = null;
                      }),
                      onSubmitted: (_) => _findVoicings(),
                    ),
                  ),
                  const SizedBox(width: 8),
                  FilledButton.tonal(
                    key: const ValueKey('apply-pattern-find'),
                    onPressed: _findVoicings,
                    child: const Text('Find voicings'),
                  ),
                ],
              ),
              if (v != null && v.voicings.isNotEmpty) ...[
                const SizedBox(height: 8),
                DropdownButtonFormField<int>(
                  key: const ValueKey('apply-pattern-voicing'),
                  initialValue: _voicing,
                  decoration: InputDecoration(
                    labelText: 'Voicing of ${v.symbol}',
                  ),
                  items: [
                    for (final (i, x) in v.voicings.indexed)
                      DropdownMenuItem(value: i, child: Text(x.chartText)),
                  ],
                  onChanged: (i) {
                    if (i == null) return;
                    _voicing = i;
                    _changed();
                  },
                ),
              ],
              const SizedBox(height: 8),
              Row(
                children: [
                  Expanded(
                    child: TextField(
                      key: const ValueKey('apply-pattern-first'),
                      controller: _first,
                      keyboardType: TextInputType.number,
                      decoration: const InputDecoration(labelText: 'First bar'),
                      onChanged: (_) => _changed(range: true),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: TextField(
                      key: const ValueKey('apply-pattern-last'),
                      controller: _last,
                      keyboardType: TextInputType.number,
                      decoration: const InputDecoration(labelText: 'Last bar'),
                      onChanged: (_) => _changed(range: true),
                    ),
                  ),
                ],
              ),
              if (_needsFit) ...[
                const SizedBox(height: 12),
                const Text(
                  'The range is not a whole number of repetitions. '
                  'Choose how to fit the pattern:',
                ),
                const SizedBox(height: 4),
                SegmentedButton<FitDto>(
                  key: const ValueKey('apply-pattern-fit'),
                  emptySelectionAllowed: true,
                  showSelectedIcon: false,
                  segments: const [
                    ButtonSegment(
                      value: FitDto.repeatAndCut,
                      label: Text('Repeat and cut'),
                    ),
                    ButtonSegment(
                      value: FitDto.wholeRepetitionsOnly,
                      label: Text('Whole repetitions only'),
                    ),
                  ],
                  selected: _fit == null ? <FitDto>{} : {_fit!},
                  onSelectionChanged: (s) {
                    setState(() => _fit = s.isEmpty ? null : s.first);
                    _preview();
                  },
                ),
              ],
              if (report != null) ...[
                const SizedBox(height: 12),
                Semantics(
                  liveRegion: true,
                  child: Text(
                    _summary(report),
                    key: const ValueKey('apply-pattern-summary'),
                  ),
                ),
              ],
              if (_error != null) ...[
                const SizedBox(height: 12),
                Semantics(
                  liveRegion: true,
                  child: Text(
                    _error!,
                    key: const ValueKey('apply-pattern-error'),
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        OutlinedButton(
          key: const ValueKey('apply-pattern-preview'),
          onPressed: _pattern == null ? null : _preview,
          child: const Text('Preview'),
        ),
        FilledButton(
          key: const ValueKey('apply-pattern-apply'),
          onPressed: report == null ? null : _apply,
          child: const Text('Apply'),
        ),
      ],
    );
  }
}
