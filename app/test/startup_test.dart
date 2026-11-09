import 'package:app_platform/app_platform.dart';
import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:guitar_inspire/src/shell.dart';
import 'package:guitar_inspire/src/startup.dart';

const _dir = 'recovery-dir';

final class _Paths implements AppPaths {
  const _Paths();
  @override
  Future<String> recoveryDirectory() async => _dir;
}

/// Autosaves of an earlier session: one with a saved source, one untitled. The session
/// marker is present, as after a crash.
FakeDocumentFactory _crashedFactory() => FakeDocumentFactory()
  ..sessionMarkers.add(_dir)
  ..recovery[_dir] = [
    const RecoveryEntryDto(
      projectId: 'p-1',
      path: '$_dir/p-1/autosave-1.recovery',
      source: r'C:\songs\riff.giproj',
      timestamp: 1,
    ),
    const RecoveryEntryDto(
      projectId: 'p-2',
      path: '$_dir/p-2/autosave-2.recovery',
      timestamp: 2,
    ),
  ];

Future<void> _pumpGate(WidgetTester tester, FakeDocumentFactory f) async {
  tester.view.physicalSize = const Size(1400, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        documentFactoryProvider.overrideWithValue(f),
        appPathsProvider.overrideWithValue(const _Paths()),
      ],
      child: const StartupGate(),
    ),
  );
  // Not pumpAndSettle: the loading spinner behind the dialog never settles.
  for (var i = 0; i < 5; i++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
}

void main() {
  testWidgets('without autosaves the shell opens directly', (tester) async {
    await _pumpGate(tester, FakeDocumentFactory());
    expect(find.text('Recover documents'), findsNothing);
    expect(find.byType(Shell), findsOneWidget);
  });

  testWidgets('recovering keeps the autosave files until the next save', (
    tester,
  ) async {
    final f = _crashedFactory();
    await _pumpGate(tester, f);
    expect(find.text('Recover documents'), findsOneWidget);
    expect(find.text('riff.giproj'), findsOneWidget);
    expect(find.text('Untitled'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('recover-p-1')));
    await tester.pumpAndSettle();
    expect(find.byType(Shell), findsOneWidget);
    expect(f.recoveryEntries(_dir), hasLength(2));
  });

  testWidgets('Discard all removes every entry', (tester) async {
    final f = _crashedFactory();
    await _pumpGate(tester, f);
    await tester.tap(find.text('Discard all'));
    await tester.pumpAndSettle();
    expect(find.byType(Shell), findsOneWidget);
    expect(f.recoveryEntries(_dir), isEmpty);
  });

  testWidgets('C7: after a clean session autosaves are not offered', (
    tester,
  ) async {
    final f = _crashedFactory()..sessionMarkers.clear();
    await _pumpGate(tester, f);
    expect(find.text('Recover documents'), findsNothing);
    expect(find.byType(Shell), findsOneWidget);
    // Not offered is not deleted: the files stay for a later decision.
    expect(f.recoveryEntries(_dir), hasLength(2));
    // Starting writes the marker, so a crash now is detected next time.
    expect(f.sessionMarkers, contains(_dir));
  });

  testWidgets('C8: an autosave older than its source is not offered', (
    tester,
  ) async {
    final f = _crashedFactory()..sourceModified[r'C:\songs\riff.giproj'] = 5;
    await _pumpGate(tester, f);
    expect(find.text('Recover documents'), findsOneWidget);
    expect(find.text('riff.giproj'), findsNothing);
    expect(find.text('Untitled'), findsOneWidget);
  });

  testWidgets('C8: Open original opens the saved file and keeps the autosave', (
    tester,
  ) async {
    final f = _crashedFactory();
    final saved = f.create();
    await saved.saveAs(r'C:\songs\riff.giproj');
    await _pumpGate(tester, f);

    await tester.tap(find.byKey(const ValueKey('original-p-1')));
    await tester.pumpAndSettle();
    expect(find.byType(Shell), findsOneWidget);
    expect(find.text('Could not open the original'), findsNothing);
    expect(f.recoveryEntries(_dir), hasLength(2));
  });

  testWidgets('Open original is disabled for an untitled document', (
    tester,
  ) async {
    await _pumpGate(tester, _crashedFactory());
    final button = tester.widget<OutlinedButton>(
      find.byKey(const ValueKey('original-p-2')),
    );
    expect(button.onPressed, isNull);
  });

  testWidgets('Discard removes one entry and shows the rest', (tester) async {
    final f = _crashedFactory();
    await _pumpGate(tester, f);
    await tester.tap(find.byKey(const ValueKey('discard-p-1')));
    for (var i = 0; i < 5; i++) {
      await tester.pump(const Duration(milliseconds: 100));
    }
    expect(f.recoveryEntries(_dir).map((e) => e.projectId), ['p-2']);
    expect(find.text('Recover documents'), findsOneWidget);
    expect(find.text('riff.giproj'), findsNothing);
    expect(find.text('Untitled'), findsOneWidget);
  });
}
