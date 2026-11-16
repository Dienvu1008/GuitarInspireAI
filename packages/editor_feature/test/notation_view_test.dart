// ADR-041 section 9: the notation stave in the TAB view (F4) and the boundary check
// (F6). Painting only; the core decides every glyph and position.
import 'dart:io';
import 'dart:ui' as ui;

import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:editor_feature/src/notation_painter.dart';
import 'package:flutter/material.dart';
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

  testWidgets('a tap on a notation stave selects its track, nothing else', (
    tester,
  ) async {
    final (c, ids) = await _twoTracks(tester);
    final before = c.read(editorProvider);
    expect(before.activeTrack, ids[0]);
    await tester.tapAt(tester.getCenter(_notation(ids[1])));
    await tester.pump();
    final after = c.read(editorProvider);
    expect(after.activeTrack, ids[1]);
    expect(after.cursor, EditorCursor.start);
    // On the active track's notation, a tap moves nothing.
    await tester.tapAt(tester.getCenter(_notation(ids[1])));
    await tester.pump();
    expect(c.read(editorProvider).cursor, EditorCursor.start);
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
