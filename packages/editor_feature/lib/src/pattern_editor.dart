import 'dart:math' as math;

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'document_controller.dart';

/// Opens the pattern editor (ADR-034, slice 6a-5d): the grid and the gesture palette.
Future<void> showPatternEditor(BuildContext context) => showDialog<void>(
  context: context,
  builder: (_) => const PatternEditorDialog(),
);

/// A fraction of a whole note. Only for drawing: the times come from the core.
final class _Frac {
  const _Frac(this.n, this.d);
  final int n;
  final int d;
  int compareTo(_Frac o) => (n * o.d).compareTo(o.n * d);
  bool operator <(_Frac o) => compareTo(o) < 0;
  bool operator <=(_Frac o) => compareTo(o) <= 0;
}

/// A row of the editor: a lane band (all steps of a lane, also rests and holds), or a
/// row of the core's grid (a target or a gesture).
final class _UiRow {
  const _UiRow.lane(this.lane, this.label) : core = null;
  const _UiRow.core(int this.core, this.label) : lane = null;
  final PatternLaneDto? lane;
  final int? core;
  final String label;
}

/// The keyboard and grid cursor: a row of [_UiRow]s and a cell of a beat.
final class _Cursor {
  const _Cursor(this.row, this.beat, this.cell);
  final int row;
  final int beat;
  final int cell;
}

/// The pattern editor (ADR-034 section 9). Rows are the targets and gestures of the
/// core's grid, plus one colour band per lane; columns are the cells of each beat, with
/// a snap per beat. Every edit is one core command: the core decides values, spans,
/// order and every library rule (GR-05). The editor only maps clicks and keys to cells
/// and step indices, and draws the view model.
class PatternEditorDialog extends ConsumerStatefulWidget {
  const PatternEditorDialog({super.key});

  @override
  ConsumerState<PatternEditorDialog> createState() => _PatternEditorState();
}

class _PatternEditorState extends ConsumerState<PatternEditorDialog> {
  static const _beatWidth = 96.0;
  static const _rowHeight = 26.0;
  static const _labelWidth = 120.0;
  static const _snapChoices = [1, 2, 3, 4, 6, 8];

  final _focus = FocusNode(debugLabel: 'pattern-grid');
  int? _pattern;
  PatternLaneDto? _lane;

  /// Cells per beat, beat by beat (view state, not stored in the pattern).
  List<int> _snaps = const [];
  _Cursor _cursor = const _Cursor(0, 0, 0);

  /// The selected run of steps of [_lane], by index.
  (int, int)? _selection;
  String? _error;

  String? _viewKey;
  GridViewDto? _view;

  @override
  void initState() {
    super.initState();
    final patterns = ref.read(editorProvider).service.patterns();
    if (patterns.isNotEmpty) _pattern = patterns.first.id;
  }

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  DocumentService get _service => ref.read(editorProvider).service;

  /// The view at the current revision, pattern and snaps (cached).
  GridViewDto? _currentView(int revision) {
    final p = _pattern;
    if (p == null) return null;
    final key = '$revision|$p|${_snaps.join(',')}';
    if (key == _viewKey) return _view;
    try {
      _view = _service.patternGrid(p, _snaps);
      _viewKey = key;
      final lanes = _view!.lanes;
      if (_lane == null || !lanes.contains(_lane)) _lane = lanes.first;
    } on ApiError {
      // The pattern is gone (undo of its creation, or removed): pick another.
      final patterns = _service.patterns();
      _pattern = patterns.isEmpty ? null : patterns.first.id;
      _viewKey = null;
      _selection = null;
      return _pattern == null ? null : _currentView(revision);
    }
    return _view;
  }

  List<_UiRow> _rows(GridViewDto v) => [
    for (final l in v.lanes) _UiRow.lane(l, '${_laneName(l)} lane'),
    for (final (i, r) in v.rows.indexed) _UiRow.core(i, r.label),
  ];

  static String _laneName(PatternLaneDto l) => switch (l) {
    PatternLaneDto.strum => 'Strum',
    PatternLaneDto.thumb => 'Thumb',
    PatternLaneDto.fingers => 'Fingers',
  };

  _Frac _beat(GridViewDto v) => _Frac(v.beatNumerator, v.beatDenominator);

  /// Start and end of a cell, in whole notes.
  (_Frac, _Frac) _cellTime(GridViewDto v, int beat, int cell) {
    final parts = v.snaps[beat];
    final b = _beat(v);
    return (
      _Frac((beat * parts + cell) * b.n, parts * b.d),
      _Frac((beat * parts + cell + 1) * b.n, parts * b.d),
    );
  }

