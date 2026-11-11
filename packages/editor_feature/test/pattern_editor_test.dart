import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

/// The pattern editor of ADR-034 (slice 6a-5d, PE-14): every edit is one core
/// command; place, select, resize, move and delete by mouse and keyboard; a snap per
/// beat; techniques on a step or a run; a gesture from a strike; semantic labels.

late FakeDocumentFactory _factory;

ProviderContainer _container() {
  _factory = FakeDocumentFactory();
  return ProviderContainer(
    overrides: [documentFactoryProvider.overrideWithValue(_factory)],
  );
}

Widget _host(ProviderContainer c) => UncontrolledProviderScope(
  container: c,
  child: MaterialApp(
    home: Scaffold(
      body: Builder(
        builder: (context) => TextButton(
          key: const ValueKey('open'),
          onPressed: () => showPatternEditor(context),
          child: const Text('Open'),
        ),
      ),
    ),
  ),
);

Future<void> _open(WidgetTester tester, ProviderContainer c) async {
  tester.view.physicalSize = const Size(1400, 1000);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(_host(c));
  await tester.tap(find.byKey(const ValueKey('open')));
  await tester.pumpAndSettle();
}

Finder _k(String key) => find.byKey(ValueKey(key));

int _revision(ProviderContainer c) => c.read(editorProvider).summary.revision;

DocumentService _service(ProviderContainer c) => c.read(editorProvider).service;

/// The pattern the editor shows: the newest one.
int _pattern(ProviderContainer c) => _service(c).patterns().last.id;

GridViewDto _view(ProviderContainer c, [List<int> snaps = const []]) =>
    _service(c).patternGrid(_pattern(c), snaps);

/// The editor row of a core row label: one lane band comes first for a one-lane
/// pattern.
int _row(ProviderContainer c, String label) =>
    _view(c).lanes.length + _view(c).rows.indexWhere((r) => r.label == label);

Future<void> _tap(WidgetTester tester, String key) async {
  await tester.tap(_k(key));
  await tester.pumpAndSettle();
}

Future<void> _cell(WidgetTester tester, int row, int beat, int cell) async {
  final key = 'pattern-cell-$row-$beat-$cell';
  await tester.ensureVisible(_k(key));
  await tester.pumpAndSettle();
  await _tap(tester, key);
}

Future<void> _key(WidgetTester tester, LogicalKeyboardKey key) async {
  await tester.sendKeyEvent(key);
  await tester.pumpAndSettle();
}

Future<void> _with(
  WidgetTester tester,
  LogicalKeyboardKey modifier,
  LogicalKeyboardKey key,
) async {
  await tester.sendKeyDownEvent(modifier);
  await tester.sendKeyEvent(key);
  await tester.sendKeyUpEvent(modifier);
  await tester.pumpAndSettle();
}

/// Creates a one-bar 4/4 pattern through the New pattern dialog.
Future<void> _newPattern(
  WidgetTester tester, {
  String name = 'Mine',
  bool twoLanes = false,
}) async {
  await _tap(tester, 'pattern-editor-new');
  await tester.enterText(_k('new-pattern-name'), name);
  if (twoLanes) await _tap(tester, 'new-pattern-two-lanes');
  await _tap(tester, 'new-pattern-ok');
}

