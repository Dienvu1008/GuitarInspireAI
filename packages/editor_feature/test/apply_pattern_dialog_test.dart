import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

/// The minimal apply dialog of ADR-034 (slice 6a-3): preview, fit when the core needs
/// one, confirmation before overwriting, one command, standard keyboard operation.

late FakeDocumentFactory _factory;

ProviderContainer _container() {
  _factory = FakeDocumentFactory();
  return ProviderContainer(
    overrides: [documentFactoryProvider.overrideWithValue(_factory)],
  );
}

Widget _host(ProviderContainer c) => UncontrolledProviderScope(
  container: c,
  child: MaterialApp(
    home: Scaffold(
      body: Builder(
        builder: (context) => TextButton(
          key: const ValueKey('open'),
          onPressed: () => showApplyPatternDialog(context),
          child: const Text('Open'),
        ),
      ),
    ),
  ),
);

Future<void> _open(WidgetTester tester, ProviderContainer c) async {
  await tester.pumpWidget(_host(c));
  await tester.tap(find.byKey(const ValueKey('open')));
  await tester.pumpAndSettle();
}

Finder _k(String key) => find.byKey(ValueKey(key));

Future<void> _range(WidgetTester tester, String first, String last) async {
  await tester.enterText(_k('apply-pattern-first'), first);
  await tester.enterText(_k('apply-pattern-last'), last);
  await tester.pump();
}

Future<void> _tapAndSettle(WidgetTester tester, String key) async {
  await tester.tap(_k(key));
  await tester.pumpAndSettle();
}

int _revision(ProviderContainer c) => c.read(editorProvider).summary.revision;

List<BeatDto> _bar(ProviderContainer c, int bar) {
  final s = c.read(editorProvider);
  return s.service.beats(s.activeTrack, bar, 0);
}

/// Beats as comparable text (BeatDto compares its note list by reference).
List<String> _shape(List<BeatDto> beats) => [
  for (final b in beats)
    '${b.stroke}:${b.notes.map((n) => '${n.string}/${n.fret}').join(',')}',
];

bool _applyEnabled(WidgetTester tester) =>
    tester.widget<FilledButton>(_k('apply-pattern-apply')).onPressed != null;

String _text(WidgetTester tester, String key) =>
    tester.widget<Text>(_k(key)).data!;

