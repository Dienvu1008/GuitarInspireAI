import 'package:core_bridge/testing.dart';

import 'contract/musicxml_service_contract.dart';

void main() {
  musicXmlServiceContract(
    'fake',
    FakeMusicXmlService.new,
    FakeDocumentFactory.new,
  );
}
