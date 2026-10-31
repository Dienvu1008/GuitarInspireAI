import 'package:core_bridge/core_bridge.dart';
import 'package:core_bridge/testing.dart';

import 'contract/document_service_contract.dart';

MetricsDto testMetrics() => const MetricsDto(
  width: 1000,
  marginLeft: 24,
  marginRight: 24,
  stringSpacing: 12,
  digitWidth: 7,
  glyphPadding: 2,
  minColumnWidth: 18,
  durationSpacing: 0.4,
  barPadding: 8,
  emptyBarWidth: 80,
  headerWidth: 28,
  timeSignatureWidth: 16,
  repeatWidth: 10,
  aboveStaff: 28,
  belowStaff: 36,
  systemGap: 16,
  stemLength: 20,
);

void main() {
  documentServiceContract(
    'fake',
    () async => FakeDocumentFactory(),
    testMetrics,
  );
}