void main() {
  testWidgets('previews and applies the sample pattern with one command', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    expect(find.text('Apply pattern'), findsOneWidget);
    expect(
      find.text('Boom-chick'),
      findsOneWidget,
      reason: 'the sample pattern',
    );
    expect(_applyEnabled(tester), isFalse, reason: 'no preview yet');
    await _tapAndSettle(tester, 'apply-pattern-find');
    expect(
      find.text('x32010'),
      findsOneWidget,
      reason: 'voicings from the core',
    );
    await _range(tester, '1', '2');
    await _tapAndSettle(tester, 'apply-pattern-preview');
    expect(
      _text(tester, 'apply-pattern-summary'),
      '2 repetition(s), 8 note(s) on bars 1 to 2; voice 1.',
    );
    final rev = _revision(c);
    await _tapAndSettle(tester, 'apply-pattern-apply');
    expect(find.text('Apply pattern'), findsNothing, reason: 'closed');
    expect(_revision(c), rev + 1, reason: 'one command');
    expect(_bar(c, 0), isNotEmpty);
    expect(_bar(c, 1), isNotEmpty);
    expect(_bar(c, 2), isEmpty);
  });

  testWidgets('asks before overwriting; Cancel changes nothing', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    final s = c.read(editorProvider);
    await c
        .read(editorProvider.notifier)
        .execute(
          CommandDto.insertBeat(
            track: s.activeTrack,
            bar: 1,
            voice: 0,
            index: 0,
            duration: const DurationDto(base: 2, dots: 0),
          ),
        );
    await _open(tester, c);
    await _tapAndSettle(tester, 'apply-pattern-find');
    await _range(tester, '1', '3');
    await _tapAndSettle(tester, 'apply-pattern-preview');
    expect(
      _text(tester, 'apply-pattern-summary'),
      contains('replaces the content of bars 2'),
    );
    final rev = _revision(c);
    await _tapAndSettle(tester, 'apply-pattern-apply');
    expect(find.text('Overwrite existing content?'), findsOneWidget);
    expect(find.textContaining('Bars 2 already contain beats'), findsOneWidget);
    await tester.tap(find.text('Cancel').last);
    await tester.pumpAndSettle();
    expect(_revision(c), rev, reason: 'cancel sends nothing');
    expect(find.text('Apply pattern'), findsOneWidget, reason: 'still open');
    await _tapAndSettle(tester, 'apply-pattern-apply');
    await _tapAndSettle(tester, 'apply-pattern-overwrite');
    expect(_revision(c), rev + 1);
    expect(find.text('Apply pattern'), findsNothing);
  });

  testWidgets('asks for a fit when the core needs one', (tester) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    _factory.created.first.patternNeedsFit = true;
    await _tapAndSettle(tester, 'apply-pattern-find');
    await _range(tester, '1', '2');
    await _tapAndSettle(tester, 'apply-pattern-preview');
    expect(
      find.textContaining('not a whole number of repetitions'),
      findsOneWidget,
    );
    expect(_k('apply-pattern-fit'), findsOneWidget);
    expect(_applyEnabled(tester), isFalse, reason: 'no fit chosen yet');
    await tester.tap(find.text('Repeat and cut'));
    await tester.pumpAndSettle();
    expect(_k('apply-pattern-summary'), findsOneWidget);
    expect(_applyEnabled(tester), isTrue);
    final rev = _revision(c);
    await _tapAndSettle(tester, 'apply-pattern-apply');
    expect(_revision(c), rev + 1);
  });

  testWidgets('a range change clears the preview and the fit', (tester) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    _factory.created.first.patternNeedsFit = true;
    await _tapAndSettle(tester, 'apply-pattern-find');
    await _range(tester, '1', '2');
    await _tapAndSettle(tester, 'apply-pattern-preview');
    await tester.tap(find.text('Whole repetitions only'));
    await tester.pumpAndSettle();
    expect(_applyEnabled(tester), isTrue);
    await tester.enterText(_k('apply-pattern-last'), '3');
    await tester.pump();
    expect(_applyEnabled(tester), isFalse);
    expect(_k('apply-pattern-fit'), findsNothing);
    // No remembered choice (decision 13.6): the next preview asks again.
    await _tapAndSettle(tester, 'apply-pattern-preview');
    expect(_k('apply-pattern-fit'), findsOneWidget);
    expect(_applyEnabled(tester), isFalse);
  });

  /// An `insertBeat` of a rest (a beat without notes) in voice 1 of [bar].
  Future<void> rest(ProviderContainer c, int bar) async {
    final s = c.read(editorProvider);
    await s.service.execute(
      CommandDto.insertBeat(
        track: s.activeTrack,
        bar: bar,
        voice: 0,
        index: 0,
        duration: const DurationDto(base: 2, dots: 0),
      ),
    );
  }

  testWidgets('a voice with only rests still asks, and says so', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await rest(c, 1);
    expect(_bar(c, 1).single.notes, isEmpty, reason: 'a rest, no notes');
    await _open(tester, c);
    await _tapAndSettle(tester, 'apply-pattern-find');
    await _range(tester, '1', '2');
    await _tapAndSettle(tester, 'apply-pattern-preview');
    final rev = _revision(c);
    await _tapAndSettle(tester, 'apply-pattern-apply');
    expect(
      find.text(
        'Bars 2 already contain beats in the voices the pattern writes, '
        'including rests. Replace them?',
      ),
      findsOneWidget,
    );
    expect(_revision(c), rev, reason: 'nothing sent before the answer');
  });

  testWidgets('content that appears after the preview is refused as stale', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    await _tapAndSettle(tester, 'apply-pattern-find');
    await _range(tester, '1', '2');
    await _tapAndSettle(tester, 'apply-pattern-preview');
    await rest(c, 0); // appears after the preview
    final service = c.read(editorProvider).service;
    final rev = service.summary().revision;
    await _tapAndSettle(tester, 'apply-pattern-apply');
    expect(service.summary().revision, rev, reason: 'the core refused (E-01)');
    expect(
      _text(tester, 'apply-pattern-error'),
      contains('changed after the preview'),
    );
    expect(_applyEnabled(tester), isFalse, reason: 'the preview was dropped');
    expect(find.text('Apply pattern'), findsOneWidget);
  });

  testWidgets('previewing again after a change uses the new revision', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    await _tapAndSettle(tester, 'apply-pattern-find');
    await _range(tester, '1', '2');
    await _tapAndSettle(tester, 'apply-pattern-preview');
    await rest(c, 3); // outside the range, but the revision moves on
    await _tapAndSettle(tester, 'apply-pattern-preview');
    final service = c.read(editorProvider).service;
    final rev = service.summary().revision;
    await _tapAndSettle(tester, 'apply-pattern-apply');
    expect(service.summary().revision, rev + 1, reason: 'applied, not stale');
    expect(find.text('Apply pattern'), findsNothing);
  });
  testWidgets(
    'a confirmed overwrite is refused when the range changed after the preview',
    (tester) async {
      // The dangerous sequence: the preview finds content, the user is asked, another
      // command changes the same range, the user presses Overwrite.
      final c = _container();
      addTearDown(c.dispose);
      await rest(c, 1);
      await _open(tester, c);
      await _tapAndSettle(tester, 'apply-pattern-find');
      await _range(tester, '1', '3');
      await _tapAndSettle(tester, 'apply-pattern-preview');
      await _tapAndSettle(tester, 'apply-pattern-apply');
      expect(find.text('Overwrite existing content?'), findsOneWidget);
      // While the confirmation is open, a note is added to the bar to be replaced.
      final s = c.read(editorProvider);
      final beat = s.service.beats(s.activeTrack, 1, 0).single;
      await s.service.execute(
        CommandDto.setNote(
          beat: BeatAddressDto(
            track: s.activeTrack,
            bar: 1,
            voice: 0,
            beat: beat.id,
          ),
          string: 1,
          fret: 7,
        ),
      );
      final changed = _shape(_bar(c, 1));
      final rev = s.service.summary().revision;
      await _tapAndSettle(tester, 'apply-pattern-overwrite');
      // The old confirmation overwrote nothing, and was not sent again.
      expect(s.service.summary().revision, rev);
      expect(_shape(_bar(c, 1)), changed);
      expect(_bar(c, 1).single.notes.single.fret, 7);
      expect(_bar(c, 0), isEmpty);
      expect(find.text('Overwrite existing content?'), findsNothing);
      expect(find.text('Apply pattern'), findsOneWidget, reason: 'still open');
      expect(_text(tester, 'apply-pattern-error'), contains('Preview again'));
      expect(_applyEnabled(tester), isFalse, reason: 'must preview again');
      // Previewing again shows the new content; confirming again applies.
      await _tapAndSettle(tester, 'apply-pattern-preview');
      await _tapAndSettle(tester, 'apply-pattern-apply');
      await _tapAndSettle(tester, 'apply-pattern-overwrite');
      expect(s.service.summary().revision, rev + 1);
      expect(_bar(c, 1), hasLength(4));
      expect(find.text('Apply pattern'), findsNothing);
    },
  );
  testWidgets('invalid inputs are reported and nothing is applied', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    await _tapAndSettle(tester, 'apply-pattern-preview');
    expect(
      _text(tester, 'apply-pattern-error'),
      'Find the voicings of a chord first.',
    );
    await _tapAndSettle(tester, 'apply-pattern-find');
    await _range(tester, '0', '2');
    await _tapAndSettle(tester, 'apply-pattern-preview');
    expect(_text(tester, 'apply-pattern-error'), contains('between 1 and 4'));
    await _range(tester, '3', '2');
    await _tapAndSettle(tester, 'apply-pattern-preview');
    expect(_k('apply-pattern-error'), findsOneWidget);
    await tester.enterText(_k('apply-pattern-chord'), '');
    await _tapAndSettle(tester, 'apply-pattern-find');
    expect(_text(tester, 'apply-pattern-error'), 'empty chord symbol');
    expect(_applyEnabled(tester), isFalse);
  });

  testWidgets('works with Tab, Enter and Space', (tester) async {
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    await _range(tester, '1', '1');
    Future<void> tabTo(Finder target) async {
      final element = tester.element(target);
      for (var i = 0; i < 30; i++) {
        await tester.sendKeyEvent(LogicalKeyboardKey.tab);
        await tester.pump();
        final focused = FocusManager.instance.primaryFocus?.context;
        if (focused == null) continue;
        var inside = focused == element;
        focused.visitAncestorElements((e) {
          inside = inside || e == element;
          return !inside;
        });
        if (inside) return;
      }
      fail('Tab never reached $target');
    }

    await tabTo(_k('apply-pattern-find'));
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    expect(find.text('x32010'), findsOneWidget);
    await tabTo(_k('apply-pattern-preview'));
    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pumpAndSettle();
    expect(_k('apply-pattern-summary'), findsOneWidget);
    final rev = _revision(c);
    await tabTo(_k('apply-pattern-apply'));
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    expect(_revision(c), rev + 1);
  });

  testWidgets('fields and messages have semantic labels', (tester) async {
    final handle = tester.ensureSemantics();
    final c = _container();
    addTearDown(c.dispose);
    await _open(tester, c);
    for (final label in ['Pattern', 'Chord', 'First bar', 'Last bar']) {
      expect(find.bySemanticsLabel(RegExp(label)), findsWidgets, reason: label);
    }
    for (final label in ['Find voicings', 'Preview', 'Apply', 'Cancel']) {
      expect(find.bySemanticsLabel(label), findsWidgets, reason: label);
    }
    await _tapAndSettle(tester, 'apply-pattern-preview');
    expect(
      tester.getSemantics(_k('apply-pattern-error')),
      isSemantics(
        label: 'Find the voicings of a chord first.',
        isLiveRegion: true,
      ),
    );
    handle.dispose();
  });
}
