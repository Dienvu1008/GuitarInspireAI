import 'dart:math' as math;
import 'dart:typed_data';

import 'generated/api/pattern.dart';
import 'generated/api/pattern_grid.dart';
import 'generated/api/types.dart';

/// The pattern library of `FakeDocumentService` (ADR-034, 6a-5): a placeholder for
/// widget tests. It keeps patterns, steps, spans and gestures as they are sent and
/// decides no music: every step is shown as an eighth (the core computes the real
/// value of a cell), holds are not checked against their strike, and the only
/// refusals are an unknown pattern, gesture, lane or step, a snap that is not 1, 2,
/// 3, 4, 6 or 8 (`PE-14`), two steps starting together in a lane, a duplicate
/// gesture name and the removal of a gesture in use (`PT`). The contract suite checks
/// the behaviour both libraries share.
final class FakePatternLibrary {
  FakePatternLibrary._(
    this._patterns,
    this._gestures,
    this._nextId, [
    this._generation = 0,
  ]);

  /// The built-in sample, as in the core: one 4/4 bar, one strum lane.
  factory FakePatternLibrary.sample() => FakePatternLibrary._(
    [
      _Pattern(
        id: sampleId,
        name: 'Boom-chick',
        description: 'Sample',
        twoLanes: false,
        lengthNumerator: 1,
        lengthDenominator: 1,
        meter: null,
      ),
    ],
    [],
    2,
  );

  static const sampleId = 1;
  static const _snaps = [1, 2, 3, 4, 6, 8];

  final List<_Pattern> _patterns;
  final List<GestureInfoDto> _gestures;
  int _nextId;

  /// Bumped by every change; copies keep it, so undo back to a saved copy is clean.
  int _generation;

  FakePatternLibrary copy() => FakePatternLibrary._(
    [for (final p in _patterns) p.copy()],
    List.of(_gestures),
    _nextId,
    _generation,
  );

  /// Equal for an unchanged library (for the fake's dirty check).
  int get generation => _generation;

  bool get sampleUnchanged => _patterns.any(
    (p) => p.id == sampleId && p.name == 'Boom-chick' && p.steps.isEmpty,
  );

  bool contains(int id) => _patterns.any((p) => p.id == id);

  List<PatternInfoDto> infos() => [
    for (final p in _patterns)
      PatternInfoDto(
        id: p.id,
        name: p.name,
        description: p.description,
        lengthNumerator: p.lengthNumerator,
        lengthDenominator: p.lengthDenominator,
        twoLanes: p.twoLanes,
      ),
  ];

  List<GestureInfoDto> gestures() => List.unmodifiable(_gestures);

  static Never _refuse(String rule, String message) =>
      throw ApiError(rule: rule, message: message);

  _Pattern _pattern(int id) => _patterns.firstWhere(
    (p) => p.id == id,
    orElse: () => _refuse('PT', 'no pattern $id'),
  );

  int create(
    String name,
    bool twoLanes,
    int numerator,
    int denominator,
    MeterDto? meter,
  ) {
    _generation++;
    final id = _nextId++;
    _patterns.add(
      _Pattern(
        id: id,
        name: name,
        description: '',
        twoLanes: twoLanes,
        lengthNumerator: numerator,
        lengthDenominator: denominator,
        meter: meter,
      ),
    );
    return id;
  }

  void remove(int id) {
    _patterns.remove(_pattern(id));
    _generation++;
  }

  int defineGesture(String name, GridStrikeDto strike) {
    if (_gestures.any((g) => g.name.toLowerCase() == name.toLowerCase())) {
      _refuse('PT', 'a gesture is already named $name');
    }
    _generation++;
    final id = _nextId++;
    _gestures.add(GestureInfoDto(id: id, name: name, strike: strike));
    return id;
  }

  int _gestureIndex(int id) {
    final i = _gestures.indexWhere((g) => g.id == id);
    if (i < 0) _refuse('PT', 'no gesture $id');
    return i;
  }

  void renameGesture(int id, String name) {
    final i = _gestureIndex(id);
    _generation++;
    _gestures[i] = GestureInfoDto(
      id: id,
      name: name,
      strike: _gestures[i].strike,
    );
  }

  void changeGesture(int id, GridStrikeDto strike) {
    final i = _gestureIndex(id);
    _generation++;
    _gestures[i] = GestureInfoDto(
      id: id,
      name: _gestures[i].name,
      strike: strike,
    );
  }

