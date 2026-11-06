import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'tuner_controller.dart';

/// The guitar tuner panel: a cents needle, the nearest string, the frequency, and the
/// microphone on/off. Shows a clear message when no microphone is available or access is
/// denied (T2), without blocking the rest of the app. No musical rule here (GR-05).
class TunerView extends ConsumerWidget {
  const TunerView({super.key});

  /// String names by 1-based index for standard 6-string display; falls back to
  /// "String N" for other tunings (the core decides the pitch, this only labels).
  static const _names = ['', '1 (e)', '2 (B)', '3 (G)', '4 (D)', '5 (A)', '6 (E)'];

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = ref.watch(tunerProvider);
    final tuner = ref.read(tunerProvider.notifier);
    final scheme = Theme.of(context).colorScheme;

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              const Text('Tuner', style: TextStyle(fontWeight: FontWeight.bold)),
              const Spacer(),
              FilledButton.tonalIcon(
                key: const ValueKey('tuner-toggle'),
                icon: Icon(s.listening ? Icons.mic : Icons.mic_off),
                label: Text(s.listening ? 'Stop' : 'Start'),
                onPressed: () =>
                    s.listening ? tuner.disable() : tuner.enable(),
              ),
            ],
          ),
          const SizedBox(height: 16),
          SizedBox(
            height: 120,
            child: CustomPaint(
              painter: _NeedlePainter(
                cents: s.hasPitch ? s.cents.clamp(-50.0, 50.0) : 0.0,
                active: s.hasPitch,
                inTune: s.inTune,
                scheme: scheme,
              ),
              child: const SizedBox.expand(),
            ),
          ),
          const SizedBox(height: 12),
          Center(
            child: Text(
              _bigLabel(s),
              key: const ValueKey('tuner-note'),
              style: TextStyle(
                fontSize: 28,
                fontWeight: FontWeight.w600,
                color: s.inTune ? Colors.green : scheme.onSurface,
              ),
            ),
          ),
          const SizedBox(height: 4),
          Center(
            child: Text(
              s.hasPitch
                  ? '${s.cents >= 0 ? '+' : ''}${s.cents.toStringAsFixed(0)} cents   ${s.hz.toStringAsFixed(1)} Hz'
                  : (s.listening ? 'Play a string' : 'Microphone off'),
              key: const ValueKey('tuner-detail'),
            ),
          ),
          if (s.message != null) ...[
            const SizedBox(height: 12),
            Text(
              s.message!,
              key: const ValueKey('tuner-message'),
              style: TextStyle(color: scheme.error),
            ),
          ],
        ],
      ),
    );
  }

  String _bigLabel(TunerState s) {
    if (!s.hasPitch) return '—';
    final i = s.string;
    return (i >= 1 && i < _names.length) ? _names[i] : 'String $i';
  }
}

class _NeedlePainter extends CustomPainter {
  _NeedlePainter({
    required this.cents,
    required this.active,
    required this.inTune,
    required this.scheme,
  });

  final double cents; // -50..50
  final bool active;
  final bool inTune;
  final ColorScheme scheme;

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height);
    final radius = size.height * 0.9;
    final track = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 3
      ..color = scheme.outlineVariant;
    // Arc from -60 to +60 degrees (map -50..50 cents to that).
    const sweep = math.pi * 2 / 3; // 120 degrees
    const start = -math.pi / 2 - sweep / 2;
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: radius),
      start,
      sweep,
      false,
      track,
    );
    // In-tune zone (center tick).
    final zone = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 3
      ..color = Colors.green.withValues(alpha: 0.6);
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: radius),
      -math.pi / 2 - sweep * (5.0 / 100.0),
      sweep * (10.0 / 100.0),
      false,
      zone,
    );
    // Needle.
    final t = (cents + 50.0) / 100.0; // 0..1
    final angle = start + sweep * t;
    final needle = Paint()
      ..strokeWidth = 3
      ..color = !active
          ? scheme.outline
          : (inTune ? Colors.green : scheme.primary);
    final tip = center + Offset(radius * math.cos(angle), radius * math.sin(angle));
    canvas.drawLine(center, tip, needle);
    canvas.drawCircle(center, 5, Paint()..color = needle.color);
  }

  @override
  bool shouldRepaint(_NeedlePainter old) =>
      old.cents != cents || old.active != active || old.inTune != inTune;
}
