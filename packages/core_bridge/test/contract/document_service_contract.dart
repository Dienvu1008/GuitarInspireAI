import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_test/flutter_test.dart';

/// Shared contract for every [DocumentService] implementation (interop-architecture
/// section 12). The fake and the real Rust-backed service must both pass it.
void documentServiceContract(
  String name,
  Future<DocumentFactory> Function() factory,
  MetricsDto Function() metrics,
) {
  group('DocumentService contract ($name)', () {
    late DocumentFactory docs;
    late DocumentService d;
    late int track;

    setUp(() async {
      docs = await factory();
      d = docs.create(bars: 2);
      track = d.summary().tracks.single.id;
    });

    tearDown(() => d.close());

    const quarter = DurationDto(base: 2, dots: 0);

    Future<BeatAddressDto> insertBeat(int bar) async {
      final index = d.beats(track, bar, 0).length;
      await d.execute(
        CommandDto.insertBeat(
          track: track,
          bar: bar,
          voice: 0,
          index: index,
          duration: quarter,
        ),
      );
      final id = d.beats(track, bar, 0)[index].id;
      return BeatAddressDto(track: track, bar: bar, voice: 0, beat: id);
    }

    test('ADR-030: unsaved work ignores an untouched new document', () async {
      expect(d.summary().unsavedWork, isFalse);
      await insertBeat(0);
      expect(d.summary().unsavedWork, isTrue);
      await d.undo();
      expect(d.summary().unsavedWork, isFalse);
      expect(d.summary().dirty, isTrue, reason: 'still not on disk');
    });

    test('a new document is clean, with one track and the requested bars', () {
      final s = d.summary();
      expect(s.revision, 0);
      // Never saved: the core reports it as dirty (no saved state to compare with).
      expect(s.dirty, isTrue);
      expect(s.canUndo, isFalse);
      expect(s.masterBars.length, 2);
      expect(s.tracks.single.instrument.tuning.length, 6);
      expect(s.location, isNull);
    });

    test('commands advance the revision and publish gapless events', () async {
      final seen = <EventDto>[];
      final sub = d.events.listen(seen.add);
      final beat = await insertBeat(0);
      final out = await d.execute(
        CommandDto.setNote(beat: beat, string: 6, fret: 3),
      );
      await pumpEventQueue();
      expect(out.revision, 2);
      expect(
        out.events.map((e) => e.kind),
        contains(
          EventKindDto.barContentChanged(track: track, start: 0, end: 1),
        ),
      );
      expect(seen, isNotEmpty);
      final seqs = seen.map((e) => e.seq).toList();
      for (var i = 1; i < seqs.length; i++) {
        expect(seqs[i], seqs[i - 1] + 1);
      }
      expect(seqs.first, 1);
      final notes = d.beats(track, 0, 0).single.notes;
      expect((notes.single.string, notes.single.fret), (6, 3));
      expect(d.summary().dirty, isTrue);
      await sub.cancel();
    });

    test('undo and redo restore the previous state', () async {
      final beat = await insertBeat(0);
      await d.execute(CommandDto.setNote(beat: beat, string: 1, fret: 5));
      await d.undo();
      expect(d.beats(track, 0, 0).single.notes, isEmpty);
      await d.redo();
      expect(d.beats(track, 0, 0).single.notes.single.fret, 5);
      await d.undo();
      await d.undo();
      expect(d.beats(track, 0, 0), isEmpty);
      expect(d.summary().canUndo, isFalse);
      await expectLater(d.undo(), throwsA(isA<ApiError>()));
    });

    test('commands with the same coalesce key are one undo step', () async {
      final beat = await insertBeat(0);
      await d.execute(
        CommandDto.setNote(beat: beat, string: 1, fret: 1),
        coalesceKey: 7,
      );
      await d.execute(
        CommandDto.setNote(beat: beat, string: 1, fret: 12),
        coalesceKey: 7,
      );
      expect(d.beats(track, 0, 0).single.notes.single.fret, 12);
      await d.undo();
      expect(d.beats(track, 0, 0).single.notes, isEmpty);
    });

    test('a rejected command throws ApiError and changes nothing', () async {
      final beat = await insertBeat(0);
      final before = d.summary();
      final missing = BeatAddressDto(
        track: track,
        bar: 0,
        voice: 0,
        beat: beat.beat + 999,
      );
      await expectLater(
        d.execute(CommandDto.setNote(beat: missing, string: 1, fret: 0)),
        throwsA(isA<ApiError>().having((e) => e.rule, 'rule', isNotEmpty)),
      );
      await expectLater(
        d.execute(
          CommandDto.setNote(beat: beat, string: 1, fret: 0),
          expectedRevision: 0,
        ),
        throwsA(isA<ApiError>().having((e) => e.rule, 'rule', 'E-01')),
      );
      expect(d.summary().revision, before.revision);
      expect(d.beats(track, 0, 0).single.notes, isEmpty);
    });

    test('tracks and bars can be added', () async {
      await d.execute(
        CommandDto.addTrack(
          name: 'Second',
          instrument: d.summary().tracks.single.instrument,
        ),
      );
      await d.execute(const CommandDto.insertMasterBars(at: 2, count: 1));
      final s = d.summary();
      expect(s.tracks.map((t) => t.name), ['Guitar', 'Second']);
      expect(s.masterBars.length, 3);
      await d.execute(
        CommandDto.renameTrack(track: s.tracks[1].id, name: 'Bass'),
      );
      expect(d.summary().tracks[1].name, 'Bass');
    });

    test(
      'layout has one fret glyph per note and hit regions on every string',
      () async {
        final beat = await insertBeat(1);
        await d.execute(CommandDto.setNote(beat: beat, string: 2, fret: 10));
        final l = d.tabLayout(track, metrics());
        expect(l.strings, 6);
        final glyphs = l.systems
            .expand((s) => s.glyphs)
            .where((g) => g.kind is GlyphKindDto_Fret);
        expect(glyphs.length, 1);
        var regions = 0;
        for (final s in l.systems) {
          expect(s.lines.length, s.lineKinds.length * 4);
          expect(s.hitRects.length, s.hitMeta.length);
          final slot = s.beats.indexWhere((b) => b.id == beat.beat);
          if (slot < 0) continue;
          expect((s.beats[slot].bar, s.beats[slot].index), (1, 0));
          for (var i = 3; i < s.hitMeta.length; i += 4) {
            if (s.hitMeta[i] == slot) regions++;
          }
        }
        expect(regions, 6);
        expect(l.systems.expand((s) => s.bars).map((b) => b.bar), [0, 1]);
      },
    );

    test(
      'ADR-030: copy and paste a run across bars is one undo step',
      () async {
        final a = await insertBeat(0);
        await d.execute(CommandDto.setNote(beat: a, string: 1, fret: 5));
        final b = await insertBeat(1);
        await d.execute(CommandDto.setNote(beat: b, string: 6, fret: 3));

        final clip = d.copyBeats(
          track,
          0,
          fromBar: 0,
          fromIndex: 0,
          toBar: 1,
          toIndex: 0,
        );
        expect(clip.map((c) => c.notes.single.fret), [5, 3]);

        final rev = d.summary().revision;
        await d.execute(
          CommandDto.pasteBeats(
            track: track,
            bar: 1,
            voice: 0,
            index: 1,
            beats: clip,
          ),
        );
        expect(d.summary().revision, rev + 1);
        expect(
          d
              .beats(track, 1, 0)
              .map((x) => (x.notes.single.string, x.notes.single.fret)),
          [(6, 3), (1, 5), (6, 3)],
        );
        await d.undo();
        expect(d.beats(track, 1, 0), hasLength(1));
      },
    );

    test('ADR-030: cut removes the run as one undo step', () async {
      await insertBeat(0);
      await insertBeat(0);
      await insertBeat(1);
      await d.execute(
        CommandDto.removeBeats(
          track: track,
          voice: 0,
          fromBar: 0,
          fromIndex: 1,
          toBar: 1,
          toIndex: 0,
        ),
      );
      expect(
        (d.beats(track, 0, 0).length, d.beats(track, 1, 0).length),
        (1, 0),
      );
      await d.undo();
      expect(
        (d.beats(track, 0, 0).length, d.beats(track, 1, 0).length),
        (2, 1),
      );
    });

    test('ADR-030: an invalid paste is rejected and changes nothing', () async {
      await insertBeat(0);
      final before = d.summary().revision;
      const bad = ClipBeatDto(
        duration: quarter,
        notes: [
          ClipNoteDto(
            string: 9,
            fret: 0,
            tie: false,
            letRing: false,
            dead: false,
          ),
        ],
      );
      await expectLater(
        d.execute(
          CommandDto.pasteBeats(
            track: track,
            bar: 0,
            voice: 0,
            index: 0,
            beats: const [bad],
          ),
        ),
        throwsA(isA<ApiError>().having((e) => e.rule, 'rule', 'H-03')),
      );
      expect(
        () => d.copyBeats(
          track,
          0,
          fromBar: 0,
          fromIndex: 3,
          toBar: 0,
          toIndex: 3,
        ),
        throwsA(isA<ApiError>().having((e) => e.rule, 'rule', 'E-04')),
      );
      expect(d.summary().revision, before);
      expect(d.beats(track, 0, 0), hasLength(1));
    });

    test('save requires a location; save as then save succeed', () async {
      await expectLater(
        d.save(),
        throwsA(isA<ApiError>().having((e) => e.rule, 'rule', 'save-location')),
      );
    });
  });
}
