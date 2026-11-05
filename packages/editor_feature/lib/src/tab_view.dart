import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:ui_kit/ui_kit.dart';

import 'document_controller.dart';
import 'editor_cursor.dart';
import 'system_painter.dart';
import 'tab_metrics.dart';

/// Hit-test helpers over the packed layout arrays.
extension SystemHits on SystemDto {
  int get hitCount => hitMeta.length ~/ 4;

  Rect hitRect(int i) => Rect.fromLTWH(
    hitRects[i * 4],
    hitRects[i * 4 + 1],
    hitRects[i * 4 + 2],
    hitRects[i * 4 + 3],
  );

  EditorCursor hitCursor(int i) {
    final slot = hitMeta[i * 4 + 3];
    return EditorCursor(
      bar: hitMeta[i * 4],
      string: hitMeta[i * 4 + 1],
      voice: hitMeta[i * 4 + 2],
      beat: slot < 0 ? null : beats[slot].index,
    );
  }

  /// Hit region at a point in system coordinates, preferring [voice].
  int? hitAt(Offset p, {int voice = 0}) {
    int? first;
    for (var i = 0; i < hitCount; i++) {
      if (!hitRect(i).contains(p)) continue;
      if (hitMeta[i * 4 + 2] == voice) return i;
      first ??= i;
    }
    return first;
  }

  /// Hit region for a cursor, if the cursor is in this system.
  int? hitOf(EditorCursor c) {
    for (var i = 0; i < hitCount; i++) {
      if (hitMeta[i * 4] != c.bar ||
          hitMeta[i * 4 + 1] != c.string ||
          hitMeta[i * 4 + 2] != c.voice) {
        continue;
      }
      final slot = hitMeta[i * 4 + 3];
      final beat = slot < 0 ? null : beats[slot].index;
      if (beat == c.beat) return i;
    }
    return null;
  }
}

/// TAB of the active track: one painter per system in a virtualized list, so only
/// visible systems are built and painted (cross-platform-ui section 5).
class TabView extends ConsumerStatefulWidget {
  const TabView({super.key, this.zoom = 1.0});

  final double zoom;

  @override
  ConsumerState<TabView> createState() => _TabViewState();
}

class _TabViewState extends ConsumerState<TabView> {
  TabLayoutDto? _layout;
  (int, int, double, double)? _key;

  /// Layout is recomputed only when revision, track, width or zoom change
  /// (projection cache, cross-platform-ui section 4).
  TabLayoutDto _layoutFor(EditorState s, double width) {
    final key = (s.summary.revision, s.activeTrack, width, widget.zoom);
    if (_key != key || _layout == null) {
      _layout = s.service.tabLayout(
        s.activeTrack,
        measureTabMetrics(width: width, zoom: widget.zoom),
      );
      _key = key;
    }
    return _layout!;
  }

  /// Maps a playback tick to an x inside `system` from the played-bar tick ranges and
  /// the system's bar x-positions. A played bar's master-bar index is matched to the
  /// bar layout; x is the bar's left plus the within-bar fraction times its width.
  /// Returns null when the tick is not in a bar this system draws.
  double? Function(int tick) _tickToX(SystemDto system, List<PlayedBarDto> played) {
    return (int tick) {
      for (final pb in played) {
        if (tick < pb.startTick || tick >= pb.startTick + pb.lengthTicks) {
          continue;
        }
        for (final bar in system.bars) {
          if (bar.bar != pb.masterBar) continue;
          final frac = pb.lengthTicks <= 0
              ? 0.0
              : (tick - pb.startTick) / pb.lengthTicks;
          return bar.x + frac * bar.width;
        }
      }
      return null;
    };
  }

  @override
  Widget build(BuildContext context) {
    final s = ref.watch(editorProvider);
    final colors = TabColors.of(Theme.of(context).colorScheme);
    final fontSize = 11.0 * widget.zoom;
    final cursorBuilder = ref.watch(cursorOverlayProvider);
    // Played-bar tick ranges for the cursor; empty when there is no cursor overlay.
    final played = cursorBuilder == null
        ? const <PlayedBarDto>[]
        : s.service.playedBars();
    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth.clamp(320.0, 4000.0);
        final layout = _layoutFor(s, width);
        final gap = 16.0 * widget.zoom;
        return ColoredBox(
          color: colors.paper,
          child: ListView.builder(
            key: const ValueKey('tab-systems'),
            itemCount: layout.systems.length,
            itemBuilder: (context, i) {
              final system = layout.systems[i];
              final cursorHit = system.hitOf(s.cursor);
              return Padding(
                padding: EdgeInsets.only(bottom: gap),
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onTapDown: (d) {
                    final hit = system.hitAt(
                      d.localPosition,
                      voice: s.cursor.voice,
                    );
                    if (hit != null) {
                      ref
                          .read(editorProvider.notifier)
                          .moveCursor(system.hitCursor(hit));
                    }
                  },
                  child: SizedBox(
                    width: width,
                    height: system.height,
                    child: Stack(
                      children: [
                        Positioned.fill(
                          child: RepaintBoundary(
                            child: CustomPaint(
                              key: ValueKey('system-${system.index}'),
                              painter: SystemPainter(
                                system: system,
                                colors: colors,
                                fontSize: fontSize,
                              ),
                            ),
                          ),
                        ),
                        Positioned.fill(
                          child: CustomPaint(
                            painter: CursorPainter(
                              rect: cursorHit == null
                                  ? null
                                  : system.hitRect(cursorHit),
                              color: colors.cursor,
                            ),
                          ),
                        ),
                        if (cursorBuilder != null)
                          Positioned.fill(
                            child: cursorBuilder(
                              _tickToX(system, played),
                              system.height,
                            ),
                          ),
                      ],
                    ),
                  ),
                ),
              );
            },
          ),
        );
      },
    );
  }
}