  void removeGesture(int id) {
    final i = _gestureIndex(id);
    final used = _patterns.any(
      (p) => p.steps.any(
        (s) => s.action is GridActionDto_Gesture &&
            (s.action as GridActionDto_Gesture).gesture == id,
      ),
    );
    if (used) _refuse('PT', 'gesture $id is used');
    _generation++;
    _gestures.removeAt(i);
  }

  int gestureFromStep(
    int pattern,
    PatternLaneDto lane,
    int index,
    String name,
  ) {
    final p = _pattern(pattern);
    final steps = p.lane(lane);
    if (index < 0 || index >= steps.length) {
      _refuse('PE-14', 'no step $index');
    }
    final action = steps[index].action;
    if (action is! GridActionDto_Strike) _refuse('PE-14', 'not a strike');
    final id = defineGesture(name, action.strike);
    steps[index].action = GridActionDto.gesture(gesture: id);
    return id;
  }

  static void _checkSnap(int parts) {
    if (!_snaps.contains(parts)) {
      _refuse('PE-14', 'a beat has 1, 2, 3, 4, 6 or 8 cells, not $parts');
    }
  }

  /// Applies [e] to [pattern]. The caller works on a copy, so a refusal part way
  /// through changes nothing.
  void edit(int pattern, GridEditDto e) {
    final p = _pattern(pattern);
    _generation++;
    switch (e) {
      case GridEditDto_Place(:final lane, :final at, :final cells, :final action):
        _checkSnap(at.parts);
        if (at.cell >= at.parts || cells < 1) _refuse('PE-14', 'bad cell');
        final steps = p.lane(lane);
        if (steps.any((s) => _compare(s.cell, at) == 0)) {
          _refuse('PT', 'a step already starts there');
        }
        final pos = steps.where((s) => _compare(s.cell, at) < 0).length;
        steps.insert(pos, _Step(at, cells, action));
        for (final s in p.spans.where((s) => s.lane == lane)) {
          if (s.first >= pos) {
            s.first++;
            s.last++;
          } else if (s.last >= pos) {
            s.last++;
          }
        }
      case GridEditDto_Move(:final lane, :final index, :final to):
        _checkSnap(to.parts);
        p.step(lane, index).cell = to;
        p.lane(lane).sort((a, b) => _compare(a.cell, b.cell));
      case GridEditDto_Resize(:final lane, :final index, :final parts, :final cells):
        _checkSnap(parts);
        p.step(lane, index).cells = cells;
      case GridEditDto_Delete(:final lane, :final first, :final last):
        final steps = p.lane(lane);
        if (first > last || last >= steps.length) _refuse('PE-14', 'bad run');
        steps.removeRange(first, last + 1);
        p.spans.removeWhere(
          (s) => s.lane == lane && s.first >= first && s.last <= last,
        );
      case GridEditDto_SetAction(:final lane, :final index, :final action):
        p.step(lane, index).action = action;
      case GridEditDto_ToggleTarget(:final lane, :final index, :final selector):
        final s = p.step(lane, index);
        final a = s.action;
        if (a is! GridActionDto_Strike) _refuse('PE-14', 'not a strike');
        final targets = List.of(a.strike.targets);
        if (!targets.remove(selector)) targets.add(selector);
        if (targets.isEmpty) _refuse('PE-14', 'needs a target');
        s.action = GridActionDto.strike(
          strike: GridStrikeDto(
            targets: targets,
            direction: a.strike.direction,
            techniques: a.strike.techniques,
          ),
        );
      case GridEditDto_SetDirection(:final lane, :final first, :final last, :final direction):
        for (var i = first; i <= last; i++) {
          final s = p.step(lane, i);
          final a = s.action;
          if (a is GridActionDto_Strike) {
            s.action = GridActionDto.strike(
              strike: GridStrikeDto(
                targets: a.strike.targets,
                direction: direction,
                techniques: a.strike.techniques,
              ),
            );
          }
        }
      case GridEditDto_SetTechnique(:final lane, :final index, :final technique, :final on_):
        final s = p.step(lane, index);
        final a = s.action;
        if (a is! GridActionDto_Strike) _refuse('PE-14', 'not a strike');
        s.action = GridActionDto.strike(
          strike: GridStrikeDto(
            targets: a.strike.targets,
            direction: a.strike.direction,
            techniques: _with(a.strike.techniques, technique, on_),
          ),
        );
      case GridEditDto_AddSpan(:final lane, :final first, :final last, :final technique):
        if (first > last || last >= p.lane(lane).length) {
          _refuse('PE-14', 'bad run');
        }
        p.spans.add(_Span(lane, first, last, technique));
      case GridEditDto_RemoveSpan(:final index):
        if (index >= p.spans.length) _refuse('PE-14', 'no span $index');
        p.spans.removeAt(index);
      case GridEditDto_SetLength(:final numerator, :final denominator):
        p.lengthNumerator = numerator;
        p.lengthDenominator = denominator;
      case GridEditDto_SetMeter(:final meter):
        p.meter = meter;
      case GridEditDto_SetName(:final name):
        p.name = name;
      case GridEditDto_SetMeta(:final description):
        p.description = description;
    }
  }