  static _Frac _onset(GridStepDto s) =>
      _Frac(s.onsetNumerator, s.onsetDenominator);
  static _Frac _end(GridStepDto s) => _Frac(s.endNumerator, s.endDenominator);

  /// The step of [lane] sounding at the start of a cell.
  GridStepDto? _stepAt(GridViewDto v, PatternLaneDto lane, int beat, int cell) {
    final (start, _) = _cellTime(v, beat, cell);
    for (final s in v.steps) {
      if (s.lane == lane && _onset(s) <= start && start < _end(s)) return s;
    }
    return null;
  }

  /// Steps shown in a cell of a row: of any lane for a lane band of that lane, or the
  /// steps that mark a core row.
  List<GridStepDto> _shown(GridViewDto v, _UiRow row, int beat, int cell) {
    final (start, end) = _cellTime(v, beat, cell);
    return [
      for (final s in v.steps)
        if (_onset(s) < end &&
            start < _end(s) &&
            (row.lane != null ? s.lane == row.lane : s.rows.contains(row.core)))
          s,
    ];
  }

  bool _selected(GridStepDto s) {
    final sel = _selection;
    return sel != null &&
        s.lane == _lane &&
        s.index >= sel.$1 &&
        s.index <= sel.$2;
  }

  // -------------------------------------------------------------- commands

  /// Sends one command at the current revision; shows a rejection.
  Future<bool> _send(CommandDto c) async {
    final s = ref.read(editorProvider);
    final rejection = await ref
        .read(editorProvider.notifier)
        .executeAt(c, expectedRevision: s.summary.revision);
    if (!mounted) return false;
    setState(() => _error = rejection?.message);
    return rejection == null;
  }

  Future<bool> _edit(GridEditDto e) {
    final p = _pattern;
    if (p == null) return Future.value(false);
    return _send(CommandDto.editPattern(pattern: p, edit: e));
  }

  GridCellDto _cellDto(GridViewDto v, int beat, int cell) =>
      GridCellDto(beat: beat, parts: v.snaps[beat], cell: cell);

  static const _noTechniques = TechniquesDto(
    accent: false,
    palmMute: false,
    dead: false,
    letRing: false,
    stringPercussive: false,
  );

  /// The action a click on a core row places: a strike on its target, or its gesture.
  GridActionDto? _actionFor(GridViewDto v, int coreRow) {
    final r = v.rows[coreRow];
    final g = r.gesture;
    if (g != null) return GridActionDto.gesture(gesture: g);
    final sel = r.selector;
    if (sel == null) return null;
    return GridActionDto.strike(
      strike: GridStrikeDto(targets: [sel], techniques: _noTechniques),
    );
  }

  /// A click, or Enter at the cursor: select the step there, toggle a target of it
  /// (with [toggle]), or place a new step.
  Future<void> _activate(
    GridViewDto v,
    _Cursor at, {
    bool toggle = false,
    bool extend = false,
  }) async {
    final rows = _rows(v);
    final row = rows[at.row];
    final lane = row.lane ?? _lane!;
    setState(() {
      _cursor = at;
      if (row.lane != null) _lane = row.lane;
    });
    final step = _stepAt(v, lane, at.beat, at.cell);
    if (step != null) {
      final sel = _selection;
      setState(() {
        _selection = extend && sel != null
            ? (
                sel.$1 < step.index ? sel.$1 : step.index,
                sel.$2 > step.index ? sel.$2 : step.index,
              )
            : (step.index, step.index);
      });
      final selector = row.core == null ? null : v.rows[row.core!].selector;
      if (toggle && selector != null && step.kind == GridStepKindDto.strike) {
        await _edit(
          GridEditDto.toggleTarget(
            lane: lane,
            index: step.index,
            selector: selector,
          ),
        );
      }
      return;
    }
    final core = row.core;
    if (core == null) return;
    final action = _actionFor(v, core);
    if (action == null) return;
    await _place(v, lane, at, action);
  }

  Future<void> _place(
    GridViewDto v,
    PatternLaneDto lane,
    _Cursor at,
    GridActionDto action,
  ) async {
    final ok = await _edit(
      GridEditDto.place(
        lane: lane,
        at: _cellDto(v, at.beat, at.cell),
        cells: 1,
        action: action,
      ),
    );
    if (ok) {
      // Select the new step: it is the step of the lane at this cell now.
      final nv = _currentView(ref.read(editorProvider).summary.revision);
      final s = nv == null ? null : _stepAt(nv, lane, at.beat, at.cell);
      if (s != null) setState(() => _selection = (s.index, s.index));
    }
  }

