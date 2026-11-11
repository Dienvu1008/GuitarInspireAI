import 'package:core_bridge/testing.dart';

import 'contract/guitar_pro_service_contract.dart';

void main() {
  guitarProServiceContract(
    'fake',
    FakeGuitarProService.new,
    FakeDocumentFactory.new,
  );
}
