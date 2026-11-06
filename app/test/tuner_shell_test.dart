import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:guitar_inspire/src/shell.dart';
import 'package:tools_feature/tools_feature.dart';

Future<ProviderContainer> _pump(WidgetTester tester, FakeTunerService tuner) async {
  tester.view.physicalSize = const Size(1600, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        documentFactoryProvider.overrideWithValue(FakeDocumentFactory()),
        tunerFactoryProvider.overrideWithValue(
          FakeTunerFactory(service: tuner),
        ),
        tunerTrackProvider.overrideWithValue(1),
      ],
      child: const GuitarInspireApp(),
    ),
  );
  return ProviderScope.containerOf(tester.element(find.byType(Shell)));
}

void main() {
  testWidgets('the Tools menu toggles the tuner panel', (tester) async {
    final tuner = FakeTunerService();
    await _pump(tester, tuner);

    // Hidden by default.
    expect(find.byType(TunerView), findsNothing);

    await tester.tap(find.text('Tools'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('tools-tuner')));
    await tester.pumpAndSettle();
    expect(find.byType(TunerView), findsOneWidget);

    // Toggling off hides it again.
    await tester.tap(find.text('Tools'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('tools-tuner')));
    await tester.pumpAndSettle();
    expect(find.byType(TunerView), findsNothing);
  });

  testWidgets('the tuner panel starts and stops the microphone', (tester) async {
    final tuner = FakeTunerService();
    final c = await _pump(tester, tuner);
    await tester.tap(find.text('Tools'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('tools-tuner')));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('tuner-toggle')));
    await tester.pump();
    await tester.pump();
    expect(c.read(tunerProvider).listening, isTrue);

    // Hiding the panel releases the microphone (privacy).
    await tester.tap(find.text('Tools'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('tools-tuner')));
    await tester.pumpAndSettle();
    expect(c.read(tunerProvider).listening, isFalse);
  });
}
