import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/foundation.dart' show listEquals;
import 'package:flutter/rendering.dart';
import 'package:ui_kit/ui_kit.dart';

/// Paints one system of the Rust display list (ADR-018). Every position comes from
/// the layout; this painter only chooses symbols and colours.
final class SystemPainter extends CustomPainter {
  SystemPainter({
    required this.system,
    required this.colors,
    required this.fontSize,
  });

  final SystemDto system;
  final TabColors colors;
  final double fontSize;

  // Laid-out labels shared by all painters: every edit creates new painters, and
  // re-laying out each fret number on every frame dominated paint time. The set of
  // distinct labels is small (frets, bar numbers, a few symbols per size and colour);
  // the cap only guards against unbounded growth, for example many marker texts.
  static final Map<String, TextPainter> _text = {};
  static const _maxCachedLabels = 4096;

  TextPainter _label(String text, {double scale = 1.0, Color? color}) {
    final ink = color ?? colors.ink;
    final size = fontSize * scale;
    final key = '$text|$size|${ink.toARGB32()}';
    final cached = _text[key];
    if (cached != null) return cached;
    if (_text.length >= _maxCachedLabels) {
      // Not disposed: recorded pictures may still reference their paragraphs.
      _text.clear();
    }
    return _text[key] = TextPainter(
      text: TextSpan(
        text: text,
        style: tabTextStyle(size: size, color: ink),
      ),
      textDirection: TextDirection.ltr,
    )..layout();
  }

  void _centered(
    Canvas canvas,
    String text,
    double x,
    double y, {
    double scale = 1.0,
    Color? color,
  }) {
    final p = _label(text, scale: scale, color: color);
    p.paint(canvas, Offset(x - p.width / 2, y - p.height / 2));
  }

  @override
  void paint(Canvas canvas, Size size) {
    final staff = Paint()
      ..color = colors.staff
      ..strokeWidth = 1;
    final ink = Paint()
      ..color = colors.ink
      ..strokeWidth = 1;
    final thick = Paint()
      ..color = colors.ink
      ..strokeWidth = 3;
    final beam = Paint()
      ..color = colors.ink
      ..strokeWidth = 2.5;

    // Diagnostic bars (D-01) get a light warning tint, like red bars in Guitar Pro.
    final warn = Paint()..color = colors.warning.withValues(alpha: 0.08);
    for (final b in system.bars.where((b) => b.diagnostic)) {
      canvas.drawRect(Rect.fromLTWH(b.x, 0, b.width, system.height), warn);
    }

    final lines = system.lines;
    for (var i = 0; i < system.lineKinds.length; i++) {
      final from = Offset(lines[i * 4], lines[i * 4 + 1]);
      final to = Offset(lines[i * 4 + 2], lines[i * 4 + 3]);
      final paint = switch (system.lineKinds[i]) {
        LineKind.string => staff,
        LineKind.thickBar => thick,
        LineKind.beam => beam,
        _ => ink,
      };
      canvas.drawLine(from, to, paint);
    }

    final clear = Paint()..color = colors.paper;
    for (final g in system.glyphs) {
      switch (g.kind) {
        case GlyphKindDto_Fret(:final fret, :final tied, :final dead):
          final text = dead ? 'x' : (tied ? '($fret)' : '$fret');
          final p = _label(text);
          // Clear the string line behind the number.
          canvas.drawRect(
            Rect.fromCenter(
              center: Offset(g.x, g.y),
              width: p.width + 2,
              height: p.height * 0.8,
            ),
            clear,
          );
          p.paint(canvas, Offset(g.x - p.width / 2, g.y - p.height / 2));
        case GlyphKindDto_Rest(:final exponent):
          _paintRest(canvas, g.x, g.y, exponent, ink);
        case GlyphKindDto_Dot():
          canvas.drawCircle(
            Offset(g.x, g.y),
            1.6,
            ink..style = PaintingStyle.fill,
          );
        case GlyphKindDto_Tuplet(:final actual):
          _centered(canvas, '$actual', g.x, g.y, scale: 0.8);
        case GlyphKindDto_TimeSignature(:final numerator, :final denominator):
          _centered(
            canvas,
            '$numerator',
            g.x,
            g.y - fontSize * 0.75,
            scale: 1.4,
          );
          _centered(
            canvas,
            '$denominator',
            g.x,
            g.y + fontSize * 0.75,
            scale: 1.4,
          );
        case GlyphKindDto_RepeatDots():
          final fill = Paint()..color = colors.ink;
          canvas.drawCircle(Offset(g.x, g.y - fontSize * 0.6), 1.8, fill);
          canvas.drawCircle(Offset(g.x, g.y + fontSize * 0.6), 1.8, fill);
        case GlyphKindDto_RepeatCount(:final count):
          _centered(canvas, 'x$count', g.x, g.y, scale: 0.85);
        case GlyphKindDto_Volta(:final endings):
          _centered(canvas, '${endings.join('., ')}.', g.x, g.y, scale: 0.85);
        case GlyphKindDto_Marker(:final text):
          _centered(canvas, text, g.x, g.y, scale: 0.9);
        case GlyphKindDto_BarNumber(:final number):
          _centered(
            canvas,
            '$number',
            g.x,
            g.y,
            scale: 0.75,
            color: colors.staff,
          );
        case GlyphKindDto_TabClef():
          final step = fontSize * 1.05;
          for (final (i, c) in ['T', 'A', 'B'].indexed) {
            _centered(canvas, c, g.x, g.y + (i - 1) * step, scale: 1.1);
          }
        case GlyphKindDto_StringName(:final name):
          _centered(canvas, name, g.x, g.y, scale: 0.75, color: colors.staff);
        // ADR-035 technique marks; the core decides where they go.
        case GlyphKindDto_Accent():
          _centered(canvas, '>', g.x, g.y, scale: 0.9);
        case GlyphKindDto_Stroke(:final up):
          _paintArrow(canvas, g.x, g.y, up: up);
        case GlyphKindDto_StringPercussive():
          _centered(canvas, 'S', g.x, g.y, scale: 0.75);
        case GlyphKindDto_PalmMute(:final run):
          _paintPalmMute(canvas, g.x, g.y, g.width, run: run, ink: ink);
      }
    }
  }

