import 'package:core_bridge/testing.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:playback_feature/playback_feature.dart';

void main() {
  late ProviderContainer container;

  setUp(() {
    container = ProviderContainer(
      overrides: [midiServiceProvider.overrideWithValue(FakeMidiService())],
    );
  });

  tearDown(() => container.dispose());

  test('import returns a document and a summary', () async {
    final actions = container.read(midiActionsProvider);
    final outcome = await actions.import(const [0x4d, 0x54, 0x68, 0x64]);
    expect(outcome.document.summary().tracks, isNotEmpty);
    expect(outcome.summary, contains('Imported'));
    outcome.document.close();
  });

  test('export returns bytes and a summary', () async {
    final actions = container.read(midiActionsProvider);
    final doc = FakeDocumentService(bars: 1);
    final outcome = await actions.export(doc);
    expect(outcome.bytes, isNotEmpty);
    expect(outcome.summary, contains('Exported'));
    doc.close();
  });
}
