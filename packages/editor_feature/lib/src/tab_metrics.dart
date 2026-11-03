import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/painting.dart';
import 'package:ui_kit/ui_kit.dart';

/// Layout metrics measured from the bundled TAB font at a zoom level (ADR-018,
/// ADR-021). Measuring with the same [tabTextStyle] used for painting keeps Rust
/// spacing and Dart text in agreement (R-05).
MetricsDto measureTabMetrics({required double width, double zoom = 1.0}) {
  final fontSize = 11.0 * zoom;
  final painter = TextPainter(
    text: TextSpan(
      text: '0',
      style: tabTextStyle(size: fontSize),
    ),
    textDirection: TextDirection.ltr,
  )..layout();
  final digit = painter.width;
  painter.dispose();
  final spacing = 12.0 * zoom;
  return MetricsDto(
    width: width,
    marginLeft: 24 * zoom,
    marginRight: 24 * zoom,
    stringSpacing: spacing,
    digitWidth: digit,
    glyphPadding: 2 * zoom,
    minColumnWidth: 18 * zoom,
    durationSpacing: 0.4,
    barPadding: 8 * zoom,
    emptyBarWidth: 80 * zoom,
    headerWidth: 28 * zoom,
    timeSignatureWidth: 16 * zoom,
    repeatWidth: 10 * zoom,
    aboveStaff: 28 * zoom,
    belowStaff: 36 * zoom,
    systemGap: 16 * zoom,
    stemLength: 20 * zoom,
  );
}
