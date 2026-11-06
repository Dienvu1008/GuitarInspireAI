import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart' show HardwareKeyboard;
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

  /// One rectangle per selected beat in this system (the union of its string hit
  /// regions), for the selection painter.
  List<Rect> selectionRects((BeatSlot, BeatSlot)? selection) {
    if (selection == null) return const [];
    final (from, to) = selection;
    final bySlot = <int, Rect>{};
    for (var i = 0; i < hitCount; i++) {
      final slot = hitMeta[i * 4 + 3];
      if (slot < 0 || hitMeta[i * 4 + 2] != from.voice) continue;
      final bar = hitMeta[i * 4];
      final index = beats[slot].index;
      final afterStart =
          bar > from.bar || (bar == from.bar && index >= from.index);
      final beforeEnd = bar < to.bar || (bar == to.bar && index <= to.index);
      if (!afterStart || !beforeEnd) continue;
      final r = hitRect(i);
      bySlot[slot] = bySlot[slot]?.expandToInclude(r) ?? r;
    }
    return bySlot.values.toList();
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

/// TAB of the shown tracks (ADR-031): one system per list item, each with a stave
/// per shown track whose bar lines line up. The outline sizes the virtualized list;
/// system content is laid out by the core only when an item is built, so a large
/// score with many tracks stays fast (cross-platform-ui section 5).
class TabView extends ConsumerStatefulWidget {
  const TabView({super.key, this.zoom = 1.0});

  final double zoom;

  @override
  ConsumerState<TabView> createState() => _TabViewState();
}

class _TabViewState extends ConsumerState<TabView> {
  /// Systems laid out per core call when an item needs one that is not cached.
  static const _batch = 4;

  String? _key;
  late MetricsDto _metrics;
  late List<int> _tracks;
  ScoreOutlineDto? _outline;
  final Map<int, ScoreSystemDto> _systems = {};

  /// The outline is recomputed and the system cache cleared only when revision,
  /// shown tracks, width or zoom change (projection cache, cross-platform-ui
  /// section 4).
  ScoreOutlineDto _outlineFor(EditorState s, double width) {
    final tracks = s.shownTracks;
    final key =
        '${s.summary.revision}|${tracks.join(',')}|$width|${widget.zoom}';
    if (_key != key || _outline == null) {
      _metrics = measureTabMetrics(width: width, zoom: widget.zoom);
      _tracks = tracks;
      _outline = s.service.scoreOutline(tracks, _metrics);
      _systems.clear();
      _key = key;
    }
    return _outline!;
  }

  ScoreSystemDto _system(EditorState s, int index) {
    final cached = _systems[index];
    if (cached != null) return cached;
    for (final sys in s.service.scoreSystems(
      _tracks,
      _metrics,
      index,
      _batch,
    )) {
      _systems[sys.index] = sys;
    }
    return _systems[index]!;
  }

  /// Maps a playback tick to an x inside a system from the played-bar tick ranges
  /// and the system's bar x-positions (equal in every stave). Returns null when the
  /// tick is not in a bar this system draws.
  double? Function(int tick) _tickToX(
    SystemDto system,
    List<PlayedBarDto> played,
  ) {
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
    final names = {for (final t in s.summary.tracks) t.id: t.name};
    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth.clamp(320.0, 4000.0);
        final outline = _outlineFor(s, width);
        final gap = 16.0 * widget.zoom;
        final labelled = _tracks.length > 1;
        return ColoredBox(
          color: colors.paper,
          child: ListView.builder(
            key: const ValueKey('tab-systems'),
            itemCount: outline.systems.length,
            itemBuilder: (context, i) {
              final sys = _system(s, i);
              return Padding(
                padding: EdgeInsets.only(bottom: gap),
                child: SizedBox(
                  key: ValueKey('system-${sys.index}'),
                  width: width,
                  height: sys.height,
                  child: Stack(
                    children: [
                      for (final stave in sys.staves)
                        Positioned(
                          left: 0,
                          right: 0,
                          top: stave.top,
                          height: stave.system.height,
                          child: _StaveView(
                            index: sys.index,
                            stave: stave,
                            state: s,
                            colors: colors,
                            fontSize: fontSize,
                            label: labelled ? names[stave.track] : null,
                          ),
                        ),
                      // One playback cursor across every stave of the system.
                      if (cursorBuilder != null && sys.staves.isNotEmpty)
                        Positioned.fill(
                          child: cursorBuilder(
                            _tickToX(sys.staves.first.system, played),
                            sys.height,
                          ),
                        ),
                    ],
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

/// One track's stave: TAB painting, and on the active track the selection and the
/// cursor. A click on another track's stave makes it active and places the cursor.
class _StaveView extends ConsumerWidget {
  const _StaveView({
    required this.index,
    required this.stave,
    required this.state,
    required this.colors,
    required this.fontSize,
    required this.label,
  });

  final int index;
  final StaveDto stave;
  final EditorState state;
  final TabColors colors;
  final double fontSize;

  /// Track name shown at the stave's top left when several tracks are shown.
  final String? label;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final system = stave.system;
    final active = stave.track == state.activeTrack;
    final cursorHit = active ? system.hitOf(state.cursor) : null;
    return GestureDetector(
      key: ValueKey('stave-$index-${stave.track}'),
      behavior: HitTestBehavior.opaque,
      onTapDown: (d) {
        final editor = ref.read(editorProvider.notifier);
        final hit = system.hitAt(
          d.localPosition,
          voice: active ? state.cursor.voice : 0,
        );
        if (!active) editor.selectTrack(stave.track);
        if (hit == null) return;
        final target = system.hitCursor(hit);
        // Shift+click extends the selection to a beat (ADR-030), on the same track.
        final extend =
            active &&
            HardwareKeyboard.instance.isShiftPressed &&
            target.beat != null;
        editor.moveCursor(target, extend: extend);
      },
      child: Stack(
        children: [
          Positioned.fill(
            child: RepaintBoundary(
              child: CustomPaint(
                painter: SystemPainter(
                  system: system,
                  colors: colors,
                  fontSize: fontSize,
                ),
              ),
            ),
          ),
          if (active) ...[
            Positioned.fill(
              child: CustomPaint(
                key: ValueKey('selection-$index'),
                painter: SelectionPainter(
                  rects: system.selectionRects(state.selection),
                  color: colors.cursor,
                ),
              ),
            ),
            Positioned.fill(
              child: CustomPaint(
                painter: CursorPainter(
                  rect: cursorHit == null ? null : system.hitRect(cursorHit),
                  color: colors.cursor,
                ),
              ),
            ),
          ],
          if (label != null)
            Positioned(
              left: 4,
              top: 0,
              child: Text(
                label!,
                style: TextStyle(
                  fontSize: fontSize * 0.9,
                  color: active ? colors.cursor : colors.staff,
                  fontWeight: active ? FontWeight.w600 : FontWeight.normal,
                ),
              ),
            ),
        ],
      ),
    );
  }
}
