import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart' show LogicalKeyboardKey;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:guitar_inspire/src/shell.dart';

void main() {
  testWidgets('shell shows menus, the track list and an untitled document', (tester) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [documentFactoryProvider.overrideWithValue(FakeDocumentFactory())],
        child: const GuitarInspireApp(),
      ),
    );
    expect(find.text('File'), findsOneWidget);
    expect(find.text('Edit'), findsOneWidget);
    expect(find.text('Guitar'), findsOneWidget);
    expect(find.byType(TabView), findsOneWidget);
    expect(find.text('* Untitled'), findsOneWidget);

    // Ctrl+N replaces the document; the shell keeps working.
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyN);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.pump();
    expect(find.text('Guitar'), findsOneWidget);
  });
}
