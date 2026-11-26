import 'package:core_bridge/testing.dart';

import 'contract/pdf_service_contract.dart';

void main() {
  pdfServiceContract('fake', FakePdfService.new, FakeDocumentFactory.new);
}
