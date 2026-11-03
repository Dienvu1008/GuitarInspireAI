import 'package:flutter/material.dart';

/// Font family of the bundled TAB font (ADR-021). Use [tabTextStyle] rather than the
/// name directly: the family lives in this package, so it needs the `package` field.
const String tabFontFamily = 'NotoSansMono';
const String tabFontPackage = 'ui_kit';

/// Text style for fret numbers and other TAB text. Layout metrics are measured with
/// exactly this style, so painting and layout agree (R-05).
TextStyle tabTextStyle({double size = 11, Color color = Colors.black}) =>
    TextStyle(
      fontFamily: tabFontFamily,
      package: tabFontPackage,
      fontSize: size,
      height: 1.0,
      color: color,
    );

/// Application theme (Material 3, light and dark).
abstract final class AppTheme {
  static ThemeData light() => _theme(Brightness.light);

  static ThemeData dark() => _theme(Brightness.dark);

  static ThemeData _theme(Brightness brightness) {
    final scheme = ColorScheme.fromSeed(
      seedColor: const Color(0xFF2E6B5E),
      brightness: brightness,
    );
    return ThemeData(
      colorScheme: scheme,
      useMaterial3: true,
      visualDensity: VisualDensity.compact,
    );
  }
}

/// Colours used to paint TAB, derived from the theme.
@immutable
final class TabColors {
  const TabColors({
    required this.ink,
    required this.staff,
    required this.paper,
    required this.cursor,
    required this.warning,
  });

  factory TabColors.of(ColorScheme scheme) => TabColors(
    ink: scheme.onSurface,
    staff: scheme.onSurface.withValues(alpha: 0.45),
    paper: scheme.surface,
    cursor: scheme.primary,
    warning: scheme.error,
  );

  final Color ink;
  final Color staff;
  final Color paper;
  final Color cursor;
  final Color warning;
}
