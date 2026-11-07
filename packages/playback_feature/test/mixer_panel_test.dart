import 'package:core_bridge/testing.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:core_bridge/core_bridge.dart' show PlaybackService, PositionDto;
import 'package:flutter_test/flutter_test.dart';
import 'package:playback_feature/playback_feature.dart';

const _tracks = [
  MixerTrack(id: 1, name: 'Lead'),
  MixerTrack(id: 2, name: 'Rhythm'),
  MixerTrack(id: 3, name: 'Bass'),
];

({ProviderScope scope, FakePlaybackService service}) _harness() {
  final service = FakePlaybackService(masterBars: 4);
  final scope = ProviderScope(
    overrides: [
      playbackServiceProvider.overrideWithValue(service),
      midiServiceProvider.overrideWithValue(FakeMidiService()),
      mixerTracksProvider.overrideWithValue(_tracks),
    ],
    child: const MaterialApp(
      home: Scaffold(
        body: SizedBox(width: 300, height: 700, child: MixerPanel()),
      ),
    ),
  );
  return (scope: scope, service: service);
}

Slider _slider(WidgetTester tester, String key) =>
    tester.widget<Slider>(find.byKey(ValueKey(key)));

void main() {
  testWidgets('X13: every track shows M, S, volume and pan', (tester) async {
    final h = _harness();
    await tester.pumpWidget(h.scope);
    for (final t in _tracks) {
      expect(find.text(t.name), findsOneWidget);
      expect(find.byKey(ValueKey('mixer-mute-${t.id}')), findsOneWidget);
      expect(find.byKey(ValueKey('mixer-solo-${t.id}')), findsOneWidget);
      expect(_slider(tester, 'mixer-volume-${t.id}').value, 100);
      expect(_slider(tester, 'mixer-pan-${t.id}').value, 0);
    }
    // The panel says it is session state.
    expect(find.byKey(const ValueKey('mixer-note')), findsOneWidget);
    h.service.close();
  });

  testWidgets('X13: M and S send one command each and show the state', (
    tester,
  ) async {
    final h = _harness();
    await tester.pumpWidget(h.scope);

    await tester.tap(find.byKey(const ValueKey('mixer-mute-2')));
    await tester.pump();
    expect(h.service.isMuted(2), isTrue);
    expect(h.service.isMuted(1), isFalse);
    expect(find.byTooltip('Unmute Rhythm'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('mixer-solo-1')));
    await tester.pump();
    expect(h.service.isSoloed(1), isTrue);
    expect(find.byTooltip('Unsolo Lead'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('mixer-solo-1')));
    await tester.pump();
    expect(h.service.isSoloed(1), isFalse);
    expect(find.byTooltip('Solo Lead'), findsOneWidget);
    h.service.close();
  });

  testWidgets('X13: a slider change sends exactly one command', (tester) async {
    final h = _harness();
    await tester.pumpWidget(h.scope);

    _slider(tester, 'mixer-volume-3').onChanged!(80);
    await tester.pump();
    expect(h.service.mixCalls, [(track: 3, volume: 80, pan: 0)]);
    expect(_slider(tester, 'mixer-volume-3').value, 80);

    _slider(tester, 'mixer-pan-3').onChanged!(-20);
    await tester.pump();
    expect(h.service.mixCalls.last, (track: 3, volume: 80, pan: -20));
    expect(h.service.mixCalls, hasLength(2));
    // Other tracks keep their defaults.
    expect(_slider(tester, 'mixer-volume-1').value, 100);
    h.service.close();
  });

  testWidgets('Reset returns volume and pan to the defaults', (tester) async {
    final h = _harness();
    await tester.pumpWidget(h.scope);
    final reset = find.byKey(const ValueKey('mixer-reset-2'));
    expect(
      tester.widget<IconButton>(reset).onPressed,
      isNull,
      reason: 'default',
    );

    _slider(tester, 'mixer-volume-2').onChanged!(30);
    await tester.pump();
    _slider(tester, 'mixer-pan-2').onChanged!(40);
    await tester.pump();
    expect(tester.widget<IconButton>(reset).onPressed, isNotNull);

    await tester.tap(reset);
    await tester.pump();
    expect(h.service.mixOf(2), (volume: 100, pan: 0));
    expect(_slider(tester, 'mixer-volume-2').value, 100);
    expect(_slider(tester, 'mixer-pan-2').value, 0);
    h.service.close();
  });

  testWidgets('sliders can be reached and moved with the keyboard', (
    tester,
  ) async {
    final h = _harness();
    await tester.pumpWidget(h.scope);
    // Tab order in the first row: M, S, (Reset is disabled at the default), then the
    // volume slider.
    for (var i = 0; i < 3; i++) {
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pump();
    }
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    await tester.pump();
    expect(h.service.mixCalls, hasLength(1));
    expect(h.service.mixCalls.single.track, 1);
    expect(h.service.mixCalls.single.volume, lessThan(100));
    h.service.close();
  });

  test(
    'X13: a rejected value shows the error and keeps the last good one',
    () async {
      final service = FakePlaybackService(masterBars: 4);
      final container = ProviderContainer(
        overrides: [playbackServiceProvider.overrideWithValue(service)],
      );
      addTearDown(() {
        container.dispose();
        service.close();
      });
      final transport = container.read(transportProvider.notifier);

      expect(await transport.setTrackMix(1, volume: 60, pan: 5), isTrue);
      expect(await transport.setTrackMix(1, volume: 200, pan: 5), isFalse);

      final s = container.read(transportProvider);
      expect(s.message, contains('transport'));
      expect(
        s.mixOf(1),
        const TrackMixView(volume: 60, pan: 5),
        reason: 'reverted',
      );
      expect(service.mixCalls, [
        (track: 1, volume: 60, pan: 5),
      ], reason: 'rejected not stored');
    },
  );

  test('a burst of changes sends in order and the newest value wins', () async {
    final service = FakePlaybackService(masterBars: 4);
    final container = ProviderContainer(
      overrides: [playbackServiceProvider.overrideWithValue(service)],
    );
    addTearDown(() {
      container.dispose();
      service.close();
    });
    final transport = container.read(transportProvider.notifier);

    final sends = [
      for (final v in [10, 20, 30, 40, 50])
        transport.setTrackMix(2, volume: v, pan: 0),
    ];
    await Future.wait(sends);

    final volumes = [for (final c in service.mixCalls) c.volume];
    expect(volumes.last, 50, reason: 'newest wins');
    expect(volumes, orderedEquals([...volumes]..sort()), reason: 'in order');
    expect(container.read(transportProvider).mixOf(2).volume, 50);
    expect(service.mixOf(2).volume, 50);
  });

  test('a slow earlier send cannot overwrite a newer value', () async {
    final inner = FakePlaybackService(masterBars: 4);
    final container = ProviderContainer(
      overrides: [
        playbackServiceProvider.overrideWithValue(_ReversingService(inner)),
      ],
    );
    addTearDown(() {
      container.dispose();
      inner.close();
    });
    final transport = container.read(transportProvider.notifier);
    await Future.wait([
      for (final v in [10, 20, 30, 40, 50])
        transport.setTrackMix(2, volume: v, pan: 0),
    ]);
    final volumes = [for (final c in inner.mixCalls) c.volume];
    expect(volumes.last, 50);
    expect(volumes, orderedEquals([...volumes]..sort()));
    expect(inner.mixOf(2).volume, 50);
  });
  test('a default mix is not kept in the state', () async {
    final service = FakePlaybackService(masterBars: 4);
    final container = ProviderContainer(
      overrides: [playbackServiceProvider.overrideWithValue(service)],
    );
    addTearDown(() {
      container.dispose();
      service.close();
    });
    final transport = container.read(transportProvider.notifier);
    await transport.setTrackMix(1, volume: 50, pan: 0);
    expect(container.read(transportProvider).mixes, isNotEmpty);
    await transport.resetMix(1);
    expect(container.read(transportProvider).mixes, isEmpty);
  });
}

