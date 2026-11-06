// ADR-030 section 3, criteria C6, C7 and C9: the "save changes?" prompt on window
// close (through the framework's exit request), on New, and the session marker.
import 'dart:ui' show AppExitResponse;

import 'package:app_platform/app_platform.dart';
import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart' show LogicalKeyboardKey;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:guitar_inspire/src/close_guard.dart';
import 'package:guitar_inspire/src/shell.dart';
import 'package:guitar_inspire/src/startup.dart';

const _dir = 'recovery-dir';

final class _Paths implements AppPaths {
  const _Paths();
  @override
  Future<String> recoveryDirectory() async => _dir;
}

final class _Dialogs implements FileDialogs {
  String? savePath;
  int saveAsks = 0;
  @override
  Future<String?> pickProjectToOpen() async => null;
  @override
  Future<String?> pickProjectToSave({required String suggestedName}) async {
    saveAsks++;
    return savePath;
  }

  @override
  Future<List<int>?> pickMidiToImport() async => null;
  @override
  Future<String?> saveMidi(
    List<int> bytes, {
    required String suggestedName,
  }) async => null;
}

Future<ProviderContainer> _start(
  WidgetTester tester,
  FakeDocumentFactory f, [
  _Dialogs? dialogs,
]) async {
  tester.view.physicalSize = const Size(1400, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        documentFactoryProvider.overrideWithValue(f),
        appPathsProvider.overrideWithValue(const _Paths()),
        fileDialogsProvider.overrideWithValue(dialogs ?? _Dialogs()),
        ...closeOverrides(),
      ],
      child: const StartupGate(),
    ),
  );
  for (var i = 0; i < 5; i++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
  return ProviderScope.containerOf(tester.element(find.byType(Shell)));
}

Future<void> _edit(ProviderContainer c) async {
  final s = c.read(editorProvider);
  await c
      .read(editorProvider.notifier)
      .execute(
        CommandDto.insertBeat(
          track: s.activeTrack,
          bar: 0,
          voice: 0,
          index: 0,
          duration: const DurationDto(base: 2, dots: 0),
        ),
      );
}

/// Sends the exit request the engine sends for the window close button, and returns
/// the framework's answer once the prompt (if any) is resolved by [answer].
Future<AppExitResponse> _close(
  WidgetTester tester, {
  String? answer,
}) async {
  final response = tester.binding.handleRequestAppExit();
  await tester.pump();
  await tester.pump();
  if (answer != null) {
    expect(find.text('Save changes?'), findsOneWidget);
    await tester.tap(find.byKey(ValueKey(answer)));
    await tester.pump();
    await tester.pump();
  }
  return response;
}

void main() {
  testWidgets(
    'C6/C7: an untouched new document closes without asking and ends the session',
    (tester) async {
      final f = FakeDocumentFactory();
      await _start(tester, f);
      expect(f.sessionMarkers, contains(_dir));

      expect(await _close(tester), AppExitResponse.exit);
      expect(find.text('Save changes?'), findsNothing);
      expect(f.sessionMarkers, isEmpty, reason: 'clean exit');
    },
  );

  testWidgets('C9: Cancel keeps the app, the autosave and the marker', (
    tester,
  ) async {
    final f = FakeDocumentFactory();
    final c = await _start(tester, f);
    await _edit(c);

    expect(
      await _close(tester, answer: 'close-cancel'),
      AppExitResponse.cancel,
    );
    final doc = f.created.first;
    expect(doc.autosaveWrites, 1, reason: 'written before asking (C9)');
    expect(doc.autosaveDiscards, 0);
    expect(f.sessionMarkers, contains(_dir));
    expect(c.read(editorProvider).summary.unsavedWork, isTrue);
  });

  testWidgets("C6/C7: Don't save discards the autosave and exits cleanly", (
    tester,
  ) async {
    final f = FakeDocumentFactory();
    final c = await _start(tester, f);
    await _edit(c);

    expect(
      await _close(tester, answer: 'close-discard'),
      AppExitResponse.exit,
    );
    expect(f.created.first.autosaveDiscards, 1);
    expect(f.sessionMarkers, isEmpty);
  });

  testWidgets('C6: Save asks for a location and exits only when saved', (
    tester,
  ) async {
    final f = FakeDocumentFactory();
    final dialogs = _Dialogs(); // savePath null: the user cancels the save dialog
    final c = await _start(tester, f, dialogs);
    await _edit(c);

    expect(
      await _close(tester, answer: 'close-save'),
      AppExitResponse.cancel,
      reason: 'a cancelled save dialog keeps the app open',
    );
    expect(dialogs.saveAsks, 1);
    expect(f.sessionMarkers, contains(_dir));

    dialogs.savePath = r'C:\songs\new.giproj';
    expect(await _close(tester, answer: 'close-save'), AppExitResponse.exit);
    expect(c.read(editorProvider).summary.dirty, isFalse);
    expect(f.sessionMarkers, isEmpty);
  });

  testWidgets('C6: Ctrl+N asks first; Cancel keeps the document', (
    tester,
  ) async {
    final f = FakeDocumentFactory();
    final c = await _start(tester, f);
    await _edit(c);
    final before = c.read(editorProvider).service;

    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyN);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.pump();
    await tester.pump();
    expect(find.text('Save changes?'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('close-cancel')));
    await tester.pump();
    await tester.pump();
    expect(c.read(editorProvider).service, same(before));

    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyN);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.pump();
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('close-discard')));
    await tester.pump();
    await tester.pump();
    expect(c.read(editorProvider).service, isNot(same(before)));
    expect(c.read(editorProvider).summary.unsavedWork, isFalse);
    // The session continues: New does not end it.
    expect(f.sessionMarkers, contains(_dir));
  });
}
