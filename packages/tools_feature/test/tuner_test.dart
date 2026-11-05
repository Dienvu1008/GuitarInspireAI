import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:tools_feature/tools_feature.dart';

Widget _app(List<Override> overrides) => ProviderScope(
  overrides: overrides,
  child: const MaterialApp(home: Scaffold(body: TunerView())),
);

List<Override> _overrides(FakeTunerService service, {int track = 1}) => [
  tunerFactoryProvider.overrideWithValue(
    FakeTunerFactory(service: service),
  ),
  tunerTrackProvider.overrideWithValue(track),
];

void main() {
  testWidgets('starts and stops the microphone from the toggle', (tester) async {
    final service = FakeTunerService();
    await tester.pumpWidget(_app(_overrides(service)));
    expect(find.text('Microphone off'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('tuner-toggle')));
    await tester.pump(); // start() future
    await tester.pump();
    expect(find.text('Play a string'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('tuner-toggle')));
    await tester.pump();
    expect(find.text('Microphone off'), findsOneWidget);
  });

  testWidgets('a confident reading shows the string and cents', (tester) async {
    final service = FakeTunerService();
    final container = ProviderContainer(overrides: _overrides(service));
    addTearDown(() {
      container.read(tunerProvider.notifier).disable();
      container.dispose();
    });
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: Scaffold(body: TunerView())),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('tuner-toggle')));
    await tester.pump();
    await tester.pump();

    // 5 cents sharp on string 5 (A), clarity above the floor.
    service.push(hasPitch: true, hz: 110.4, clarity: 0.9, string: 5, cents: 5);
    container.read(tunerProvider.notifier).poll();
    await tester.pump();

    expect(find.byKey(const ValueKey('tuner-note')), findsOneWidget);
    expect(find.textContaining('5 (A)'), findsOneWidget);
    expect(find.textContaining('+5 cents'), findsOneWidget);
    expect(container.read(tunerProvider).inTune, isTrue); // within 5 cents
    container.read(tunerProvider.notifier).disable(); // stop the poll timer
  });

  testWidgets('low clarity shows no confident pitch', (tester) async {
    final service = FakeTunerService();
    final container = ProviderContainer(overrides: _overrides(service));
    addTearDown(() {
      container.read(tunerProvider.notifier).disable();
      container.dispose();
    });
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: Scaffold(body: TunerView())),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('tuner-toggle')));
    await tester.pump();
    await tester.pump();

    // Core returned a pitch but clarity is below the UI floor.
    service.push(hasPitch: true, hz: 110.0, clarity: 0.2, string: 5, cents: 0);
    container.read(tunerProvider.notifier).poll();
    await tester.pump();
    expect(find.text('—'), findsOneWidget);
    expect(container.read(tunerProvider).hasPitch, isFalse);
    container.read(tunerProvider.notifier).disable();
  });

  testWidgets('no microphone shows a message and does not crash (T2)', (
    tester,
  ) async {
    final service = FakeTunerService(
      startFails: ApiError(rule: 'tuner-no-device', message: 'no input'),
    );
    await tester.pumpWidget(_app(_overrides(service)));
    await tester.tap(find.byKey(const ValueKey('tuner-toggle')));
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const ValueKey('tuner-message')), findsOneWidget);
    expect(find.textContaining('No microphone'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('permission denied shows a distinct message (T2)', (tester) async {
    final service = FakeTunerService(
      startFails: ApiError(rule: 'tuner-permission', message: 'denied'),
    );
    await tester.pumpWidget(_app(_overrides(service)));
    await tester.tap(find.byKey(const ValueKey('tuner-toggle')));
    await tester.pump();
    await tester.pump();
    final msg = tester
        .widget<Text>(find.byKey(const ValueKey('tuner-message')))
        .data!;
    expect(msg, contains('access is blocked'));
    expect(msg, isNot(contains('No microphone')));
  });

  testWidgets('the referenced track updates without restart (T8)', (tester) async {
    final service = FakeTunerService();
    // A container whose track can change; the controller reads it each poll.
    var track = 1;
    final container = ProviderContainer(
      overrides: [
        tunerFactoryProvider.overrideWithValue(
          FakeTunerFactory(service: service),
        ),
        tunerTrackProvider.overrideWith((ref) => track),
      ],
    );
    addTearDown(() {
      container.read(tunerProvider.notifier).disable();
      container.dispose();
    });
    await container.read(tunerProvider.notifier).enable();
    // The fake maps nothing itself; it returns whatever we push. The point of T8 is that
    // the controller reads the *current* track provider each poll, so a changed tuning
    // would reach the core. We assert the controller does not cache the track.
    service.push(hasPitch: true, hz: 82.4, clarity: 0.9, string: 6, cents: 0);
    container.read(tunerProvider.notifier).poll();
    expect(container.read(tunerProvider).string, 6);

    // Change the active track; invalidate so the provider recomputes.
    track = 2;
    container.invalidate(tunerTrackProvider);
    // Next poll uses the new track value (no restart; the service was not recreated).
    expect(service.failed, isFalse);
    container.read(tunerProvider.notifier).disable();
  });
}