  static TechniquesDto _with(TechniquesDto t, TechniqueKindDto k, bool on) =>
      TechniquesDto(
        accent: k == TechniqueKindDto.accent ? on : t.accent,
        palmMute: k == TechniqueKindDto.palmMute ? on : t.palmMute,
        dead: k == TechniqueKindDto.dead ? on : t.dead,
        letRing: k == TechniqueKindDto.letRing ? on : t.letRing,
        stringPercussive: k == TechniqueKindDto.stringPercussive
            ? on
            : t.stringPercussive,
      );

  /// Orders cells by their start: beat + cell / parts.
  static int _compare(GridCellDto a, GridCellDto b) {
    final x = (a.beat * a.parts + a.cell) * b.parts;
    final y = (b.beat * b.parts + b.cell) * a.parts;
    return x.compareTo(y);
  }

  static GridSelectorDto _sel(SelectorKindDto kind, {int count = 0, int string = 0}) =>
      GridSelectorDto(kind: kind, count: count, string: string, from: 0, to: 0);

  GridViewDto view(int id, List<int> snaps) {
    snaps.forEach(_checkSnap);
    final p = _pattern(id);
    final beatDen = p.meter?.denominator ?? 4;
    // Whole beats, rounded up.
    final beats = math.max(
      1,
      (p.lengthNumerator * beatDen + p.lengthDenominator - 1) ~/
          p.lengthDenominator,
    );
    final fallback = snaps.isEmpty ? 2 : snaps.last;
    int snapOf(int beat) => beat < snaps.length ? snaps[beat] : fallback;
    final rowSelectors = [
      _sel(SelectorKindDto.bass),
      _sel(SelectorKindDto.alt),
      _sel(SelectorKindDto.treble, count: 3),
      for (var n = 1; n <= 6; n++) _sel(SelectorKindDto.string, string: n),
      _sel(SelectorKindDto.strum),
    ];
    String label(GridSelectorDto s) => switch (s.kind) {
      SelectorKindDto.bass => 'Bass',
      SelectorKindDto.alt => 'Alt',
      SelectorKindDto.treble => s.count == 3 ? 'Treble' : 'Treble(${s.count})',
      SelectorKindDto.string => 'String ${s.string}',
      SelectorKindDto.strum => 'Strum',
      SelectorKindDto.strumRange => 'Strum(${s.from}..${s.to})',
    };
    final rows = [
      for (final s in rowSelectors) GridRowDto(label: label(s), selector: s),
      for (final g in _gestures) GridRowDto(label: g.name, gesture: g.id),
    ];
    final steps = <GridStepDto>[];
    for (final lane in p.lanes) {
      for (final (i, s) in p.lane(lane).indexed) {
        final c = s.cell;
        // Fractions of a whole note: (beat + cell / parts) / beatDen.
        final den = c.parts * beatDen;
        final on = c.beat * c.parts + c.cell;
        final a = s.action;
        final marked = switch (a) {
          GridActionDto_Strike(:final strike) => [
            for (final t in strike.targets)
              if (rowSelectors.contains(t)) rowSelectors.indexOf(t),
          ],
          GridActionDto_Gesture(:final gesture) => [
            rows.indexWhere((r) => r.gesture == gesture),
          ],
          GridActionDto_Hold(:final strings) => [
            for (final t in strings ?? const <GridSelectorDto>[])
              if (rowSelectors.contains(t)) rowSelectors.indexOf(t),
          ],
          GridActionDto_Rest() => <int>[],
        };
        final spanFlags = p.spans
            .where((x) => x.lane == lane && x.first <= i && i <= x.last)
            .fold(
              const TechniquesDto(
                accent: false,
                palmMute: false,
                dead: false,
                letRing: false,
                stringPercussive: false,
              ),
              (t, x) => _with(t, x.technique, true),
            );
        steps.add(
          GridStepDto(
            lane: lane,
            index: i,
            onsetNumerator: on,
            onsetDenominator: den,
            endNumerator: on + s.cells,
            endDenominator: den,
            value: const DurationDto(base: 3, dots: 0),
            // The cell in the snap of the view, if the step starts on it.
            cell: (c.cell * snapOf(c.beat)) % c.parts == 0
                ? GridCellDto(
                    beat: c.beat,
                    parts: snapOf(c.beat),
                    cell: c.cell * snapOf(c.beat) ~/ c.parts,
                  )
                : null,
            kind: switch (a) {
              GridActionDto_Strike() => GridStepKindDto.strike,
              GridActionDto_Gesture() => GridStepKindDto.gesture,
              GridActionDto_Hold() => GridStepKindDto.hold,
              GridActionDto_Rest() => GridStepKindDto.rest,
            },
            gesture: a is GridActionDto_Gesture ? a.gesture : null,
            rows: Uint32List.fromList(marked),
            direction: a is GridActionDto_Strike ? a.strike.direction : null,
            techniques: a is GridActionDto_Strike
                ? a.strike.techniques
                : const TechniquesDto(
                    accent: false,
                    palmMute: false,
                    dead: false,
                    letRing: false,
                    stringPercussive: false,
                  ),
            spanTechniques: spanFlags,
          ),
        );
      }
    }
    return GridViewDto(
      name: p.name,
      lengthNumerator: p.lengthNumerator,
      lengthDenominator: p.lengthDenominator,
      meter: p.meter,
      beatNumerator: 1,
      beatDenominator: beatDen,
      snaps: Uint8List.fromList([
        for (var b = 0; b < beats; b++) snapOf(b),
      ]),
      rows: rows,
      lanes: p.lanes,
      steps: steps,
      spans: [
        for (final (i, x) in p.spans.indexed)
          GridSpanDto(
            index: i,
            lane: x.lane,
            first: x.first,
            last: x.last,
            technique: x.technique,
            startNumerator: 0,
            startDenominator: 1,
            endNumerator: 0,
            endDenominator: 1,
          ),
      ],
    );
  }
}

