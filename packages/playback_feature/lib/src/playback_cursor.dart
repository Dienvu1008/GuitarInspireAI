import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'transport_controller.dart';

/// Maps a playback tick to an x offset in the TAB view's coordinate space. The editor
/// supplies it from its layout (bar start ticks and x positions); playback_feature does
/// not own layout. Returns `null` when the tick is not on screen.
typedef TickToX = double? Function(int tick);

/// A thin vertical line at the current playback position, drawn on its own layer above
/// the TAB so it never forces the score to repaint (ADR-028: cursor on a separate
/// layer). The line follows the engine position stream; between updates it stays put
/// (extrapolation is a later refinement, ADR-005 amendment 5).
class PlaybackCursor extends ConsumerWidget {
  const PlaybackCursor({
    super.key,
    required this.tickToX,
    required this.height,
    this.color,
  });

  final TickToX tickToX;
  final double height;
  final Color? color;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final playing = ref.watch(transportProvider.select((s) => s.playing));
    final tick = ref.watch(transportProvider.select((s) => s.tick));
    // Hide the cursor when stopped at the very start, so a fresh document is clean.
    if (!playing && tick == 0) return const SizedBox.shrink();
    final x = tickToX(tick);
    if (x == null) return const SizedBox.shrink();
    return IgnorePointer(
      child: CustomPaint(
        painter: _CursorPainter(
          x: x,
          height: height,
          color: color ?? Theme.of(context).colorScheme.primary,
        ),
      ),
    );
  }
}

class _CursorPainter extends CustomPainter {
  _CursorPainter({required this.x, required this.height, required this.color});

  final double x;
  final double height;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = 1.5;
    canvas.drawLine(Offset(x, 0), Offset(x, height), paint);
  }

  @override
  bool shouldRepaint(_CursorPainter old) =>
      old.x != x || old.height != height || old.color != color;
}