/// A service whose volume/pan sends take longer the earlier they were issued, as an
/// async bridge that does not keep order could behave. Everything else forwards.
class _ReversingService implements PlaybackService {
  _ReversingService(this.inner);

  final FakePlaybackService inner;
  int _issued = 0;

  @override
  Future<void> setTrackMix(
    int track, {
    required int volume,
    required int pan,
  }) async {
    final delay = Duration(milliseconds: (6 - _issued++).clamp(1, 6) * 15);
    await Future<void>.delayed(delay);
    return inner.setTrackMix(track, volume: volume, pan: pan);
  }

  @override
  Future<void> play({int? fromMasterBar}) =>
      inner.play(fromMasterBar: fromMasterBar);
  @override
  Future<void> stop() => inner.stop();
  @override
  Future<void> seek(int tick) => inner.seek(tick);
  @override
  Future<void> setMuted(int track, bool muted) => inner.setMuted(track, muted);
  @override
  Future<void> setSoloed(int track, bool soloed) =>
      inner.setSoloed(track, soloed);
  @override
  Future<void> setTempoFactor(int hundredths) =>
      inner.setTempoFactor(hundredths);
  @override
  Future<void> refreshAfterEdit() => inner.refreshAfterEdit();
  @override
  PositionDto position() => inner.position();
  @override
  String? get lastError => inner.lastError;
  @override
  Stream<PositionDto> get positions => inner.positions;
  @override
  void close() => inner.close();
}

void main2() {}
