import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

/// ADR-035 TQ-9: the inspector shows and toggles every technique flag and the stroke
/// by mouse and by standard keyboard navigation (Tab, Space, Enter), with semantic
/// labels, and sends one command per change.

Widget _inspector(ProviderContainer c) => UncontrolledProviderScope(
  container: c,
  child: const MaterialApp(
    home: Scaffold(body: SingleChildScrollView(child: Inspector())),
  ),
);

ProviderContainer _container() => ProviderContainer(
  overrides: [documentFactoryProvider.overrideWithValue(FakeDocumentFactory())],
);

/// A quarter beat in bar 0 with a note on string 3 (when [note]), cursor on it.
Future<void> _beat(ProviderContainer c, {bool note = true}) async {
  final editor = c.read(editorProvider.notifier);
  final s = c.read(editorProvider);
  await editor.execute(
    CommandDto.insertBeat(
      track: s.activeTrack,
      bar: 0,
      voice: 0,
      index: 0,
      duration: const DurationDto(base: 2, dots: 0),
    ),
  );
  final id = s.service.beats(s.activeTrack, 0, 0).single.id;
  if (note) {
    await editor.execute(
      CommandDto.setNote(
        beat: BeatAddressDto(track: s.activeTrack, bar: 0, voice: 0, beat: id),
        string: 3,
        fret: 7,
      ),
    );
  }
  editor.moveCursor(const EditorCursor(bar: 0, voice: 0, string: 3, beat: 0));
}

BeatDto _current(ProviderContainer c) {
  final s = c.read(editorProvider);
  return s.service.beats(s.activeTrack, 0, 0).single;
}

int _revision(ProviderContainer c) => c.read(editorProvider).summary.revision;

/// The note flags as (accent, palm mute, string-percussive, dead, let ring).
(bool, bool, bool, bool, bool) _flags(ProviderContainer c) {
  final n = _current(c).notes.single;
  return (n.accent, n.palmMute, n.stringPercussive, n.dead, n.letRing);
}

Finder _tile(String title) => find.widgetWithText(CheckboxListTile, title);

/// Presses Tab until the focused widget is inside [target]; fails after 40 presses.
Future<void> _tabTo(WidgetTester tester, Finder target) async {
  final element = tester.element(target);
  for (var i = 0; i < 40; i++) {
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    final focused = FocusManager.instance.primaryFocus?.context;
    if (focused == null) continue;
    var inside = false;
    focused.visitAncestorElements((e) {
      inside = e == element;
      return !inside;
    });
    if (inside || focused == element) return;
  }
  fail('Tab never reached $target');
}