final class _Step {
  _Step(this.cell, this.cells, this.action);
  GridCellDto cell;
  int cells;
  GridActionDto action;
  _Step copy() => _Step(cell, cells, action);
}

final class _Span {
  _Span(this.lane, this.first, this.last, this.technique);
  final PatternLaneDto lane;
  int first;
  int last;
  final TechniqueKindDto technique;
  _Span copy() => _Span(lane, first, last, technique);
}

final class _Pattern {
  _Pattern({
    required this.id,
    required this.name,
    required this.description,
    required this.twoLanes,
    required this.lengthNumerator,
    required this.lengthDenominator,
    required this.meter,
    List<_Step>? thumbOrStrum,
    List<_Step>? fingers,
    List<_Span>? spans,
  }) : steps = thumbOrStrum ?? [],
       fingers = fingers ?? [],
       spans = spans ?? [];

  final int id;
  String name;
  String description;
  final bool twoLanes;
  int lengthNumerator;
  int lengthDenominator;
  MeterDto? meter;

  /// The strum lane, or the thumb lane of a two-lane pattern.
  final List<_Step> steps;
  final List<_Step> fingers;
  final List<_Span> spans;

  List<PatternLaneDto> get lanes => twoLanes
      ? const [PatternLaneDto.thumb, PatternLaneDto.fingers]
      : const [PatternLaneDto.strum];

  List<_Step> lane(PatternLaneDto l) {
    if (!lanes.contains(l)) {
      throw ApiError(rule: 'PE-14', message: 'no ${l.name} lane');
    }
    return l == PatternLaneDto.fingers ? fingers : steps;
  }

  _Step step(PatternLaneDto l, int index) {
    final s = lane(l);
    if (index < 0 || index >= s.length) {
      throw ApiError(rule: 'PE-14', message: 'no step $index');
    }
    return s[index];
  }

  _Pattern copy() => _Pattern(
    id: id,
    name: name,
    description: description,
    twoLanes: twoLanes,
    lengthNumerator: lengthNumerator,
    lengthDenominator: lengthDenominator,
    meter: meter,
    thumbOrStrum: [for (final s in steps) s.copy()],
    fingers: [for (final s in fingers) s.copy()],
    spans: [for (final s in spans) s.copy()],
  );


}
