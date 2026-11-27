import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:ui_kit/ui_kit.dart';

/// Paints one notation stave from the core's display list (ADR-018, ADR-041 section
/// 9). Painting only: every glyph, position, line and thickness comes from the DTO;
/// this class decides colours and nothing musical.
final class NotationPainter extends CustomPainter {
  NotationPainter({required this.staff, required this.colors});

  final NotationStaffDto staff;
  final TabColors colors;

  /// Codepoints in the order the last paint drew them (for tests, F4).
  @visibleForTesting
  final List<int> painted = [];

  // Laid-out glyphs shared by all painters, as for TAB labels: the set of distinct
  // glyphs per size is small. The cap only guards against unbounded growth.
  static final Map<String, (TextPainter, double)> _glyphs = {};
  static const _maxCached = 2048;

  /// The glyph and its baseline offset (from the top of its line box).
  (TextPainter, double) _glyph(int codepoint, double size, Color color) {
    final key = '$codepoint|$size|${color.toARGB32()}';
    final cached = _glyphs[key];
    if (cached != null) return cached;
    if (_glyphs.length >= _maxCached) _glyphs.clear();
    final p = TextPainter(
      text: TextSpan(
        text: String.fromCharCode(codepoint),
        style: musicTextStyle(size: size, color: color),
      ),
      textDirection: TextDirection.ltr,
    )..layout();
    final baseline = p.computeDistanceToActualBaseline(TextBaseline.alphabetic);
    return _glyphs[key] = (p, baseline);
  }

  @override
  void paint(Canvas canvas, Size size) {
    painted.clear();
    final s = staff;
    final pen = Paint()
      ..strokeCap = StrokeCap.butt
      ..style = PaintingStyle.stroke;
    for (var i = 0; i < s.lineKinds.length; i++) {
      final o = i * 5;
      pen
        ..color = s.lineKinds[i] == NotationLineKind.staff
            ? colors.staff
            : colors.ink
        ..strokeWidth = s.lines[o + 4];
      canvas.drawLine(
        Offset(s.lines[o], s.lines[o + 1]),
        Offset(s.lines[o + 2], s.lines[o + 3]),
        pen,
      );
    }
    final fill = Paint()..color = colors.ink;
    for (var i = 0; i * 5 < s.beams.length; i++) {
      final o = i * 5;
      final h = s.beams[o + 4] / 2;
      canvas.drawPath(
        Path()
          ..moveTo(s.beams[o], s.beams[o + 1] - h)
          ..lineTo(s.beams[o + 2], s.beams[o + 3] - h)
          ..lineTo(s.beams[o + 2], s.beams[o + 3] + h)
          ..lineTo(s.beams[o], s.beams[o + 1] + h)
          ..close(),
        fill,
      );
    }
    for (var i = 0; i < s.tieUp.length; i++) {
      final o = i * 6;
      final (x1, x2, y) = (s.ties[o], s.ties[o + 1], s.ties[o + 2]);
      final dir = s.tieUp[i] == 1 ? -1.0 : 1.0;
      final outer = s.ties[o + 3] * dir;
      final inner = (s.ties[o + 3] - s.ties[o + 5]) * dir;
      final mid = (x1 + x2) / 2;
      // A crescent: the outer arc peaks at the tie height, the inner one at the
      // height less the middle thickness (a quadratic peaks at half its control).
      canvas.drawPath(
        Path()
          ..moveTo(x1, y)
          ..quadraticBezierTo(mid, y + 2 * outer, x2, y)
          ..quadraticBezierTo(mid, y + 2 * inner, x1, y)
          ..close(),
        fill,
      );
    }
    // SMuFL: one em is four staff spaces.
    final em = 4 * s.staffSpace;
    for (var i = 0; i < s.glyphCodepoints.length; i++) {
      final cp = s.glyphCodepoints[i];
      final (p, baseline) = _glyph(cp, em, colors.ink);
      p.paint(
        canvas,
        Offset(s.glyphPositions[i * 2], s.glyphPositions[i * 2 + 1] - baseline),
      );
      painted.add(cp);
    }
  }

  @override
  bool shouldRepaint(NotationPainter old) =>
      old.colors != colors || !_sameStaff(old.staff, staff);
}

bool _sameStaff(NotationStaffDto a, NotationStaffDto b) =>
    identical(a, b) ||
    (a.staffSpace == b.staffSpace &&
        listEquals(a.lines, b.lines) &&
        listEquals(a.lineKinds, b.lineKinds) &&
        listEquals(a.glyphCodepoints, b.glyphCodepoints) &&
        listEquals(a.glyphPositions, b.glyphPositions) &&
        listEquals(a.beams, b.beams) &&
        listEquals(a.ties, b.ties) &&
        listEquals(a.tieUp, b.tieUp));
