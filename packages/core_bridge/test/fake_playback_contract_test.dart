import 'package:core_bridge/testing.dart';

import 'contract/playback_service_contract.dart';

void main() {
  playbackServiceContract(
    'fake',
    () async => FakePlaybackFactory(),
    FakeDocumentFactory.new,
  );
}
