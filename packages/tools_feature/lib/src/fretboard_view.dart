import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';

/// Paints a single voicing on a small fretboard diagram (chord-chart style): strings as
/// vertical lines, frets as horizontal lines, a dot per fretted note, "o"/"x" above open
/// and muted strings. Pure presentation; the voicing comes from the core (GR-05).
///
/// `frets` is per string, string 1 (highest) first, as in [VoicingDto]: -1 muted, 0 open,
/// >0 fretted. The diagram shows string 1 on the right (chord-chart convention).
class FretboardView extends StatelessWidget {
  const FretboardView({super.key, required this.voicing, this.size = const Size(140, 160)});

  final VoicingDto voicing;
  final Size size;

  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      size: size,
      painter: _FretboardPainter(
        frets: voicing.frets,
        baseFret: voicing.lowestFret > 1 ? voicing.lowestFret : 1,
        color: Theme.of(context).colorScheme.onSurface,
        dot: Theme.of(context).colorScheme.primary,
      ),
    );
  }
}

class _FretboardPainter extends CustomPainter {
  _FretboardPainter({
    required this.frets,
    required this.baseFret,
    required this.color,
    required this.dot,
  });

  final List<int> frets; // string 1 first; -1 mute, 0 open, >0 fret
  final int baseFret;
  final Color color;
  final Color dot;

  static const _fretsShown = 4;

  @override
  void paint(Canvas canvas, Size size) {
    final strings = frets.length;
    if (strings == 0) return;
    const marginTop = 24.0;
    const marginBottom = 8.0;
    final left = 14.0;
    final right = size.width - 14.0;
    final top = marginTop;
    final bottom = size.height - marginBottom;
    final colW = (right - left) / (strings - 1);
    final rowH = (bottom - top) / _fretsShown;

    final line = Paint()
      ..color = color.withValues(alpha: 0.7)
      ..strokeWidth = 1;

    // String 1 (highest) on the right: draw from low string on the left.
    for (var s = 0; s < strings; s++) {
      final x = left + s * colW;
      canvas.drawLine(Offset(x, top), Offset(x, bottom), line);
    }
    for (var fr = 0; fr <= _fretsShown; fr++) {
      final y = top + fr * rowH;
      final p = Paint()
        ..color = color.withValues(alpha: fr == 0 && baseFret == 1 ? 1.0 : 0.7)
        ..strokeWidth = fr == 0 && baseFret == 1 ? 3 : 1; // nut when at the top
      canvas.drawLine(Offset(left, y), Offset(right, y), p);
    }
    // Base-fret label when not starting at the nut.
    if (baseFret > 1) {
      final tp = TextPainter(
        text: TextSpan(
          text: '${baseFret}fr',
          style: TextStyle(color: color, fontSize: 10),
        ),
        textDirection: TextDirection.ltr,
      )..layout();
      tp.paint(canvas, Offset(right + 2, top - 2));
    }

    final textStyle = TextStyle(color: color, fontSize: 12);
    for (var s = 0; s < strings; s++) {
      // Chart order: string 1 on the right -> column index (strings-1-s).
      final x = left + (strings - 1 - s) * colW;
      final f = frets[s];
      if (f < 0) {
        _glyph(canvas, 'x', x, top - 16, textStyle);
      } else if (f == 0) {
        _glyph(canvas, 'o', x, top - 16, textStyle);
      } else {
        final rel = f - baseFret; // 0-based row within the shown window
        final y = top + (rel + 0.5) * rowH;
        canvas.drawCircle(Offset(x, y), 6, Paint()..color = dot);
      }
    }
  }

  void _glyph(Canvas canvas, String g, double x, double y, TextStyle style) {
    final tp = TextPainter(
      text: TextSpan(text: g, style: style),
      textDirection: TextDirection.ltr,
    )..layout();
    tp.paint(canvas, Offset(x - tp.width / 2, y));
  }

  @override
  bool shouldRepaint(_FretboardPainter old) =>
      old.frets != frets || old.baseFret != baseFret;
}