  GridStepDto? _selectedStep(GridViewDto v) {
    final sel = _selection;
    if (sel == null) return null;
    for (final s in v.steps) {
      if (s.lane == _lane && s.index == sel.$1) return s;
    }
    return null;
  }

  Future<void> _delete() async {
    final sel = _selection;
    final lane = _lane;
    if (sel == null || lane == null) return;
    if (await _edit(
      GridEditDto.delete(lane: lane, first: sel.$1, last: sel.$2),
    )) {
      setState(() => _selection = null);
    }
  }

  /// The length of [s] in cells of the snap of its start beat, if it is whole.
  int? _cellsOf(GridViewDto v, GridStepDto s) {
    final cell = s.cell;
    if (cell == null) return null;
    final b = _beat(v);
    // (end - onset) / (beat / parts)
    final lenN =
        s.endNumerator * s.onsetDenominator -
        s.onsetNumerator * s.endDenominator;
    final lenD = s.endDenominator * s.onsetDenominator;
    final num = lenN * b.d * cell.parts;
    final den = lenD * b.n;
    return num % den == 0 ? num ~/ den : null;
  }

  Future<void> _resize(GridViewDto v, int delta) async {
    final s = _selectedStep(v);
    if (s == null) return;
    final cells = _cellsOf(v, s);
    final cell = s.cell;
    if (cells == null || cell == null) {
      setState(() => _error = 'This step does not start on the grid.');
      return;
    }
    final next = cells + delta;
    if (next < 1) return;
    await _edit(
      GridEditDto.resize(
        lane: s.lane,
        index: s.index,
        parts: cell.parts,
        cells: next,
      ),
    );
  }

  Future<void> _move(GridViewDto v, int delta) async {
    final s = _selectedStep(v);
    if (s == null) return;
    final c = s.cell;
    if (c == null) {
      setState(() => _error = 'This step does not start on the grid.');
      return;
    }
    var beat = c.beat;
    var cell = c.cell + delta;
    if (cell >= c.parts) {
      beat++;
      cell = 0;
    } else if (cell < 0) {
      beat--;
      if (beat < 0) return;
      cell = v.snaps[beat] - 1;
    }
    if (beat >= v.snaps.length) return;
    if (await _edit(
      GridEditDto.move(
        lane: s.lane,
        index: s.index,
        to: _cellDto(v, beat, cell),
      ),
    )) {
      setState(() => _cursor = _Cursor(_cursor.row, beat, cell));
    }
  }

  /// A technique on one step is its flag (toggled); on a run it is a span.
  Future<void> _technique(GridViewDto v, TechniqueKindDto t) async {
    final sel = _selection;
    final lane = _lane;
    if (sel == null || lane == null) return;
    if (sel.$1 != sel.$2) {
      await _edit(
        GridEditDto.addSpan(
          lane: lane,
          first: sel.$1,
          last: sel.$2,
          technique: t,
        ),
      );
      return;
    }
    final s = _selectedStep(v);
    if (s == null) return;
    final on = !switch (t) {
      TechniqueKindDto.accent => s.techniques.accent,
      TechniqueKindDto.palmMute => s.techniques.palmMute,
      TechniqueKindDto.dead => s.techniques.dead,
      TechniqueKindDto.letRing => s.techniques.letRing,
      TechniqueKindDto.stringPercussive => s.techniques.stringPercussive,
    };
    await _edit(
      GridEditDto.setTechnique(
        lane: lane,
        index: s.index,
        technique: t,
        on_: on,
      ),
    );
  }

  Future<void> _direction(StrokeDto? d) async {
    final sel = _selection;
    final lane = _lane;
    if (sel == null || lane == null) return;
    await _edit(
      GridEditDto.setDirection(
        lane: lane,
        first: sel.$1,
        last: sel.$2,
        direction: d,
      ),
    );
  }

  Future<void> _placeAt(GridViewDto v, _Cursor at, GridActionDto action) async {
    final lane = _lane;
    if (lane == null) return;
    await _place(v, lane, at, action);
  }

  /// Places [action] at the cursor as it is now, when its turn comes.
  void _atCursor(GridActionDto action) {
    final at = _cursor;
    _run((v) => _placeAt(v, at, action));
  }

  /// Actions run one at a time, each on the view of the revision it starts at, as the
  /// shell's key handler does (Q-15): core commands complete on a worker thread, so a
  /// second key must not be sent with the revision the first one is about to change.
  Future<void> _queue = Future.value();

