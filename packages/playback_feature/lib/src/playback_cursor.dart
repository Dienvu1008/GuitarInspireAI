import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'transport_controller.dart';

/// Maps a playback tick to an x offset in the TAB view's coordinate space. The editor
/// supplies it from its layout (bar start ticks and x positions); playback_feature does
/// not own layout. Returns `null` when the tick is not on screen.
typedef TickToX = double? Function(int tick);

/// A thin vertical line at the current playback position, drawn on its own layer above
/// the TAB so it never forces the score to repaint (ADR-028: cursor on a separate
/// layer). While playing it extrapolates between the engine's ~60 Hz position updates
/// using the measured tick velocity and the real frame time, so motion is smooth even
/// when a position update arrives late (ADR-005 amendment 5). The extrapolation is in
/// [TransportController.displayTick]; this widget only repaints each vsync frame.
class PlaybackCursor extends ConsumerStatefulWidget {
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
  ConsumerState<PlaybackCursor> createState() => _PlaybackCursorState();
}

class _PlaybackCursorState extends ConsumerState<PlaybackCursor>
    with SingleTickerProviderStateMixin {
  late final Ticker _ticker;
  int _tick = 0;

  @override
  void initState() {
    super.initState();
    // Repaint each vsync frame while mounted; the controller computes the extrapolated
    // tick from real time, so a dropped or late frame does not make the cursor jump.
    _ticker = createTicker((_) {
      final next = ref.read(transportProvider.notifier).displayTick();
      if (next != _tick) setState(() => _tick = next);
    })..start();
  }

  @override
  void dispose() {
    _ticker.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final playing = ref.watch(transportProvider.select((s) => s.playing));
    // Hide the cursor when stopped at the very start, so a fresh document is clean.
    if (!playing && _tick == 0) return const SizedBox.shrink();
    final x = widget.tickToX(_tick);
    if (x == null) return const SizedBox.shrink();
    return IgnorePointer(
      child: CustomPaint(
        painter: _CursorPainter(
          x: x,
          height: widget.height,
          color: widget.color ?? Theme.of(context).colorScheme.primary,
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