void main() {
  testWidgets('a new pattern, then place, select, resize and delete by mouse', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    expect(find.text('Pattern editor'), findsOneWidget);
    final rev = _revision(c);
    await _newPattern(tester);
    expect(_revision(c), rev + 1, reason: 'one command');
    expect(_service(c).patterns().map((p) => p.name), contains('Mine'));
    expect(find.text('Mine'), findsOneWidget, reason: 'the editor shows it');

    final bass = _row(c, 'Bass');
    await _cell(tester, bass, 0, 0);
    expect(_revision(c), rev + 2, reason: 'a click places one step');
    var v = _view(c);
    expect(v.steps, hasLength(1));
    expect(v.rows[v.steps.single.rows.single].label, 'Bass');
    expect(v.steps.single.cell, const GridCellDto(beat: 0, parts: 2, cell: 0));

    // A click on the step selects it; ] makes it one cell longer (in the core).
    await _cell(tester, bass, 0, 0);
    expect(_revision(c), rev + 2, reason: 'selecting sends nothing');
    await _key(tester, LogicalKeyboardKey.bracketRight);
    expect(_revision(c), rev + 3);
    v = _view(c);
    expect(
      (v.steps.single.endNumerator, v.steps.single.endDenominator),
      (2, 8),
      reason: 'two cells of an eighth',
    );
    await _key(tester, LogicalKeyboardKey.delete);
    expect(_view(c).steps, isEmpty);
    expect(_revision(c), rev + 4);
  });

  testWidgets('mouse only: move and resize with the palette', (tester) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    await _newPattern(tester);
    final bass = _row(c, 'Bass');
    await _cell(tester, bass, 0, 0);
    await _tap(tester, 'pattern-tool-later');
    expect(
      _view(c).steps.single.cell,
      const GridCellDto(beat: 0, parts: 2, cell: 1),
    );
    await _tap(tester, 'pattern-tool-longer');
    final s = _view(c).steps.single;
    expect((s.endNumerator, s.endDenominator), (3, 8), reason: 'two cells');
    await _tap(tester, 'pattern-tool-shorter');
    await _tap(tester, 'pattern-tool-earlier');
    final back = _view(c).steps.single;
    expect(back.cell, const GridCellDto(beat: 0, parts: 2, cell: 0));
    expect((back.endNumerator, back.endDenominator), (1, 8));
  });

  testWidgets('a snap per beat, including 3', (tester) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    await _newPattern(tester);
    // Beat 2 in three cells.
    await tester.tap(_k('pattern-snap-1'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('2: 3').last);
    await tester.pumpAndSettle();
    expect(_k('pattern-cell-1-1-2'), findsOneWidget, reason: 'three cells');
    expect(_k('pattern-cell-1-0-2'), findsNothing, reason: 'beat 1 has two');
    await _cell(tester, _row(c, 'Bass'), 1, 2);
    final s = _view(c, [2, 3]).steps.single;
    expect(s.cell, const GridCellDto(beat: 1, parts: 3, cell: 2));
  });

  testWidgets('a technique is a flag on one step and a span on a run', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    await _newPattern(tester);
    final bass = _row(c, 'Bass');
    await _cell(tester, bass, 0, 0);
    await _cell(tester, bass, 1, 0);
    await _cell(tester, bass, 2, 0);
    // Run: select step 1, Shift+click step 3, palm mute.
    await _cell(tester, bass, 0, 0);
    await tester.ensureVisible(_k('pattern-cell-$bass-2-0'));
    await tester.pumpAndSettle();
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.tap(_k('pattern-cell-$bass-2-0'));
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pumpAndSettle();
    await _key(tester, LogicalKeyboardKey.keyP);
    var v = _view(c);
    expect(v.spans, hasLength(1));
    expect((v.spans.single.first, v.spans.single.last), (0, 2));
    expect(v.spans.single.technique, TechniqueKindDto.palmMute);
    // One step: accent toggles its flag; Down sets its direction.
    await _cell(tester, bass, 1, 0);
    await _tap(tester, 'pattern-tool-accent');
    await _key(tester, LogicalKeyboardKey.keyD);
    v = _view(c);
    expect(v.steps[1].techniques.accent, isTrue);
    expect(v.steps[1].direction, StrokeDto.down);
    expect(v.steps[0].techniques.accent, isFalse);
    await _key(tester, LogicalKeyboardKey.keyA);
    expect(_view(c).steps[1].techniques.accent, isFalse, reason: 'toggled off');
  });

  testWidgets('a gesture from a strike, in the palette and the grid', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    await _newPattern(tester);
    final strum = _row(c, 'Strum');
    await _cell(tester, strum, 0, 0);
    final rev = _revision(c);
    await _key(tester, LogicalKeyboardKey.keyG);
    await tester.enterText(_k('pattern-name-input'), 'Kịch');
    await _tap(tester, 'pattern-name-ok');
    expect(_revision(c), rev + 1, reason: 'one command');
    final g = _service(c).gestures().single;
    expect(g.name, 'Kịch');
    expect(_view(c).steps.single.kind, GridStepKindDto.gesture);
    expect(_k('pattern-gesture-${g.id}'), findsOneWidget);
    expect(find.text('Kịch'), findsWidgets, reason: 'a grid row and a chip');
    // Removing a gesture in use is refused by the core, and the reason is shown.
    await _tap(tester, 'pattern-gesture-remove-${g.id}');
    expect(_k('pattern-editor-error'), findsOneWidget);
    expect(_service(c).gestures(), hasLength(1));
    // Rename.
    await _tap(tester, 'pattern-gesture-rename-${g.id}');
    await tester.enterText(_k('pattern-name-input'), 'Chuck');
    await _tap(tester, 'pattern-name-ok');
    expect(_service(c).gestures().single.name, 'Chuck');
  });

  testWidgets('keyboard only: place, move, resize, rest, undo and redo', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    await _newPattern(tester);
    // The cursor starts on the lane band; Down moves to Bass.
    await _key(tester, LogicalKeyboardKey.arrowDown);
    await _key(tester, LogicalKeyboardKey.enter);
    expect(_view(c).steps, hasLength(1));
    // Alt+Right moves the selected step one cell later.
    await _with(
      tester,
      LogicalKeyboardKey.altLeft,
      LogicalKeyboardKey.arrowRight,
    );
    expect(
      _view(c).steps.single.cell,
      const GridCellDto(beat: 0, parts: 2, cell: 1),
    );
    // A rest at the cursor (beat 3).
    for (var i = 0; i < 4; i++) {
      await _key(tester, LogicalKeyboardKey.arrowRight);
    }
    await _key(tester, LogicalKeyboardKey.keyR);
    final kinds = _view(c).steps.map((s) => s.kind).toList();
    expect(kinds, [GridStepKindDto.strike, GridStepKindDto.rest]);
    final rev = _revision(c);
    await _with(
      tester,
      LogicalKeyboardKey.controlLeft,
      LogicalKeyboardKey.keyZ,
    );
    expect(_view(c).steps, hasLength(1), reason: 'undo');
    await _with(
      tester,
      LogicalKeyboardKey.controlLeft,
      LogicalKeyboardKey.keyY,
    );
    expect(_view(c).steps, hasLength(2), reason: 'redo');
    expect(_revision(c), rev + 2);
  });

  testWidgets('a refused edit shows the reason and changes nothing', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    await _newPattern(tester);
    final bass = _row(c, 'Bass');
    final alt = _row(c, 'Alt');
    await _cell(tester, bass, 0, 0);
    // The cursor row Alt, same cell: the lane already has a step there. A click
    // selects it (no command); Ctrl+click adds Alt to its targets.
    final rev = _revision(c);
    await _cell(tester, alt, 0, 0);
    expect(_revision(c), rev);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.tap(_k('pattern-cell-$alt-0-0'));
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.pumpAndSettle();
    expect(_view(c).steps.single.rows, hasLength(2));
    // Move past the end of the pattern: nothing to do. A move onto a cell where the
    // fake (and the core) refuse: two steps starting together.
    await _cell(tester, bass, 1, 0);
    await _cell(tester, bass, 1, 0); // select it
    await _with(
      tester,
      LogicalKeyboardKey.altLeft,
      LogicalKeyboardKey.arrowLeft,
    );
    await _with(
      tester,
      LogicalKeyboardKey.altLeft,
      LogicalKeyboardKey.arrowLeft,
    );
    expect(_k('pattern-editor-error'), findsOneWidget);
  });

  testWidgets('every cell has a semantic label; errors are live regions', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    await _newPattern(tester);
    final bass = _row(c, 'Bass');
    expect(
      find.bySemanticsLabel('Bass, beat 1, cell 1 of 2: empty'),
      findsOneWidget,
    );
    await _cell(tester, bass, 0, 0);
    expect(
      find.bySemanticsLabel(
        RegExp(r'^Bass, beat 1, cell 1 of 2: Strum strike 1'),
      ),
      findsOneWidget,
    );
    expect(
      find.bySemanticsLabel(
        RegExp(r'^Strum lane, beat 1, cell 1 of 2: Strum strike 1'),
      ),
      findsOneWidget,
    );
    expect(
      find.bySemanticsLabel(RegExp('Pattern grid of Mine')),
      findsOneWidget,
    );
    // Two steps starting together: the error is announced.
    await _tap(tester, 'pattern-tool-hold'); // at the cursor, on the strike
    expect(
      tester.getSemantics(_k('pattern-editor-error')),
      isSemantics(isLiveRegion: true),
    );
    handle.dispose();
  });

  testWidgets('a two-lane pattern edits the chosen lane', (tester) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    await _newPattern(tester, twoLanes: true);
    expect(_view(c).lanes, [PatternLaneDto.thumb, PatternLaneDto.fingers]);
    expect(_k('pattern-editor-lane'), findsOneWidget);
    final one = _row(c, 'String 1');
    await _tap(tester, 'pattern-editor-lane');
    await tester.tap(find.text('Fingers'));
    await tester.pumpAndSettle();
    await _cell(tester, one, 0, 0);
    expect(_view(c).steps.single.lane, PatternLaneDto.fingers);
  });
}
