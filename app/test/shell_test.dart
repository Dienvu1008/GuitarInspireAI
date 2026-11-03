import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
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

Future<void> _ctrl(WidgetTester tester, LogicalKeyboardKey key) async {
  await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
  await tester.sendKeyEvent(key);
  await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
  await tester.pump();
}

void main() {
  testWidgets('shell shows menus, the track list and an untitled document', (
    tester,
  ) async {
    await _pumpShell(tester);
    expect(find.text('File'), findsOneWidget);
    expect(find.text('Edit'), findsOneWidget);
    expect(find.text('Guitar'), findsOneWidget);
    expect(find.byType(TabView), findsOneWidget);
    expect(find.text('* Untitled'), findsOneWidget);

    // Ctrl+N replaces the document; the shell keeps working.
    await _ctrl(tester, LogicalKeyboardKey.keyN);
    expect(find.text('Guitar'), findsOneWidget);
  });

  testWidgets('typed digits form one fret and one undo step (Q-15)', (
    tester,
  ) async {
    final c = await _pumpShell(tester);
    List<int> frets(int bar) {
      final s = c.read(editorProvider);
      return [
        for (final b in s.service.beats(s.activeTrack, bar, 0))
          for (final n in b.notes) n.fret,
      ];
    }

    // On the append slot: "1", "2" inserts one beat with fret 12 on string 1.
    await tester.sendKeyEvent(LogicalKeyboardKey.digit1);
    await tester.sendKeyEvent(LogicalKeyboardKey.digit2);
    await tester.pump();
    expect(frets(0), [12]);
    expect(c.read(editorProvider).cursor.beat, 0);

    // Right moves to the append slot after the beat; a new digit adds a second beat.
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.sendKeyEvent(LogicalKeyboardKey.digit3);
    await tester.pump();
    expect(frets(0), [12, 3]);
    final second = c.read(editorProvider);
    expect(
      second.service.beats(second.activeTrack, 0, 0)[1].notes.single.string,
      2,
    );

    // Each note is one undo step, including the beat it created.
    await _ctrl(tester, LogicalKeyboardKey.keyZ);
    expect(frets(0), [12]);
    await _ctrl(tester, LogicalKeyboardKey.keyZ);
    expect(frets(0), isEmpty);
    await _ctrl(tester, LogicalKeyboardKey.keyY);
    expect(frets(0), [12]);

    // Let the coalescing window close before the test ends.
    await tester.pump(const Duration(milliseconds: coalesceWindowMs + 100));
  });

  testWidgets('Tab is left to keyboard focus traversal', (tester) async {
    await _pumpShell(tester);
    final before = FocusManager.instance.primaryFocus;
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    expect(FocusManager.instance.primaryFocus, isNot(same(before)));
  });

  testWidgets('Add track adds a second track that can be made active', (
    tester,
  ) async {
    final c = await _pumpShell(tester);
    await tester.tap(find.byKey(const ValueKey('add-track')));
    await tester.pump();
    final s = c.read(editorProvider);
    expect(s.summary.tracks, hasLength(2));
    expect(find.text('Track 2'), findsOneWidget);
    expect(
      s.summary.tracks[1].instrument,
      s.summary.tracks[0].instrument,
      reason: 'copies the active instrument',
    );

    await tester.tap(find.text('Track 2'));
    await tester.pump();
    expect(c.read(editorProvider).activeTrack, s.summary.tracks[1].id);

    // Keys still reach the editor after clicking in the track list.
    await tester.sendKeyEvent(LogicalKeyboardKey.digit5);
    await tester.pump();
    final after = c.read(editorProvider);
    expect(after.service.beats(after.activeTrack, 0, 0), hasLength(1));
    expect(after.service.beats(after.summary.tracks[0].id, 0, 0), isEmpty);
    await tester.pump(const Duration(milliseconds: coalesceWindowMs + 100));
  });
}