  void _run(Future<void> Function(GridViewDto v) action) {
    _queue = _queue
        .then((_) async {
          if (!mounted) return;
          final v = _currentView(ref.read(editorProvider).summary.revision);
          if (v != null) await action(v);
        })
        .catchError((Object _) {});
  }

  Future<String?> _askName(String title, {String initial = ''}) =>
      showDialog<String>(
        context: context,
        builder: (_) => _NameDialog(title: title, initial: initial),
      );
  Future<void> _gestureFromStep(GridViewDto v) async {
    final s = _selectedStep(v);
    final p = _pattern;
    if (s == null || p == null) return;
    final name = await _askName('Gesture from the selected strike');
    if (name == null || name.isEmpty || !mounted) return;
    await _send(
      CommandDto.gestureFromStep(
        pattern: p,
        lane: s.lane,
        index: s.index,
        name: name,
      ),
    );
  }

  Future<void> _renameGesture(GestureInfoDto g) async {
    final name = await _askName('Rename gesture', initial: g.name);
    if (name == null || name.isEmpty || !mounted) return;
    await _send(CommandDto.renameGesture(gesture: g.id, name: name));
  }

  Future<void> _newPattern() async {
    final result = await showDialog<CommandDto>(
      context: context,
      builder: (_) => const _NewPatternDialog(),
    );
    if (result == null || !mounted) return;
    final (out, rejection) = await ref
        .read(editorProvider.notifier)
        .executeForOutcome(
          result,
          expectedRevision: ref.read(editorProvider).summary.revision,
        );
    if (!mounted) return;
    if (out == null) {
      setState(() => _error = rejection?.message);
      return;
    }
    setState(() {
      _pattern = out.createdPattern;
      _lane = null;
      _snaps = const [];
      _selection = null;
      _cursor = const _Cursor(0, 0, 0);
      _error = null;
    });
  }

  Future<void> _removePattern() async {
    final p = _pattern;
    if (p == null) return;
    if (await _send(CommandDto.removePattern(pattern: p))) {
      final patterns = _service.patterns();
      setState(() {
        _pattern = patterns.isEmpty ? null : patterns.first.id;
        _selection = null;
        _lane = null;
      });
    }
  }

  // --------------------------------------------------------------- keyboard

