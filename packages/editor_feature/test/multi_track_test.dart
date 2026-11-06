// ADR-031: several tracks shown at once, one stave each; editing stays on the active
// track; clicking another stave switches to it; show and hide from the track list.
import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:editor_feature/src/system_painter.dart' show CursorPainter;
import 'package:editor_feature/src/tab_view.dart' show SystemHits;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

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

/// Three tracks: Guitar (one note), Bass (one note), Keys (empty). Returns the ids.
Future<(ProviderContainer, List<int>)> _threeTracks(WidgetTester tester) async {
  tester.view.physicalSize = const Size(1400, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final c = ProviderContainer(
    overrides: [
      documentFactoryProvider.overrideWithValue(FakeDocumentFactory()),
      cursorOverlayProvider.overrideWithValue(
        (tickToX, height) =>
            SizedBox(key: const ValueKey('play-cursor'), height: height),
      ),
    ],
  );
  addTearDown(c.dispose);
  await tester.pumpWidget(_editor(c));
  final editor = c.read(editorProvider.notifier);
  final instrument = c.read(editorProvider).summary.tracks.single.instrument;
  for (final name in ['Bass', 'Keys']) {
    await editor.execute(
      CommandDto.addTrack(name: name, instrument: instrument),
    );
  }
  final ids = c.read(editorProvider).summary.tracks.map((t) => t.id).toList();
  for (final t in ids.take(2)) {
    await editor.execute(
      CommandDto.insertBeat(
        track: t,
        bar: 0,
        voice: 0,
        index: 0,
        duration: const DurationDto(base: 2, dots: 0),
      ),
    );
  }
  await tester.pump();
  return (c, ids);
}

Finder _stave(int system, int track) =>
    find.byKey(ValueKey('stave-$system-$track'));

void main() {
  testWidgets(
    'every track is shown by default, one stave each, in score order',
    (tester) async {
      final (c, ids) = await _threeTracks(tester);
      expect(c.read(editorProvider).shownTracks, ids);
      for (final t in ids) {
        expect(_stave(0, t), findsOneWidget);
      }
      // Staves are stacked top to bottom in score order.
      final tops = [for (final t in ids) tester.getTopLeft(_stave(0, t)).dy];
      expect(tops, orderedEquals([...tops]..sort()));
      // Track names label the staves when more than one is shown.
      expect(
        find.descendant(of: find.byType(TabView), matching: find.text('Bass')),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'M4: clicking another stave makes its track active and places the cursor',
    (tester) async {
      final (c, ids) = await _threeTracks(tester);
      final s = c.read(editorProvider);
      expect(s.activeTrack, ids[0]);
      // The beat of the Bass stave on string 1, from the same layout the view shows.
      final stave = s.service
          .scoreSystems(
            s.shownTracks,
            measureTabMetrics(
              width: tester.getSize(find.byType(TabView)).width,
            ),
            0,
            1,
          )
          .single
          .staves[1];
      final system = stave.system;
      var hit = -1;
      for (var i = 0; i < system.hitCount; i++) {
        if (system.hitMeta[i * 4 + 3] >= 0 && system.hitMeta[i * 4 + 1] == 1) {
          hit = i;
        }
      }
      expect(hit, isNonNegative);
      await tester.tapAt(
        tester.getTopLeft(_stave(0, ids[1])) + system.hitRect(hit).center,
      );
      await tester.pump();
      final after = c.read(editorProvider);
      expect(after.activeTrack, ids[1]);
      expect(after.cursor, EditorCursor(bar: 0, voice: 0, string: 1, beat: 0));
      // The cursor is drawn on the active stave only.
      final cursors = tester
          .widgetList<CustomPaint>(find.byType(CustomPaint))
          .map((p) => p.painter)
          .whereType<CursorPainter>()
          .toList();
      expect(cursors, hasLength(1));
      expect(cursors.single.rect, isNotNull);
    },
  );

  testWidgets('M5: one playback cursor spans every stave of a system', (
    tester,
  ) async {
    await _threeTracks(tester);
    final cursor = find.byKey(const ValueKey('play-cursor'));
    expect(cursor, findsOneWidget);
    expect(
      tester.getSize(cursor).height,
      tester.getSize(find.byKey(const ValueKey('system-0'))).height,
    );
  });

  testWidgets(
    'hiding a track removes its stave; the active track cannot be hidden',
    (tester) async {
      final (c, ids) = await _threeTracks(tester);
      await tester.tap(find.byKey(ValueKey('show-track-${ids[1]}')));
      await tester.pump();
      expect(c.read(editorProvider).shownTracks, [ids[0], ids[2]]);
      expect(_stave(0, ids[1]), findsNothing);
      expect(
        tester
            .widget<IconButton>(find.byKey(ValueKey('show-track-${ids[0]}')))
            .onPressed,
        isNull,
        reason: 'the active track is always shown',
      );

      // Selecting a hidden track makes it active and shows it again.
      await tester.tap(
        find.descendant(
          of: find.byType(TrackList),
          matching: find.text('Bass'),
        ),
      );
      await tester.pump();
      expect(c.read(editorProvider).activeTrack, ids[1]);
      expect(_stave(0, ids[1]), findsOneWidget);

      // Showing it again from the toggle, and hiding all others, leaves one stave
      // without a name label.
      await tester.tap(find.byKey(ValueKey('show-track-${ids[0]}')));
      await tester.tap(find.byKey(ValueKey('show-track-${ids[2]}')));
      await tester.pump();
      expect(c.read(editorProvider).shownTracks, [ids[1]]);
      expect(
        find.descendant(of: find.byType(TabView), matching: find.text('Bass')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'a new document shows every track again (visibility is not kept)',
    (tester) async {
      final (c, ids) = await _threeTracks(tester);
      c.read(editorProvider.notifier).toggleTrackShown(ids[2]);
      expect(c.read(editorProvider).hiddenTracks, {ids[2]});
      await c.read(editorProvider.notifier).newDocument();
      await tester.pump();
      expect(c.read(editorProvider).hiddenTracks, isEmpty);
    },
  );
}
