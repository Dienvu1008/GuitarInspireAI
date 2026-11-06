import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:guitar_inspire/src/shell.dart';
import 'package:guitar_inspire/src/tools_wiring.dart';
import 'package:tools_feature/tools_feature.dart';

Future<ProviderContainer> _pump(WidgetTester tester) async {
  tester.view.physicalSize = const Size(1700, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        documentFactoryProvider.overrideWithValue(FakeDocumentFactory()),
        ...toolsOverrides(),
      ],
      child: const GuitarInspireApp(),
    ),
  );
  return ProviderScope.containerOf(tester.element(find.byType(Shell)));
}

Future<void> _openChordPanel(WidgetTester tester) async {
  await tester.tap(find.text('Tools'));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const ValueKey('tools-chord')));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('Tools menu shows the chord finder panel', (tester) async {
    await _pump(tester);
    expect(find.byType(ChordFinderView), findsNothing);
    await _openChordPanel(tester);
    expect(find.byType(ChordFinderView), findsOneWidget);
  });

  testWidgets('finding and inserting a chord adds notes at the cursor (T7)', (
    tester,
  ) async {
    final c = await _pump(tester);
    await _openChordPanel(tester);

    await tester.enterText(find.byKey(const ValueKey('chord-input')), 'C');
    await tester.tap(find.byKey(const ValueKey('chord-find')));
    await tester.pump();
    expect(find.byType(FretboardView), findsWidgets);

    // The fake document starts empty; insert the first voicing at the cursor.
    final before = c.read(editorProvider);
    final track = before.activeTrack;
    expect(before.service.beats(track, 0, 0), isEmpty);

    await tester.tap(find.byKey(const ValueKey('insert-0')).first);
    await tester.pump();
    await tester.pump();

    final beats = c.read(editorProvider).service.beats(track, 0, 0);
    expect(beats, hasLength(1), reason: 'one beat created at the cursor');
    // The fake voicing x32010 has 5 sounding strings (string 6 muted).
    expect(beats.single.notes.length, 5);
  });
}