  KeyEventResult _onKey(GridViewDto v, KeyEvent e) {
    if (e is! KeyDownEvent && e is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }
    final k = e.logicalKey;
    final keys = HardwareKeyboard.instance;
    final rows = _rows(v);
    void moveCursor(int dRow, int dCell) {
      var row = (_cursor.row + dRow).clamp(0, rows.length - 1);
      var beat = _cursor.beat;
      var cell = _cursor.cell + dCell;
      if (cell >= v.snaps[beat]) {
        if (beat + 1 < v.snaps.length) {
          beat++;
          cell = 0;
        } else {
          cell = v.snaps[beat] - 1;
        }
      } else if (cell < 0) {
        if (beat > 0) {
          beat--;
          cell = v.snaps[beat] - 1;
        } else {
          cell = 0;
        }
      }
      setState(() => _cursor = _Cursor(row, beat, cell));
    }

    if (keys.isControlPressed) {
      if (k == LogicalKeyboardKey.keyZ) {
        _run((_) => ref.read(editorProvider.notifier).undo());
        return KeyEventResult.handled;
      }
      if (k == LogicalKeyboardKey.keyY) {
        _run((_) => ref.read(editorProvider.notifier).redo());
        return KeyEventResult.handled;
      }
      if (k == LogicalKeyboardKey.enter || k == LogicalKeyboardKey.space) {
        final at = _cursor;
        _run((v) => _activate(v, at, toggle: true));
        return KeyEventResult.handled;
      }
      return KeyEventResult.ignored;
    }
    if (keys.isAltPressed) {
      if (k == LogicalKeyboardKey.arrowRight) {
        _run((v) => _move(v, 1));
        return KeyEventResult.handled;
      }
      if (k == LogicalKeyboardKey.arrowLeft) {
        _run((v) => _move(v, -1));
        return KeyEventResult.handled;
      }
      return KeyEventResult.ignored;
    }
    switch (k) {
      case LogicalKeyboardKey.arrowUp:
        moveCursor(-1, 0);
      case LogicalKeyboardKey.arrowDown:
        moveCursor(1, 0);
      case LogicalKeyboardKey.arrowLeft:
        moveCursor(0, -1);
        final at = _cursor;
        if (keys.isShiftPressed) _run((v) => _activate(v, at, extend: true));
      case LogicalKeyboardKey.arrowRight:
        moveCursor(0, 1);
        final at = _cursor;
        if (keys.isShiftPressed) _run((v) => _activate(v, at, extend: true));
      case LogicalKeyboardKey.enter || LogicalKeyboardKey.space:
        final at = _cursor;
        _run((v) => _activate(v, at));
      case LogicalKeyboardKey.delete || LogicalKeyboardKey.backspace:
        _run((_) => _delete());
      case LogicalKeyboardKey.bracketRight || LogicalKeyboardKey.equal:
        _run((v) => _resize(v, 1));
      case LogicalKeyboardKey.bracketLeft || LogicalKeyboardKey.minus:
        _run((v) => _resize(v, -1));
      case LogicalKeyboardKey.keyA:
        _run((v) => _technique(v, TechniqueKindDto.accent));
      case LogicalKeyboardKey.keyP:
        _run((v) => _technique(v, TechniqueKindDto.palmMute));
      case LogicalKeyboardKey.keyX:
        _run((v) => _technique(v, TechniqueKindDto.dead));
      case LogicalKeyboardKey.keyL:
        _run((v) => _technique(v, TechniqueKindDto.letRing));
      case LogicalKeyboardKey.keyS:
        _run((v) => _technique(v, TechniqueKindDto.stringPercussive));
      case LogicalKeyboardKey.keyD:
        _run((_) => _direction(StrokeDto.down));
      case LogicalKeyboardKey.keyU:
        _run((_) => _direction(StrokeDto.up));
      case LogicalKeyboardKey.keyN:
        _run((_) => _direction(null));
      case LogicalKeyboardKey.keyR:
        final at = _cursor;
        _run((v) => _placeAt(v, at, const GridActionDto.rest()));
      case LogicalKeyboardKey.keyH:
        final at = _cursor;
        _run((v) => _placeAt(v, at, const GridActionDto.hold()));
      case LogicalKeyboardKey.keyG:
        _run(_gestureFromStep);
      default:
        return KeyEventResult.ignored;
    }
    return KeyEventResult.handled;
  }

  // ------------------------------------------------------------------ view

  Color _laneColor(ColorScheme c, PatternLaneDto l) => switch (l) {
    PatternLaneDto.strum => c.primaryContainer,
    PatternLaneDto.thumb => c.tertiaryContainer,
    PatternLaneDto.fingers => c.secondaryContainer,
  };

  static String _marks(GridStepDto s) {
    final t = s.techniques;
    final u = s.spanTechniques;
    return [
      if (s.direction == StrokeDto.down) '↓',
      if (s.direction == StrokeDto.up) '↑',
      if (t.accent || u.accent) '>',
      if (t.palmMute || u.palmMute) 'PM',
      if (t.dead || u.dead) 'x',
      if (t.letRing || u.letRing) '~',
      if (t.stringPercussive || u.stringPercussive) 'S',
    ].join(' ');
  }

  static String _kindWord(GridStepDto s) => switch (s.kind) {
    GridStepKindDto.strike => 'strike',
    GridStepKindDto.gesture => 'gesture',
    GridStepKindDto.hold => 'hold',
    GridStepKindDto.rest => 'rest',
  };