void main() {
  testWidgets(
    'every new flag is shown and toggled by mouse, one command each',
    (tester) async {
      final c = _container();
      addTearDown(c.dispose);
      await _beat(c);
      await tester.pumpWidget(_inspector(c));
      for (final title in ['Accent', 'Palm mute', 'String-percussive']) {
        expect(_tile(title), findsOneWidget, reason: title);
      }
      expect(_flags(c), (false, false, false, false, false));

      final steps = <(String, (bool, bool, bool, bool, bool))>[
        ('Accent', (true, false, false, false, false)),
        ('Palm mute', (true, true, false, false, false)),
        ('String-percussive', (true, true, true, false, false)),
        ('Accent', (false, true, true, false, false)),
        ('Palm mute', (false, false, true, false, false)),
        ('String-percussive', (false, false, false, false, false)),
      ];
      for (final (title, expected) in steps) {
        final rev = _revision(c);
        await tester.tap(_tile(title));
        await tester.pumpAndSettle();
        expect(_flags(c), expected, reason: title);
        expect(_revision(c), rev + 1, reason: '$title: one command');
        final tile = tester.widget<CheckboxListTile>(_tile(title));
        final n = _current(c).notes.single;
        final shown = switch (title) {
          'Accent' => n.accent,
          'Palm mute' => n.palmMute,
          _ => n.stringPercussive,
        };
        expect(tile.value, shown, reason: '$title shows the stored value');
      }
    },
  );

  testWidgets('the existing flags are sent unchanged with a new one', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await _beat(c);
    await tester.pumpWidget(_inspector(c));
    await tester.tap(_tile('Dead note'));
    await tester.pumpAndSettle();
    await tester.tap(_tile('Let ring'));
    await tester.pumpAndSettle();
    await tester.tap(_tile('Palm mute'));
    await tester.pumpAndSettle();
    expect(_flags(c), (false, true, false, true, true));
  });

  testWidgets('the stroke is chosen by mouse, also on a rest', (tester) async {
    final c = _container();
    addTearDown(c.dispose);
    await _beat(c, note: false);
    await tester.pumpWidget(_inspector(c));
    expect(_current(c).notes, isEmpty);
    expect(_tile('Accent'), findsNothing, reason: 'no note, no note flags');
    for (final (label, expected) in [
      ('Down', StrokeDto.down),
      ('Up', StrokeDto.up),
      ('None', null),
    ]) {
      final rev = _revision(c);
      await tester.tap(find.text(label));
      await tester.pumpAndSettle();
      expect(_current(c).stroke, expected, reason: label);
      expect(_revision(c), rev + 1, reason: '$label: one command');
    }
    // Choosing the current value sends nothing.
    final rev = _revision(c);
    await tester.tap(find.text('None'));
    await tester.pumpAndSettle();
    expect(_revision(c), rev);
  });

  testWidgets('Tab reaches every control; Space and Enter activate them', (
    tester,
  ) async {
    final c = _container();
    addTearDown(c.dispose);
    await _beat(c);
    await tester.pumpWidget(_inspector(c));

    for (final (title, key) in [
      ('Accent', LogicalKeyboardKey.space),
      ('Palm mute', LogicalKeyboardKey.enter),
      ('String-percussive', LogicalKeyboardKey.space),
    ]) {
      FocusManager.instance.primaryFocus?.unfocus();
      await tester.pump();
      await _tabTo(tester, _tile(title));
      final rev = _revision(c);
      await tester.sendKeyEvent(key);
      await tester.pumpAndSettle();
      expect(_revision(c), rev + 1, reason: '$title by $key');
    }
    expect(_flags(c), (true, true, true, false, false));

    for (final (label, key, expected) in [
      ('Up', LogicalKeyboardKey.enter, StrokeDto.up),
      ('Down', LogicalKeyboardKey.space, StrokeDto.down),
    ]) {
      FocusManager.instance.primaryFocus?.unfocus();
      await tester.pump();
      await _tabTo(
        tester,
        find.ancestor(
          of: find.text(label),
          matching: find.byWidgetPredicate((w) => w is ButtonStyleButton),
        ),
      );
      await tester.sendKeyEvent(key);
      await tester.pumpAndSettle();
      expect(_current(c).stroke, expected, reason: '$label by $key');
    }
  });

  testWidgets('controls have semantic labels and states', (tester) async {
    final handle = tester.ensureSemantics();
    final c = _container();
    addTearDown(c.dispose);
    await _beat(c);
    await tester.pumpWidget(_inspector(c));
    await tester.tap(_tile('Palm mute'));
    await tester.pumpAndSettle();
    expect(
      tester.getSemantics(_tile('Accent')),
      isSemantics(
        label: 'Accent',
        hasCheckedState: true,
        isChecked: false,
        isEnabled: true,
        hasTapAction: true,
      ),
    );
    expect(
      tester.getSemantics(_tile('Palm mute')),
      isSemantics(label: 'Palm mute', hasCheckedState: true, isChecked: true),
    );
    expect(
      tester.getSemantics(_tile('String-percussive')),
      isSemantics(hasCheckedState: true, isChecked: false),
    );
    for (final label in ['Down', 'None', 'Up']) {
      expect(find.bySemanticsLabel(RegExp(label)), findsWidgets, reason: label);
    }
    handle.dispose();
  });
}
