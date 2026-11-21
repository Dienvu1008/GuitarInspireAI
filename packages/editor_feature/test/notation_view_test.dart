// ADR-041 section 9: the notation stave in the TAB view (F4) and the boundary check
// (F6). Painting only; the core decides every glyph and position.
import 'dart:io';
import 'dart:ui' as ui;

import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:editor_feature/src/notation_painter.dart';
import 'package:editor_feature/src/system_painter.dart' show CursorPainter;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:ui_kit/ui_kit.dart';

Widget _editor(ProviderContainer c) => UncontrolledProviderScope(
  container: c,
  child: const MaterialApp(
    home: Scaffold(
      body: Row(
        children: [
          SizedBox(width: 260, child: TrackList()),
          Expanded(child: TabView()),
        ],
      ),
    ),
  ),
);

Future<(ProviderContainer, List<int>)> _twoTracks(WidgetTester tester) async {
  tester.view.physicalSize = const Size(1400, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final c = ProviderContainer(
    overrides: [
      documentFactoryProvider.overrideWithValue(FakeDocumentFactory()),
    ],
  );
  addTearDown(c.dispose);
  await tester.pumpWidget(_editor(c));
  final editor = c.read(editorProvider.notifier);
  final instrument = c.read(editorProvider).summary.tracks.single.instrument;
  await editor.execute(
    CommandDto.addTrack(name: 'Bass', instrument: instrument),
  );
  await tester.pump();
  return (c, c.read(editorProvider).summary.tracks.map((t) => t.id).toList());
}

Finder _notation(int track) => find.byKey(ValueKey('notation-0-$track'));
Finder _tab(int track) => find.byKey(ValueKey('stave-0-$track'));

void main() {
  testWidgets(
    'every track shows notation above its TAB when a document opens',
    (tester) async {
      final (_, ids) = await _twoTracks(tester);
      for (final t in ids) {
        expect(_notation(t), findsOneWidget);
        expect(_tab(t), findsOneWidget);
        final n = tester.getRect(_notation(t));
        final tab = tester.getRect(_tab(t));
        expect(
          n.bottom,
          closeTo(tab.top, 0.5),
          reason: 'notation right above TAB',
        );
      }
    },
  );

  testWidgets('the notation toggle hides and shows one track\'s notation', (
    tester,
  ) async {
    final (c, ids) = await _twoTracks(tester);
    await tester.tap(find.byKey(ValueKey('notation-track-${ids[1]}')));
    await tester.pump();
    expect(_notation(ids[1]), findsNothing);
    expect(_tab(ids[1]), findsOneWidget);
    expect(_notation(ids[0]), findsOneWidget);
    expect(c.read(editorProvider).notationOff, {ids[1]});
    await tester.tap(find.byKey(ValueKey('notation-track-${ids[1]}')));
    await tester.pump();
    expect(_notation(ids[1]), findsOneWidget);
  });

  // ---------------------------------------------------------------- 6c-4 (H5)

  /// Both tracks get bar 0: beat 0 a note on string 3, beat 1 a note on string 1.
  Future<void> notes(ProviderContainer c, List<int> ids, WidgetTester t) async {
    final editor = c.read(editorProvider.notifier);
    for (final track in ids) {
      for (final (k, string) in [(0, 3), (1, 1)]) {
        await editor.execute(
          CommandDto.insertBeat(
            track: track,
            bar: 0,
            voice: 0,
            index: k,
            duration: const DurationDto(base: 2, dots: 0),
          ),
        );
        final beat = c.read(editorProvider).service.beats(track, 0, 0)[k];
        await editor.execute(
          CommandDto.setNote(
            beat: BeatAddressDto(track: track, bar: 0, voice: 0, beat: beat.id),
            string: string,
            fret: 5,
          ),
        );
      }
    }
    await t.pump();
  }

  /// The notation stave's hit arrays as the view has them, and its screen origin.
  (SystemDto, Offset) staveOf(WidgetTester t, ProviderContainer c, int track) {
    final s = c.read(editorProvider);
    final width = t.getSize(find.byType(TabView)).width;
    final m = measureTabMetrics(width: width);
    s.service.scoreOutlineWith(s.shownStaves, m);
    final sys = s.service.scoreSystemsWith(s.shownStaves, m, 0, 1).single;
    final st = sys.staves.firstWhere(
      (x) => x.track == track && x.kind == StaveKind.notation,
    );
    return (st.system, t.getTopLeft(_notation(track)));
  }

  /// Centre of the region of (beat index, string); string 0 is the beat region,
  /// tapped near its bottom, away from the heads.
  Offset point(SystemDto s, int beat, int string) {
    for (var i = 0; i < s.hitMeta.length ~/ 4; i++) {
      final slot = s.hitMeta[i * 4 + 3];
      if (slot < 0 || s.beats[slot].index != beat) continue;
      if (s.hitMeta[i * 4 + 1] != string) continue;
      final (x, y, w, h) = (
        s.hitRects[i * 4],
        s.hitRects[i * 4 + 1],
        s.hitRects[i * 4 + 2],
        s.hitRects[i * 4 + 3],
      );
      return string == 0
          ? Offset(x + w / 2, y + h - 2)
          : Offset(x + w / 2, y + h / 2);
    }
    throw StateError('no region for beat $beat string $string');
  }

  testWidgets('H5: a tap on a notehead selects that beat and string', (
    tester,
  ) async {
    final (c, ids) = await _twoTracks(tester);
    await notes(c, ids, tester);
    final (s, origin) = staveOf(tester, c, ids[0]);
    await tester.tapAt(origin + point(s, 1, 1));
    await tester.pump();
    expect(
      c.read(editorProvider).cursor,
      const EditorCursor(bar: 0, voice: 0, string: 1, beat: 1),
    );
    await tester.tapAt(origin + point(s, 0, 3));
    await tester.pump();
    expect(
      c.read(editorProvider).cursor,
      const EditorCursor(bar: 0, voice: 0, string: 3, beat: 0),
    );
  });

  testWidgets('H5: a tap on a beat region keeps the string', (tester) async {
    final (c, ids) = await _twoTracks(tester);
    await notes(c, ids, tester);
    final (s, origin) = staveOf(tester, c, ids[0]);
    await tester.tapAt(origin + point(s, 0, 3));
    await tester.pump();
    await tester.tapAt(origin + point(s, 1, 0));
    await tester.pump();
    expect(
      c.read(editorProvider).cursor,
      const EditorCursor(bar: 0, voice: 0, string: 3, beat: 1),
    );
    // The cursor is painted on the notation stave over the beat region.
    final painter =
        tester
                .widget<CustomPaint>(
                  find.byKey(const ValueKey('notation-cursor-0')),
                )
                .painter!
            as CursorPainter;
    expect(painter.rect, isNotNull);
  });

  testWidgets('H5: Shift+tap extends the selection', (tester) async {
    final (c, ids) = await _twoTracks(tester);
    await notes(c, ids, tester);
    final (s, origin) = staveOf(tester, c, ids[0]);
    await tester.tapAt(origin + point(s, 0, 3));
    await tester.pump();
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.tapAt(origin + point(s, 1, 0));
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pump();
    final sel = c.read(editorProvider).selection;
    expect(sel, isNotNull);
    expect((sel!.$1.index, sel.$2.index), (0, 1));
  });

  testWidgets(
    'H5: a tap on another track\'s notation selects it and places the cursor',
    (tester) async {
      final (c, ids) = await _twoTracks(tester);
      await notes(c, ids, tester);
      expect(c.read(editorProvider).activeTrack, ids[0]);
      final (s, origin) = staveOf(tester, c, ids[1]);
      await tester.tapAt(origin + point(s, 1, 1));
      await tester.pump();
      final after = c.read(editorProvider);
      expect(after.activeTrack, ids[1]);
      expect(
        after.cursor,
        const EditorCursor(bar: 0, voice: 0, string: 1, beat: 1),
      );
    },
  );

  testWidgets('H5: a tap outside every region changes nothing', (tester) async {
    final (c, ids) = await _twoTracks(tester);
    await notes(c, ids, tester);
    final before = c.read(editorProvider).cursor;
    // The header, left of every bar.
    await tester.tapAt(
      tester.getTopLeft(_notation(ids[0])) + const Offset(4, 4),
    );
    await tester.pump();
    expect(c.read(editorProvider).cursor, before);
    expect(c.read(editorProvider).activeTrack, ids[0]);
  });

  testWidgets('the painter draws exactly the DTO\'s glyphs, in Bravura', (
    tester,
  ) async {
    final (c, ids) = await _twoTracks(tester);
    final s = c.read(editorProvider);
    final m = measureTabMetrics(width: 1000);
    s.service.scoreOutlineWith(s.shownStaves, m);
    final sys = s.service.scoreSystemsWith(s.shownStaves, m, 0, 1).single;
    final stave = sys.staves.firstWhere((x) => x.kind == StaveKind.notation);
    final painter = NotationPainter(
      staff: stave.notation,
      colors: TabColors.of(const ColorScheme.light()),
    );
    final recorder = ui.PictureRecorder();
    painter.paint(Canvas(recorder), const Size(1000, 200));
    recorder.endRecording().dispose();
    expect(painter.painted, stave.notation.glyphCodepoints.toList());
    expect(painter.painted, isNotEmpty);
    final style = musicTextStyle(size: 32);
    expect(style.fontFamily, 'packages/ui_kit/Bravura');
    expect(ids, hasLength(2));
  });

  testWidgets('another document at the same revision is laid out anew', (
    tester,
  ) async {
    // Found by the F8 screenshot tool: the view cache key had no document identity,
    // so a newly opened document with the same revision and track ids kept the old
    // layout.
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final c = ProviderContainer(
      overrides: [
        documentFactoryProvider.overrideWithValue(FakeDocumentFactory()),
      ],
    );
    addTearDown(c.dispose);
    await tester.pumpWidget(_editor(c));
    // The staff lines end where the last bar ends.
    double staffEnd() {
      final p = tester
          .widgetList<CustomPaint>(find.byType(CustomPaint))
          .map((w) => w.painter)
          .whereType<NotationPainter>()
          .first;
      return p.staff.lines[2];
    }

    final before = staffEnd();
    final first = c.read(editorProvider).summary;
    // A fresh document of a fresh factory: same revision and track ids, more bars.
    final other = FakeDocumentFactory().create(bars: 9);
    expect(other.summary().revision, first.revision);
    expect(
      other.summary().tracks.map((t) => t.id),
      first.tracks.map((t) => t.id),
    );
    c.read(editorProvider.notifier).replace(other);
    await tester.pump();
    expect(staffEnd(), isNot(closeTo(before, 0.01)));
  });

  test('F5: ui_kit declares the bundled Bravura that FF-35 checks', () {
    final pubspec = File('../ui_kit/pubspec.yaml').readAsStringSync();
    expect(pubspec, contains('family: Bravura'));
    expect(pubspec, contains('asset: fonts/bravura/Bravura.otf'));
    final font = File('../ui_kit/fonts/bravura/Bravura.otf');
    expect(font.lengthSync(), 889228, reason: 'size in PROVENANCE.md');
  });

  test('F6: no musical rule in editor_feature', () {
    // Names of music decisions the core makes (ADR-040 decision 4): Dart only paints.
    final forbidden = RegExp(
      r'\b(spell\w*|accidental\w*|semitone\w*|staffPosition\w*|diatonic\w*|'
      r'transpos\w*|keySignature\w*|alteration\w*|enharmonic\w*)\b',
      caseSensitive: false,
    );
    final hits = <String>[];
    for (final f in Directory('lib').listSync(recursive: true)) {
      if (f is! File || !f.path.endsWith('.dart')) continue;
      for (final (i, line) in f.readAsLinesSync().indexed) {
        final code = line.split('//').first;
        if (forbidden.hasMatch(code)) hits.add('${f.path}:${i + 1}: $line');
      }
    }
    expect(hits, isEmpty);
  });
}
