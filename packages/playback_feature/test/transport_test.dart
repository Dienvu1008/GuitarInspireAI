import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:playback_feature/playback_feature.dart';

/// Builds a ProviderScope with a fake playback service bound to a fake document, plus a
/// fake MIDI service. Returns the service so tests can inspect it.
({ProviderScope scope, FakePlaybackService service, FakeDocumentService doc})
_harness(Widget child) {
  final doc = FakeDocumentService(bars: 4);
  final service = FakePlaybackService(masterBars: 4);
  final scope = ProviderScope(
    overrides: [
      playbackServiceProvider.overrideWithValue(service),
      midiServiceProvider.overrideWithValue(FakeMidiService()),
    ],
    child: MaterialApp(home: Scaffold(body: child)),
  );
  return (scope: scope, service: service, doc: doc);
}

void main() {
  testWidgets('play/stop button toggles the transport', (tester) async {
    final h = _harness(const TransportBar(cursorMasterBar: 0));
    await tester.pumpWidget(h.scope);
    expect(find.byIcon(Icons.play_arrow), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('transport-play-stop')));
    await tester.pump(); // apply the command
    await tester.pump(const Duration(milliseconds: 20)); // one ticker frame
    expect(find.byIcon(Icons.stop), findsOneWidget);
    expect(h.service.position().playing, isTrue);

    await tester.tap(find.byKey(const ValueKey('transport-play-stop')));
    await tester.pump();
    expect(find.byIcon(Icons.play_arrow), findsOneWidget);
    expect(h.service.position().playing, isFalse);
    // Clean up the broadcast stream.
    h.service.close();
  });

  testWidgets('tempo nudges snap and clamp to the Q6 grid', (tester) async {
    final h = _harness(const TransportBar());
    await tester.pumpWidget(h.scope);
    expect(find.text('1x'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('tempo-up')));
    await tester.pump();
    expect(find.text('1.05x'), findsOneWidget);
    expect(h.service.tempoFactor, 105);

    // Down to the minimum and the button disables at 0.25x.
    final down = find.byKey(const ValueKey('tempo-down'));
    for (var i = 0; i < 40; i++) {
      final button = tester.widget<IconButton>(down);
      if (button.onPressed == null) break;
      await tester.tap(down);
      await tester.pump();
    }
    expect(find.text('0.25x'), findsOneWidget);
    expect(h.service.tempoFactor, TempoFactorSteps.min);
    h.service.close();
  });

  testWidgets('mute button toggles a track and reflects state', (tester) async {
    final h = _harness(const MuteButton(track: 1));
    await tester.pumpWidget(h.scope);
    expect(find.byIcon(Icons.volume_up), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('mute-1')));
    await tester.pump();
    expect(find.byIcon(Icons.volume_off), findsOneWidget);
    expect(h.service.isMuted(1), isTrue);

    await tester.tap(find.byKey(const ValueKey('mute-1')));
    await tester.pump();
    expect(find.byIcon(Icons.volume_up), findsOneWidget);
    expect(h.service.isMuted(1), isFalse);
    h.service.close();
  });

  testWidgets('the tick readout follows the position stream', (tester) async {
    final h = _harness(const TransportBar(cursorMasterBar: 0));
    await tester.pumpWidget(h.scope);
    await tester.tap(find.byKey(const ValueKey('transport-play-stop')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 20));
    await tester.pump(const Duration(milliseconds: 20));
    final tickText = tester
        .widget<Text>(find.byKey(const ValueKey('transport-tick')))
        .data!;
    final tick = int.parse(tickText.replaceAll('Tick ', ''));
    expect(tick, greaterThan(0));
    await tester.tap(find.byKey(const ValueKey('transport-play-stop')));
    await tester.pump();
    h.service.close();
  });

  testWidgets('a playback failure shows the engine message', (tester) async {
    final h = _harness(const TransportBar());
    await tester.pumpWidget(h.scope);
    h.service.failWith('SoundFont missing: GuitarSteel.sf2');
    await tester.pump();
    expect(find.byKey(const ValueKey('transport-message')), findsOneWidget);
    expect(find.textContaining('SoundFont missing'), findsOneWidget);
    h.service.close();
  });

  testWidgets('play from a bar past the end surfaces an error', (tester) async {
    final doc = FakeDocumentService(bars: 2);
    final service = FakePlaybackService(masterBars: 2);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          playbackServiceProvider.overrideWithValue(service),
          midiServiceProvider.overrideWithValue(FakeMidiService()),
        ],
        child: const MaterialApp(
          home: Scaffold(body: TransportBar(cursorMasterBar: 99)),
        ),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('transport-play-stop')));
    await tester.pump();
    expect(service.position().playing, isFalse);
    service.close();
    doc.close();
  });
}
