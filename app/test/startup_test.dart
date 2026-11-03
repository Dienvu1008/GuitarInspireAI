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

FakeDocumentFactory _crashedFactory() =>
    FakeDocumentFactory()
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
}
