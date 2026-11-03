import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:editor_feature/src/tab_view.dart' show SystemHits;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

Widget _editor(ProviderContainer container) => UncontrolledProviderScope(
  container: container,
  child: const MaterialApp(
    home: Scaffold(
      body: Row(
        children: [
          SizedBox(width: 200, child: TrackList()),
          Expanded(child: TabView()),
        ],
      ),
    ),
  ),
);

ProviderContainer _container() => ProviderContainer(
  overrides: [documentFactoryProvider.overrideWithValue(FakeDocumentFactory())],
);

Future<BeatAddressDto> _beatWithNote(ProviderContainer c, {int bar = 0}) async {
  final editor = c.read(editorProvider.notifier);
  final s = c.read(editorProvider);
  await editor.execute(
    CommandDto.insertBeat(
      track: s.activeTrack,
      bar: bar,
      voice: 0,
      index: 0,
      duration: const DurationDto(base: 2, dots: 0),
    ),
  );
  final beat = s.service.beats(s.activeTrack, bar, 0).single;
  final address = BeatAddressDto(
    track: s.activeTrack,
    bar: bar,
    voice: 0,
    beat: beat.id,
  );
  await editor.execute(CommandDto.setNote(beat: address, string: 3, fret: 7));
  return address;
}

void main() {
  testWidgets('renders the TAB of the active track and refreshes after edits', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await tester.pumpWidget(_editor(c));
    expect(find.byKey(const ValueKey('system-0')), findsOneWidget);
    expect(find.text('Guitar'), findsOneWidget);

    await _beatWithNote(c);
    await tester.pump();
    expect(c.read(editorProvider).summary.revision, 2);
    expect(c.read(editorProvider).title, startsWith('* '));
  });

  testWidgets('tapping a beat places the cursor on that beat and string', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await tester.pumpWidget(_editor(c));
    await _beatWithNote(c, bar: 1);
    await tester.pump();

    final view = find.byType(TabView);
    final width = tester.getSize(view).width;
    final s = c.read(editorProvider);
    final layout = s.service.tabLayout(
      s.activeTrack,
      measureTabMetrics(width: width),
    );
    final system = layout.systems.single;
    final slot = system.beats.indexWhere((b) => b.bar == 1);
    var hit = -1;
    for (var i = 0; i < system.hitCount; i++) {
      if (system.hitMeta[i * 4 + 3] == slot && system.hitMeta[i * 4 + 1] == 3) {
        hit = i;
      }
    }
    expect(hit, isNonNegative);
    final origin = tester.getTopLeft(find.byKey(const ValueKey('system-0')));
    await tester.tapAt(origin + system.hitRect(hit).center);
    await tester.pump();
    expect(
      c.read(editorProvider).cursor,
      const EditorCursor(bar: 1, voice: 0, string: 3, beat: 0),
    );
  });

  testWidgets('selecting a track switches the active TAB track', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await tester.pumpWidget(_editor(c));
    final s = c.read(editorProvider);
    await c
        .read(editorProvider.notifier)
        .execute(
          CommandDto.addTrack(
            name: 'Bass',
            instrument: s.summary.tracks.single.instrument,
          ),
        );
    await tester.pump();
    await tester.tap(find.text('Bass'));
    await tester.pump();
    final after = c.read(editorProvider);
    expect(after.activeTrack, after.summary.tracks[1].id);
    expect(after.cursor, EditorCursor.start);
  });

  testWidgets('a rejected command becomes a status message', (tester) async {
    final c = _container();
    addTearDown(c.dispose);
    await tester.pumpWidget(_editor(c));
    final ok = await c
        .read(editorProvider.notifier)
        .execute(
          const CommandDto.removeBeat(
            beat: BeatAddressDto(track: 1, bar: 0, voice: 0, beat: 999),
          ),
        );
    expect(ok, isFalse);
    expect(c.read(editorProvider).message, startsWith('not-found'));
  });
}
