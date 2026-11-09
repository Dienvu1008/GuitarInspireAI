// ADR-030: beat selection with Shift, in-app clipboard with Ctrl+C / X / V, through
// the real shell and keyboard handler on the fake core.
import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:editor_feature/src/system_painter.dart' show SelectionPainter;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart' show LogicalKeyboardKey;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:guitar_inspire/src/shell.dart';

Future<ProviderContainer> _pumpShell(WidgetTester tester) async {
  tester.view.physicalSize = const Size(1400, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        documentFactoryProvider.overrideWithValue(FakeDocumentFactory()),
      ],
      child: const GuitarInspireApp(),
    ),
  );
  return ProviderScope.containerOf(tester.element(find.byType(Shell)));
}

Future<void> _settle(WidgetTester tester) async {
  for (var i = 0; i < 4; i++) {
    await tester.pump();
  }
}

Future<void> _chord(
  WidgetTester tester,
  LogicalKeyboardKey modifier,
  LogicalKeyboardKey key,
) async {
  await tester.sendKeyDownEvent(modifier);
  await tester.sendKeyEvent(key);
  await tester.sendKeyUpEvent(modifier);
  await _settle(tester);
}

Future<void> _ctrl(WidgetTester t, LogicalKeyboardKey k) =>
    _chord(t, LogicalKeyboardKey.controlLeft, k);

Future<void> _shift(WidgetTester t, LogicalKeyboardKey k) =>
    _chord(t, LogicalKeyboardKey.shiftLeft, k);

/// Appends a quarter beat with [fret] on [string] to [bar] of the active track.
Future<void> _note(
  ProviderContainer c,
  int bar,
  int fret, {
  int string = 1,
}) async {
  final editor = c.read(editorProvider.notifier);
  final s = c.read(editorProvider);
  final index = s.service.beats(s.activeTrack, bar, 0).length;
  await editor.execute(
    CommandDto.insertBeat(
      track: s.activeTrack,
      bar: bar,
      voice: 0,
      index: index,
      duration: const DurationDto(base: 2, dots: 0),
    ),
  );
  final beat = s.service.beats(s.activeTrack, bar, 0)[index];
  await editor.execute(
    CommandDto.setNote(
      beat: BeatAddressDto(
        track: s.activeTrack,
        bar: bar,
        voice: 0,
        beat: beat.id,
      ),
      string: string,
      fret: fret,
    ),
  );
}

List<int> _frets(ProviderContainer c, int bar) {
  final s = c.read(editorProvider);
  return [
    for (final b in s.service.beats(s.activeTrack, bar, 0))
      for (final n in b.notes) n.fret,
  ];
}

void _cursor(ProviderContainer c, int bar, int? beat) => c
    .read(editorProvider.notifier)
    .moveCursor(EditorCursor(bar: bar, voice: 0, string: 1, beat: beat));

/// Bar 0: frets 1, 2. Bar 1: fret 3. Bars 2 and 3 empty.
Future<ProviderContainer> _song(WidgetTester tester) async {
  final c = await _pumpShell(tester);
  await _note(c, 0, 1);
  await _note(c, 0, 2);
  await _note(c, 1, 3);
  await _settle(tester);
  return c;
}

