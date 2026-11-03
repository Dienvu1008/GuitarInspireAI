import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:ui_kit/ui_kit.dart';

void main() {
  test('tabTextStyle uses the bundled font family', () {
    final style = tabTextStyle();
    // Flutter prefixes the package name into the family for cross-package fonts.
    expect(style.fontFamily, contains('NotoSansMono'));
  });

  test('AppTheme produces valid light and dark themes', () {
    final light = AppTheme.light();
    final dark = AppTheme.dark();
    expect(light.useMaterial3, isTrue);
    expect(dark.useMaterial3, isTrue);
    expect(light.colorScheme.brightness, Brightness.light);
    expect(dark.colorScheme.brightness, Brightness.dark);
  });

  test('TabColors.of produces correct colours for a scheme', () {
    final scheme = ColorScheme.fromSeed(seedColor: Colors.teal);
    final colors = TabColors.of(scheme);
    expect(colors.ink, scheme.onSurface);
    expect(colors.cursor, scheme.primary);
  });
}
