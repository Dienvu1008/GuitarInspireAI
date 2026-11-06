import 'dart:typed_data';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:tools_feature/tools_feature.dart';

ChordResultDto _result(String symbol) => ChordResultDto(
  symbol: symbol,
  tones: const ['C', 'E', 'G'],
  voicings: [
    VoicingDto(
      frets: Int32List.fromList([0, 1, 0, 2, 3, -1]),
      chartText: 'x32010',
      fingers: 3,
      lowestFret: 1,
      highestFret: 3,
    ),
  ],
);

Widget _app(List<Override> overrides) => ProviderScope(
  overrides: overrides,
  child: const MaterialApp(home: Scaffold(body: ChordFinderView())),
);

void main() {
  testWidgets('finding a chord lists voicings', (tester) async {
    await tester.pumpWidget(
      _app([
        chordLookupProvider.overrideWithValue(
          (symbol, {int max = 5}) => _result(symbol),
        ),
      ]),
    );
    await tester.enterText(find.byKey(const ValueKey('chord-input')), 'C');
    await tester.tap(find.byKey(const ValueKey('chord-find')));
    await tester.pump();

    expect(find.byKey(const ValueKey('voicing-list')), findsOneWidget);
    expect(find.text('x32010'), findsOneWidget);
    expect(find.byType(FretboardView), findsOneWidget);
    // Insert is disabled when no insertion is wired.
    final insert = tester.widget<OutlinedButton>(
      find.byKey(const ValueKey('insert-0')),
    );
    expect(insert.onPressed, isNull);
  });

  testWidgets('a bad chord symbol shows a message, no crash', (tester) async {
    await tester.pumpWidget(
      _app([
        chordLookupProvider.overrideWithValue((symbol, {int max = 5}) {
          throw ApiError(rule: 'chord', message: 'not a chord: $symbol');
        }),
      ]),
    );
    await tester.enterText(find.byKey(const ValueKey('chord-input')), 'zzz');
    await tester.tap(find.byKey(const ValueKey('chord-find')));
    await tester.pump();
    expect(find.byKey(const ValueKey('chord-message')), findsOneWidget);
    expect(find.textContaining('not a chord'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('insert is enabled and fires when wired (T7)', (tester) async {
    VoicingDto? inserted;
    await tester.pumpWidget(
      _app([
        chordLookupProvider.overrideWithValue(
          (symbol, {int max = 5}) => _result(symbol),
        ),
        insertVoicingProvider.overrideWithValue((v) async {
          inserted = v;
        }),
      ]),
    );
    await tester.enterText(find.byKey(const ValueKey('chord-input')), 'C');
    await tester.tap(find.byKey(const ValueKey('chord-find')));
    await tester.pump();

    await tester.tap(find.byKey(const ValueKey('insert-0')));
    await tester.pump();
    expect(inserted, isNotNull);
    expect(inserted!.chartText, 'x32010');
  });

  testWidgets('an empty symbol clears the result', (tester) async {
    final container = ProviderContainer(
      overrides: [
        chordLookupProvider.overrideWithValue(
          (symbol, {int max = 5}) => _result(symbol),
        ),
      ],
    );
    addTearDown(container.dispose);
    final c = container.read(chordProvider.notifier);
    c.find('C');
    expect(container.read(chordProvider).result, isNotNull);
    c.find('   ');
    expect(container.read(chordProvider).result, isNull);
  });
}
