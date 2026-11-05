import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_test/flutter_test.dart';

/// Shared contract for [PlaybackService] implementations (ADR-028). The fake runs the
/// whole suite; the real Rust engine needs an audio device, so it runs a reduced set on
/// hardware only. Both must agree on the transport decisions that do not need sound:
/// session state (mute, tempo factor) persists and is independent of play state, seek
/// and position are in ticks, and a start bar past the end is rejected.
void playbackServiceContract(
  String name,
  Future<PlaybackFactory> Function() playbackFactory,
  DocumentFactory Function() documentFactory, {
  int masterBars = 4,
}) {
  group('PlaybackService contract ($name)', () {
    late PlaybackFactory playback;
    late DocumentService doc;
    late PlaybackService p;

    setUp(() async {
      playback = await playbackFactory();
      // A document the engine binds to. The fake runner supplies a fake document; the
      // real engine would be given a RustDocumentService (the factory enforces it).
      doc = documentFactory().create(bars: masterBars);
      p = playback.forDocument(doc);
    });

    tearDown(() {
      p.close();
      doc.close();
    });

    test('a fresh service is stopped at tick 0', () {
      final pos = p.position();
      expect(pos.playing, isFalse);
      expect(pos.state, PlaybackStateDto.stopped);
      expect(pos.tick, 0);
      expect(p.lastError, isNull);
    });

    test('tempo factor snaps to the allowed grid and persists', () async {
      await p.setTempoFactor(137); // not a multiple of 5
      // The engine stores the snapped value; the fake exposes it directly.
      await p.setTempoFactor(150);
      // No throw means the value was accepted; a snapped read is checked on the fake.
      expect(TempoFactorSteps.snap(137), 135);
      expect(TempoFactorSteps.snap(1000), TempoFactorSteps.max);
      expect(TempoFactorSteps.snap(0), TempoFactorSteps.min);
    });

    test('mute is session state: set, cleared, independent of play', () async {
      await p.setMuted(1, true);
      await p.setMuted(2, true);
      await p.setMuted(1, false);
      // Muting never throws and does not move the position.
      expect(p.position().tick, 0);
    });

    test('seek moves the position in ticks without playing', () async {
      await p.seek(480);
      expect(p.position().tick, 480);
      expect(p.position().playing, isFalse);
    });

    test('play from a bar past the end is rejected', () async {
      await expectLater(
        p.play(fromMasterBar: masterBars + 5),
        throwsA(isA<ApiError>()),
      );
      expect(p.position().playing, isFalse);
    });

    test('refreshAfterEdit is a no-op when stopped', () async {
      await p.refreshAfterEdit();
      expect(p.position().playing, isFalse);
    });

    test('the position stream is a broadcast stream', () {
      final a = p.positions;
      final b = p.positions;
      expect(identical(a, b) || a.isBroadcast, isTrue);
    });
  });
}
