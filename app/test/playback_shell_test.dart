import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:guitar_inspire/src/playback_wiring.dart';
import 'package:guitar_inspire/src/shell.dart';
import 'package:playback_feature/playback_feature.dart';

/// Pumps the shell with fake document, playback and MIDI services and the playback
/// overrides applied (as the composition root does when the engine starts).
Future<ProviderContainer> _pumpWithPlayback(WidgetTester tester) async {
  tester.view.physicalSize = const Size(1400, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final playback = FakePlaybackFactory();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        documentFactoryProvider.overrideWithValue(FakeDocumentFactory()),
        ...playbackOverrides(playback, FakeMidiService()),
      ],
      child: const GuitarInspireApp(),
    ),
  );
  return ProviderScope.containerOf(tester.element(find.byType(Shell)));
}

void main() {
  testWidgets('the transport bar appears when playback is available', (
    tester,
  ) async {
    await _pumpWithPlayback(tester);
    expect(find.byType(TransportBar), findsOneWidget);
    expect(find.byKey(const ValueKey('transport-play-stop')), findsOneWidget);
    expect(find.byKey(const ValueKey('tempo-label')), findsOneWidget);
  });

  testWidgets('play then stop from the transport bar', (tester) async {
    final c = await _pumpWithPlayback(tester);
    await tester.tap(find.byKey(const ValueKey('transport-play-stop')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 20));
    expect(c.read(transportProvider).playing, isTrue);
    await tester.tap(find.byKey(const ValueKey('transport-play-stop')));
    await tester.pump();
    expect(c.read(transportProvider).playing, isFalse);
  });

  testWidgets('tempo up changes the displayed factor', (tester) async {
    final c = await _pumpWithPlayback(tester);
    expect(find.text('1x'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('tempo-up')));
    await tester.pump();
    expect(c.read(transportProvider).tempoFactor, 105);
    expect(find.text('1.05x'), findsOneWidget);
  });

  testWidgets('the File menu has MIDI import and export items', (tester) async {
    await _pumpWithPlayback(tester);
    await tester.tap(find.text('File'));
    // The playback cursor runs a Ticker, so the tree never "settles"; pump a few
    // frames to open the menu instead of pumpAndSettle.
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    expect(find.text('Import MIDI...'), findsOneWidget);
    expect(find.text('Export MIDI...'), findsOneWidget);
  });

  testWidgets('transport bar is hidden when playback is unavailable', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          documentFactoryProvider.overrideWithValue(FakeDocumentFactory()),
          // No engine: factory null, only MIDI available.
          ...playbackOverrides(null, FakeMidiService()),
        ],
        child: const GuitarInspireApp(),
      ),
    );
    expect(find.byType(TransportBar), findsNothing);
    // MIDI menu still works without an engine.
    await tester.tap(find.text('File'));
    await tester.pumpAndSettle();
    expect(find.text('Export MIDI...'), findsOneWidget);
  });
}