void main() {
  testWidgets('Shift+Right selects across bars; copy and paste is one undo step', (
    tester,
  ) async {
    final c = await _song(tester);
    _cursor(c, 0, 1);
    await _shift(tester, LogicalKeyboardKey.arrowRight);
    final sel = c.read(editorProvider).selection!;
    expect((sel.$1.bar, sel.$1.index, sel.$2.bar, sel.$2.index), (0, 1, 1, 0));

    // The selected beats are tinted, one column each.
    final painter =
        tester
                .widget<CustomPaint>(find.byKey(const ValueKey('selection-0')))
                .painter!
            as SelectionPainter;
    expect(painter.rects, hasLength(2));

    await _ctrl(tester, LogicalKeyboardKey.keyC);
    expect(c.read(beatClipboardProvider), hasLength(2));
    expect(c.read(editorProvider).message, 'Copied 2 beats');

    _cursor(c, 2, null); // append slot of an empty bar
    final undoBefore = c.read(editorProvider).summary.revision;
    await _ctrl(tester, LogicalKeyboardKey.keyV);
    expect(_frets(c, 2), [2, 3]);
    expect(c.read(editorProvider).summary.revision, undoBefore + 1);
    expect(c.read(editorProvider).message, 'Pasted 2 beats');

    await _ctrl(tester, LogicalKeyboardKey.keyZ);
    expect(_frets(c, 2), isEmpty, reason: 'one undo step removes the paste');
    expect(_frets(c, 0), [1, 2]);
  });

  testWidgets('Ctrl+X cuts the selection; undo restores it', (tester) async {
    final c = await _song(tester);
    _cursor(c, 0, 0);
    await _shift(tester, LogicalKeyboardKey.arrowRight);
    await _ctrl(tester, LogicalKeyboardKey.keyX);
    expect(_frets(c, 0), isEmpty);
    expect(_frets(c, 1), [3]);
    expect(c.read(editorProvider).selection, isNull);
    expect(c.read(beatClipboardProvider), hasLength(2));

    await _ctrl(tester, LogicalKeyboardKey.keyZ);
    expect(_frets(c, 0), [1, 2]);
    expect(_frets(c, 1), [3]);
  });

  testWidgets('without a selection the beat under the cursor is copied', (
    tester,
  ) async {
    final c = await _song(tester);
    _cursor(c, 1, 0);
    await _ctrl(tester, LogicalKeyboardKey.keyC);
    expect(c.read(editorProvider).message, 'Copied 1 beat');
    _cursor(c, 0, 0);
    await _ctrl(tester, LogicalKeyboardKey.keyV);
    expect(_frets(c, 0), [3, 1, 2], reason: 'paste inserts before the cursor');
  });

  testWidgets('the clipboard is app-level: it survives a new document', (
    tester,
  ) async {
    final c = await _song(tester);
    _cursor(c, 0, 0);
    await _shift(tester, LogicalKeyboardKey.arrowRight);
    await _ctrl(tester, LogicalKeyboardKey.keyC);

    await _ctrl(tester, LogicalKeyboardKey.keyN);
    expect(_frets(c, 0), isEmpty, reason: 'a new, empty document');
    await _ctrl(tester, LogicalKeyboardKey.keyV);
    expect(_frets(c, 0), [1, 2]);
  });

  testWidgets('paste with an empty clipboard only shows a message', (
    tester,
  ) async {
    final c = await _song(tester);
    final rev = c.read(editorProvider).summary.revision;
    _cursor(c, 0, 0);
    await _ctrl(tester, LogicalKeyboardKey.keyV);
    expect(c.read(editorProvider).message, 'The clipboard is empty');
    expect(c.read(editorProvider).summary.revision, rev);
  });

  testWidgets('a paste that does not fit the target track changes nothing', (
    tester,
  ) async {
    final c = await _pumpShell(tester);
    await _note(c, 0, 3, string: 6);
    await _settle(tester);
    _cursor(c, 0, 0);
    await _ctrl(tester, LogicalKeyboardKey.keyC);

    final editor = c.read(editorProvider.notifier);
    await editor.execute(
      const CommandDto.addTrack(
        name: 'Bass',
        instrument: InstrumentDto(
          tuning: ['G2', 'D2', 'A1', 'E1'],
          fretCount: 20,
          capo: 0,
        ),
      ),
    );
    editor.selectTrack(c.read(editorProvider).summary.tracks[1].id);
    final rev = c.read(editorProvider).summary.revision;
    await _ctrl(tester, LogicalKeyboardKey.keyV);
    expect(c.read(editorProvider).message, startsWith('H-03'));
    expect(c.read(editorProvider).summary.revision, rev);
    expect(_frets(c, 0), isEmpty);
  });

  testWidgets('Escape and a plain move clear the selection', (tester) async {
    final c = await _song(tester);
    _cursor(c, 0, 0);
    await _shift(tester, LogicalKeyboardKey.arrowRight);
    expect(c.read(editorProvider).selection, isNotNull);
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await _settle(tester);
    expect(c.read(editorProvider).selection, isNull);

    await _shift(tester, LogicalKeyboardKey.arrowRight);
    expect(c.read(editorProvider).selection, isNotNull);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    await _settle(tester);
    expect(c.read(editorProvider).selection, isNull);
  });
}
