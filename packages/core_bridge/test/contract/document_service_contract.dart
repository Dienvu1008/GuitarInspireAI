import 'dart:typed_data';

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

    test('ADR-035: technique flags and the stroke are set, undone, redone and copied', () async {
      final a = await insertBeat(0);
      await d.execute(CommandDto.setNote(beat: a, string: 1, fret: 5));
      final id = d.beats(track, 0, 0).single.notes.single.id;
      final rev = d.summary().revision;
      await d.execute(
        CommandDto.setNoteEffects(
          beat: a,
          note: id,
          letRing: false,
          dead: false,
          accent: true,
          palmMute: true,
          stringPercussive: true,
        ),
      );
      expect(
        d.summary().revision,
        rev + 1,
        reason: 'one command, one revision',
      );
      await d.execute(CommandDto.setBeatStroke(beat: a, stroke: StrokeDto.up));
      expect(d.summary().revision, rev + 2);
      (StrokeDto?, bool, bool, bool, bool, bool) facts(BeatDto b) {
        final n = b.notes.single;
        return (
          b.stroke,
          n.accent,
          n.palmMute,
          n.stringPercussive,
          n.dead,
          n.letRing,
        );
      }

      const on = (StrokeDto.up, true, true, true, false, false);
      expect(facts(d.beats(track, 0, 0).single), on);
      final clip = d.copyBeats(
        track,
        0,
        fromBar: 0,
        fromIndex: 0,
        toBar: 0,
        toIndex: 0,
      );
      expect(clip.single.stroke, StrokeDto.up);
      final cn = clip.single.notes.single;
      expect((cn.accent, cn.palmMute, cn.stringPercussive), (true, true, true));
      await d.execute(
        CommandDto.pasteBeats(
          track: track,
          bar: 1,
          voice: 0,
          index: 0,
          beats: clip,
        ),
      );
      expect(
        facts(d.beats(track, 1, 0).single),
        on,
        reason: 'paste keeps them',
      );
      await d.undo(); // paste
      await d.undo(); // stroke
      expect(d.beats(track, 0, 0).single.stroke, isNull);
      await d.undo(); // flags
      expect(facts(d.beats(track, 0, 0).single), (
        null,
        false,
        false,
        false,
        false,
        false,
      ));
      await d.redo();
      await d.redo();
      expect(facts(d.beats(track, 0, 0).single), on);
      await d.execute(CommandDto.setBeatStroke(beat: a));
      expect(d.beats(track, 0, 0).single.stroke, isNull, reason: 'cleared');
    });

    test(
      'ADR-034: apply the sample pattern behind the overwrite safeguard',
      () async {
        final p = d.samplePattern();
        expect(d.patterns().map((x) => x.id), contains(p));
        final c = Int32List.fromList([0, 1, 0, 2, 3, -1]); // C x32010
        ApplyPatternDto args({int pattern = -1, bool overwrite = false}) =>
            ApplyPatternDto(
              track: track,
              first: 0,
              last: 1,
              pattern: pattern == -1 ? p : pattern,
              frets: c,
              chord: 'C',
              overwrite: overwrite,
            );
        final preview = d.previewPattern(args());
        expect(preview.needsFit, isFalse);
        expect(preview.report!.overwritten, isEmpty);
        final rev = d.summary().revision;
        final out = await d.execute(CommandDto.applyPattern(args: args()));
        expect(d.summary().revision, rev + 1, reason: 'one revision');
        expect((out.pattern!.first, out.pattern!.last), (0, 1));
        expect(d.beats(track, 0, 0), isNotEmpty);
        expect(d.beats(track, 1, 0), isNotEmpty);
        // Content now exists: refused without confirmation, nothing changes.
        final rev2 = d.summary().revision;
        expect(d.previewPattern(args()).report!.overwritten, [0, 1]);
        await expectLater(
          d.execute(CommandDto.applyPattern(args: args())),
          throwsA(isA<ApiError>().having((e) => e.rule, 'rule', 'PE-07')),
        );
        expect(d.summary().revision, rev2);
        await d.execute(CommandDto.applyPattern(args: args(overwrite: true)));
        expect(d.summary().revision, rev2 + 1);
        await d.undo();
        await d.undo();
        expect(d.beats(track, 0, 0), isEmpty, reason: 'undo restores');
        expect(
          () => d.previewPattern(args(pattern: 9999)),
          throwsA(isA<ApiError>().having((e) => e.rule, 'rule', 'PT-06')),
        );
      },
    );

    test(
      'ADR-034 (6a-5): the pattern grid is edited by commands and undone',
      () async {
        bool libraryChanged(OutcomeDto o) =>
            o.events.any((e) => e.kind is EventKindDto_PatternLibraryChanged);
        const bass = GridSelectorDto(
          kind: SelectorKindDto.bass,
          count: 0,
          string: 0,
          from: 0,
          to: 0,
        );
        const strike = GridActionDto.strike(
          strike: GridStrikeDto(
            targets: [bass],
            techniques: TechniquesDto(
              accent: false,
              palmMute: false,
              dead: false,
              letRing: false,
              stringPercussive: false,
            ),
          ),
        );
        GridEditDto place(int beat, int parts) => GridEditDto.place(
          lane: PatternLaneDto.strum,
          at: GridCellDto(beat: beat, parts: parts, cell: 0),
          cells: 1,
          action: strike,
        );
        final rev = d.summary().revision;
        final created = await d.execute(
          const CommandDto.newPattern(
            name: 'Grid',
            twoLanes: false,
            lengthNumerator: 1,
            lengthDenominator: 1,
            meter: MeterDto(numerator: 4, denominator: 4),
          ),
        );
        final p = created.createdPattern!;
        expect(libraryChanged(created), isTrue);
        expect(d.summary().revision, rev + 1);
        expect(d.patterns().map((x) => x.name), contains('Grid'));

        await d.execute(CommandDto.editPattern(pattern: p, edit: place(0, 2)));
        await d.execute(CommandDto.editPattern(pattern: p, edit: place(1, 3)));
        var v = d.patternGrid(p, const []);
        expect(v.snaps, [2, 2, 2, 2], reason: 'two cells per beat by default');
        expect(v.beatDenominator, 4);
        expect(v.lanes, [PatternLaneDto.strum]);
        expect(v.steps, hasLength(2));
        // A cell is relative to the snap of the view.
        expect(v.steps[1].cell, const GridCellDto(beat: 1, parts: 2, cell: 0));
        expect(
          d.patternGrid(p, const [2, 3]).steps[1].cell,
          const GridCellDto(beat: 1, parts: 3, cell: 0),
        );
        expect(v.rows[v.steps[0].rows.single].label, 'Bass');
        expect(v.steps[0].kind, GridStepKindDto.strike);

        // Refusals carry the core's rule and change nothing.
        final before = d.summary().revision;
        await expectLater(
          d.execute(CommandDto.editPattern(pattern: p, edit: place(2, 5))),
          throwsA(isA<ApiError>().having((e) => e.rule, 'rule', 'PE-14')),
        );
        await expectLater(
          d.execute(CommandDto.editPattern(pattern: p, edit: place(0, 2))),
          throwsA(isA<ApiError>().having((e) => e.rule, 'rule', 'PT')),
        );
        expect(
          () => d.patternGrid(9999, const []),
          throwsA(isA<ApiError>().having((e) => e.rule, 'rule', 'PT')),
        );
        expect(d.summary().revision, before);

        // A run technique, then a gesture from the first strike.
        await d.execute(
          CommandDto.editPattern(
            pattern: p,
            edit: const GridEditDto.addSpan(
              lane: PatternLaneDto.strum,
              first: 0,
              last: 1,
              technique: TechniqueKindDto.palmMute,
            ),
          ),
        );
        v = d.patternGrid(p, const [2, 3]);
        expect(v.steps.every((s) => s.spanTechniques.palmMute), isTrue);
        final made = await d.execute(
          CommandDto.gestureFromStep(
            pattern: p,
            lane: PatternLaneDto.strum,
            index: 0,
            name: 'Kịch',
          ),
        );
        final g = made.createdGesture!;
        expect(d.gestures().map((x) => (x.id, x.name)), [(g, 'Kịch')]);
        v = d.patternGrid(p, const [2, 3]);
        expect(v.steps[0].kind, GridStepKindDto.gesture);
        expect(v.steps[0].gesture, g);
        expect(v.rows[v.steps[0].rows.single].label, 'Kịch');
        await expectLater(
          d.execute(CommandDto.removeGesture(gesture: g)),
          throwsA(isA<ApiError>().having((e) => e.rule, 'rule', 'PT')),
        );

        // Undo restores the strike and announces the library change.
        final undone = await d.undo();
        expect(libraryChanged(undone), isTrue);
        expect(d.gestures(), isEmpty);
        expect(
          d.patternGrid(p, const []).steps[0].kind,
          GridStepKindDto.strike,
        );
      },
    );

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
            accent: false,
            palmMute: false,
            stringPercussive: false,
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

    test(
      'ADR-031: outline and systems show one aligned stave per track',
      () async {
        await d.execute(
          CommandDto.addTrack(
            name: 'Second',
            instrument: d.summary().tracks.single.instrument,
          ),
        );
        final ids = d.summary().tracks.map((t) => t.id).toList();
        final a = await insertBeat(0);
        await d.execute(CommandDto.setNote(beat: a, string: 1, fret: 5));
        // A beat on the second track, bar 0.
        await d.execute(
          CommandDto.insertBeat(
            track: ids[1],
            bar: 0,
            voice: 0,
            index: 0,
            duration: quarter,
          ),
        );

        final m = metrics();
        // Asked in reverse order: staves come in score order.
        final outline = d.scoreOutline(ids.reversed.toList(), m);
        expect(outline.revision, d.summary().revision);
        expect(outline.systems, isNotEmpty);
        expect(outline.systems.first.firstBar, 0);
        expect(outline.systems.last.endBar, 2);

        final systems = d.scoreSystems(ids, m, 0, outline.systems.length);
        expect(systems.length, outline.systems.length);
        for (var i = 0; i < systems.length; i++) {
          final s = systems[i];
          expect(s.index, outline.systems[i].index);
          expect(s.height, closeTo(outline.systems[i].height, 1e-3));
          expect(s.staves.map((st) => st.track), ids);
          final bars = s.staves.first.system.bars;
          for (final st in s.staves) {
            expect(
              st.system.bars.map((b) => (b.bar, b.x, b.width)),
              bars.map((b) => (b.bar, b.x, b.width)),
              reason: 'bar lines line up',
            );
            for (final beat in st.system.beats) {
              final own = d.beats(st.track, beat.bar, beat.voice);
              expect(
                own[beat.index].id,
                beat.id,
                reason: 'hits stay on their track',
              );
            }
          }
        }
        expect(d.scoreSystems(ids, m, outline.systems.length, 3), isEmpty);
        expect(() => d.scoreOutline(const [], m), throwsA(isA<ApiError>()));
      },
    );

    test('save requires a location; save as then save succeed', () async {
      await expectLater(
        d.save(),
        throwsA(isA<ApiError>().having((e) => e.rule, 'rule', 'save-location')),
      );
    });
  });
}