  Widget _cell(GridViewDto v, List<_UiRow> rows, int r, int beat, int cell) {
    final colors = Theme.of(context).colorScheme;
    final row = rows[r];
    final shown = _shown(v, row, beat, cell);
    final step = shown.isEmpty ? null : shown.first;
    final starts =
        step != null &&
        _onset(step).compareTo(_cellTime(v, beat, cell).$1) == 0;
    final isCursor =
        _cursor.row == r && _cursor.beat == beat && _cursor.cell == cell;
    final selected = step != null && _selected(step);
    final parts = v.snaps[beat];
    final text = step == null
        ? ''
        : !starts
        ? ''
        : row.lane != null
        ? switch (step.kind) {
            GridStepKindDto.rest => '-',
            GridStepKindDto.hold => '_',
            _ => _marks(step).isEmpty ? '•' : _marks(step),
          }
        : '●';
    final what = step == null
        ? 'empty'
        : '${_laneName(step.lane)} ${_kindWord(step)} ${step.index + 1}'
              '${_marks(step).isEmpty ? '' : ', ${_marks(step)}'}'
              '${selected ? ', selected' : ''}';
    return Semantics(
      label:
          '${row.label}, beat ${beat + 1}, cell ${cell + 1} of $parts: $what',
      button: true,
      selected: isCursor,
      excludeSemantics: true,
      child: GestureDetector(
        key: ValueKey('pattern-cell-$r-$beat-$cell'),
        behavior: HitTestBehavior.opaque,
        onTap: () {
          _focus.requestFocus();
          final keys = HardwareKeyboard.instance;
          final at = _Cursor(r, beat, cell);
          final toggle = keys.isControlPressed;
          final extend = keys.isShiftPressed;
          _run((v) => _activate(v, at, toggle: toggle, extend: extend));
        },
        child: Container(
          width: _beatWidth / parts,
          height: _rowHeight,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            color: step == null
                ? null
                : selected
                ? colors.primary.withValues(alpha: 0.45)
                : _laneColor(colors, step.lane),
            border: Border(
              left: BorderSide(
                color: cell == 0 ? colors.outline : colors.outlineVariant,
                width: cell == 0 ? 1.2 : 0.5,
              ),
              bottom: BorderSide(color: colors.outlineVariant, width: 0.5),
            ),
            boxShadow: isCursor
                ? [BoxShadow(color: colors.primary, spreadRadius: 1.5)]
                : null,
          ),
          child: Text(
            text,
            overflow: TextOverflow.clip,
            maxLines: 1,
            style: const TextStyle(fontSize: 11),
          ),
        ),
      ),
    );
  }

  Widget _snapHeader(GridViewDto v) => Row(
    children: [
      const SizedBox(width: _labelWidth),
      for (var b = 0; b < v.snaps.length; b++)
        SizedBox(
          width: _beatWidth,
          child: DropdownButton<int>(
            key: ValueKey('pattern-snap-$b'),
            isDense: true,
            value: v.snaps[b],
            underline: const SizedBox.shrink(),
            items: [
              for (final s in _snapChoices)
                DropdownMenuItem(
                  value: s,
                  child: Text(
                    '${b + 1}: $s',
                    semanticsLabel: 'Beat ${b + 1}, $s cells',
                  ),
                ),
            ],
            onChanged: (s) {
              if (s == null) return;
              setState(() {
                final next = List.of(v.snaps);
                next[b] = s;
                _snaps = next;
                _cursor = _Cursor(_cursor.row, _cursor.beat, 0);
              });
            },
          ),
        ),
    ],
  );

  Widget _grid(GridViewDto v) {
    final rows = _rows(v);
    final colors = Theme.of(context).colorScheme;
    return Focus(
      focusNode: _focus,
      autofocus: true,
      onKeyEvent: (_, e) => _onKey(v, e),
      child: Semantics(
        label: 'Pattern grid of ${v.name}',
        container: true,
        child: SingleChildScrollView(
          key: const ValueKey('pattern-editor-grid'),
          scrollDirection: Axis.horizontal,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _snapHeader(v),
              for (var r = 0; r < rows.length; r++)
                Row(
                  children: [
                    Container(
                      width: _labelWidth,
                      height: _rowHeight,
                      padding: const EdgeInsets.only(right: 6),
                      alignment: Alignment.centerRight,
                      color: rows[r].lane == null
                          ? null
                          : _laneColor(
                              colors,
                              rows[r].lane!,
                            ).withValues(alpha: 0.5),
                      child: Text(
                        rows[r].label,
                        style: TextStyle(
                          fontWeight:
                              rows[r].lane == _lane && rows[r].lane != null
                              ? FontWeight.w600
                              : FontWeight.normal,
                        ),
                      ),
                    ),
                    for (var b = 0; b < v.snaps.length; b++)
                      for (var c = 0; c < v.snaps[b]; c++)
                        _cell(v, rows, r, b, c),
                  ],
                ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _palette(GridViewDto v) {
    final hasSelection = _selection != null;
    final gestures = _service.gestures();
    Widget tool(String key, String label, VoidCallback onPressed) =>
        OutlinedButton(
          key: ValueKey('pattern-tool-$key'),
          onPressed: hasSelection ? onPressed : null,
          child: Text(label),
        );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Wrap(
          spacing: 6,
          runSpacing: 6,
          children: [
            tool(
              'accent',
              'Accent (A)',
              () => _run((v) => _technique(v, TechniqueKindDto.accent)),
            ),
            tool(
              'palm-mute',
              'Palm mute (P)',
              () => _run((v) => _technique(v, TechniqueKindDto.palmMute)),
            ),
            tool(
              'dead',
              'Dead (X)',
              () => _run((v) => _technique(v, TechniqueKindDto.dead)),
            ),
            tool(
              'let-ring',
              'Let ring (L)',
              () => _run((v) => _technique(v, TechniqueKindDto.letRing)),
            ),
            tool(
              'string-percussive',
              'String-percussive (S)',
              () =>
                  _run((v) => _technique(v, TechniqueKindDto.stringPercussive)),
            ),
            tool(
              'down',
              'Down (D)',
              () => _run((_) => _direction(StrokeDto.down)),
            ),
            tool('up', 'Up (U)', () => _run((_) => _direction(StrokeDto.up))),
            tool(
              'no-direction',
              'No direction (N)',
              () => _run((_) => _direction(null)),
            ),
            tool('delete', 'Delete (Del)', () => _run((_) => _delete())),
            tool(
              'earlier',
              'Earlier (Alt+Left)',
              () => _run((v) => _move(v, -1)),
            ),
            tool('later', 'Later (Alt+Right)', () => _run((v) => _move(v, 1))),
            tool('shorter', 'Shorter ([)', () => _run((v) => _resize(v, -1))),
            tool('longer', 'Longer (])', () => _run((v) => _resize(v, 1))),
            tool(
              'gesture',
              'Gesture from strike (G)',
              () => _run(_gestureFromStep),
            ),
            OutlinedButton(
              key: const ValueKey('pattern-tool-rest'),
              onPressed: () => _atCursor(const GridActionDto.rest()),
              child: const Text('Rest at cursor (R)'),
            ),
            OutlinedButton(
              key: const ValueKey('pattern-tool-hold'),
              onPressed: () => _atCursor(const GridActionDto.hold()),
              child: const Text('Hold at cursor (H)'),
            ),
          ],
        ),
        const SizedBox(height: 8),
        Text('Gestures', style: Theme.of(context).textTheme.titleSmall),
        if (gestures.isEmpty)
          const Text('No gestures yet: select a strike and make one (G).')
        else
          Wrap(
            spacing: 6,
            children: [
              for (final g in gestures)
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    ActionChip(
                      key: ValueKey('pattern-gesture-${g.id}'),
                      label: Text(g.name),
                      tooltip: 'Place ${g.name} at the cursor',
                      onPressed: () =>
                          _atCursor(GridActionDto.gesture(gesture: g.id)),
                    ),
                    IconButton(
                      key: ValueKey('pattern-gesture-rename-${g.id}'),
                      tooltip: 'Rename ${g.name}',
                      onPressed: () => _run((_) => _renameGesture(g)),
                      icon: const Icon(Icons.edit, size: 16),
                    ),
                    IconButton(
                      key: ValueKey('pattern-gesture-remove-${g.id}'),
                      tooltip: 'Remove ${g.name}',
                      onPressed: () =>
                          _send(CommandDto.removeGesture(gesture: g.id)),
                      icon: const Icon(Icons.close, size: 16),
                    ),
                  ],
                ),
            ],
          ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final revision = ref.watch(
      editorProvider.select((s) => s.summary.revision),
    );
    final v = _currentView(revision);
    final patterns = _service.patterns();
    return Dialog(
      key: const ValueKey('pattern-editor'),
      // At most 1040 by 680, and never larger than the window allows.
      child: SizedBox(
        width: math.min(1040, MediaQuery.sizeOf(context).width - 80),
        height: math.min(680, MediaQuery.sizeOf(context).height - 80),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Wrap(
                spacing: 8,
                runSpacing: 8,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  Text(
                    'Pattern editor',
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                  if (patterns.isNotEmpty && _pattern != null)
                    DropdownButton<int>(
                      key: const ValueKey('pattern-editor-pattern'),
                      value: _pattern,
                      items: [
                        for (final p in patterns)
                          DropdownMenuItem(value: p.id, child: Text(p.name)),
                      ],
                      onChanged: (id) => setState(() {
                        _pattern = id;
                        _lane = null;
                        _snaps = const [];
                        _selection = null;
                        _cursor = const _Cursor(0, 0, 0);
                      }),
                    ),
                  FilledButton.tonal(
                    key: const ValueKey('pattern-editor-new'),
                    onPressed: _newPattern,
                    child: const Text('New pattern...'),
                  ),
                  OutlinedButton(
                    key: const ValueKey('pattern-editor-delete'),
                    onPressed: _pattern == null ? null : _removePattern,
                    child: const Text('Delete pattern'),
                  ),
                  if (v != null && v.lanes.length > 1)
                    SegmentedButton<PatternLaneDto>(
                      key: const ValueKey('pattern-editor-lane'),
                      showSelectedIcon: false,
                      segments: [
                        for (final l in v.lanes)
                          ButtonSegment(value: l, label: Text(_laneName(l))),
                      ],
                      selected: _lane == null ? {} : {_lane!},
                      onSelectionChanged: (s) => setState(() {
                        _lane = s.first;
                        _selection = null;
                      }),
                    ),
                ],
              ),
              const SizedBox(height: 8),
              Expanded(
                child: v == null
                    ? const Center(
                        child: Text('The library has no pattern. Make one.'),
                      )
                    : SingleChildScrollView(child: _grid(v)),
              ),
              if (v != null) ...[const Divider(), _palette(v)],
              if (_error != null) ...[
                const SizedBox(height: 8),
                Semantics(
                  liveRegion: true,
                  child: Text(
                    _error!,
                    key: const ValueKey('pattern-editor-error'),
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ),
              ],
              Align(
                alignment: Alignment.centerRight,
                child: TextButton(
                  key: const ValueKey('pattern-editor-close'),
                  onPressed: () => Navigator.pop(context),
                  child: const Text('Close'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Asks for a new pattern: name, lanes, length in bars and meter. It only collects
/// values; the core checks them.
class _NewPatternDialog extends StatefulWidget {
  const _NewPatternDialog();

  @override
  State<_NewPatternDialog> createState() => _NewPatternDialogState();
}

class _NewPatternDialogState extends State<_NewPatternDialog> {
  final _name = TextEditingController(text: 'New pattern');
  final _bars = TextEditingController(text: '1');
  final _meter = TextEditingController(text: '4/4');
  bool _twoLanes = false;
  String? _error;

  @override
  void dispose() {
    _name.dispose();
    _bars.dispose();
    _meter.dispose();
    super.dispose();
  }

  void _ok() {
    final bars = int.tryParse(_bars.text.trim());
    final m = _meter.text.trim().split('/');
    final n = m.length == 2 ? int.tryParse(m[0]) : null;
    final d = m.length == 2 ? int.tryParse(m[1]) : null;
    if (bars == null || bars < 1 || n == null || d == null || n < 1 || d < 1) {
      setState(() => _error = 'Enter a number of bars and a meter like 4/4.');
      return;
    }
    Navigator.pop(
      context,
      CommandDto.newPattern(
        name: _name.text.trim(),
        twoLanes: _twoLanes,
        lengthNumerator: bars * n,
        lengthDenominator: d,
        meter: MeterDto(numerator: n, denominator: d),
      ),
    );
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('New pattern'),
    content: SizedBox(
      width: 320,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(
            key: const ValueKey('new-pattern-name'),
            controller: _name,
            decoration: const InputDecoration(labelText: 'Name'),
          ),
          TextField(
            key: const ValueKey('new-pattern-bars'),
            controller: _bars,
            keyboardType: TextInputType.number,
            decoration: const InputDecoration(labelText: 'Bars'),
          ),
          TextField(
            key: const ValueKey('new-pattern-meter'),
            controller: _meter,
            decoration: const InputDecoration(labelText: 'Meter'),
          ),
          SwitchListTile(
            key: const ValueKey('new-pattern-two-lanes'),
            title: const Text('Thumb and fingers (two lanes)'),
            value: _twoLanes,
            onChanged: (v) => setState(() => _twoLanes = v),
          ),
          if (_error != null)
            Semantics(
              liveRegion: true,
              child: Text(
                _error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ),
        ],
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const ValueKey('new-pattern-ok'),
        onPressed: _ok,
        child: const Text('Create'),
      ),
    ],
  );
}

/// Asks for a name (a gesture). It owns its text controller, so the controller lives
/// as long as the dialog, also while it closes.
class _NameDialog extends StatefulWidget {
  const _NameDialog({required this.title, required this.initial});

  final String title;
  final String initial;

  @override
  State<_NameDialog> createState() => _NameDialogState();
}

class _NameDialogState extends State<_NameDialog> {
  late final _controller = TextEditingController(text: widget.initial);

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(widget.title),
    content: TextField(
      key: const ValueKey('pattern-name-input'),
      controller: _controller,
      autofocus: true,
      decoration: const InputDecoration(labelText: 'Name'),
      onSubmitted: (s) => Navigator.pop(context, s.trim()),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const ValueKey('pattern-name-ok'),
        onPressed: () => Navigator.pop(context, _controller.text.trim()),
        child: const Text('OK'),
      ),
    ],
  );
}