  /// A stroke arrow drawn with lines (no font glyph needed): ↓ for down, ↑ for up.
  void _paintArrow(Canvas canvas, double x, double y, {required bool up}) {
    final pen = Paint()
      ..color = colors.ink
      ..strokeWidth = 1.2;
    final h = fontSize * 0.45;
    final head = fontSize * 0.22;
    final tip = Offset(x, up ? y - h : y + h);
    canvas.drawLine(Offset(x, up ? y + h : y - h), tip, pen);
    final back = up ? head : -head;
    canvas.drawLine(tip, Offset(x - head, tip.dy + back), pen);
    canvas.drawLine(tip, Offset(x + head, tip.dy + back), pen);
  }

  /// "P.M." at the start of the glyph box. A run continues as a dashed line to the
  /// end of the box, with a short end tick; a lone label has no line.
  void _paintPalmMute(
    Canvas canvas,
    double x,
    double y,
    double width, {
    required bool run,
    required Paint ink,
  }) {
    final label = _label('P.M.', scale: 0.7);
    final start = x - width / 2;
    final end = x + width / 2;
    if (!run) {
      label.paint(canvas, Offset(x - label.width / 2, y - label.height / 2));
      return;
    }
    label.paint(canvas, Offset(start, y - label.height / 2));
    final from = start + label.width + 2;
    if (from >= end) return;
    const dash = 4.0;
    const gap = 3.0;
    for (var dx = from; dx < end; dx += dash + gap) {
      final to = dx + dash < end ? dx + dash : end;
      canvas.drawLine(Offset(dx, y), Offset(to, y), ink);
    }
    canvas.drawLine(Offset(end, y - 3), Offset(end, y + 3), ink);
  }

  /// Simple rest symbols: whole and half as blocks, shorter values as a slash with
  /// flags (one per level below a quarter).
  void _paintRest(Canvas canvas, double x, double y, int exponent, Paint ink) {
    final fill = Paint()..color = colors.ink;
    if (exponent <= 1) {
      final top = exponent == 0 ? y - 3 : y;
      canvas.drawRect(Rect.fromLTWH(x - 4, top, 8, 3), fill);
      return;
    }
    canvas.drawLine(Offset(x + 2, y - 5), Offset(x - 2, y + 5), ink);
    for (var level = 0; level < exponent - 2; level++) {
      canvas.drawCircle(
        Offset(x + 1 - level * 1.5, y - 4 + level * 3),
        1.4,
        fill,
      );
    }
  }

  /// Every relayout returns new DTOs, and their == compares lists by identity, so the
  /// painted content is compared instead: an edit then repaints only the system it
  /// changed, not every visible one.
  @override
  bool shouldRepaint(SystemPainter old) =>
      old.colors != colors ||
      old.fontSize != fontSize ||
      !_samePaint(old.system, system);
}

/// True when [a] and [b] paint the same thing (hit regions are not painted).
bool _samePaint(SystemDto a, SystemDto b) =>
    identical(a, b) ||
    (a.height == b.height &&
        listEquals(a.lineKinds, b.lineKinds) &&
        listEquals(a.lines, b.lines) &&
        listEquals(a.bars, b.bars) &&
        listEquals(a.glyphs, b.glyphs));

/// The beat selection (ADR-030): one tinted column per selected beat, on its own
/// layer like the cursor.
final class SelectionPainter extends CustomPainter {
  SelectionPainter({required this.rects, required this.color});

  final List<Rect> rects;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final fill = Paint()..color = color.withValues(alpha: 0.18);
    for (final r in rects) {
      canvas.drawRect(r, fill);
    }
  }

  @override
  bool shouldRepaint(SelectionPainter old) =>
      old.color != color || !listEquals(old.rects, rects);
}

/// The cursor, painted on its own layer so moving it does not repaint the TAB
/// (cross-platform-ui section 8).
final class CursorPainter extends CustomPainter {
  CursorPainter({required this.rect, required this.color});

  final Rect? rect;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final r = rect;
    if (r == null) return;
    canvas.drawRect(
      r.deflate(0.5),
      Paint()
        ..color = color
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.5,
    );
    canvas.drawRect(r, Paint()..color = color.withValues(alpha: 0.12));
  }

  @override
  bool shouldRepaint(CursorPainter old) =>
      old.rect != rect || old.color != color;
}
