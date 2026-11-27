import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart' show HardwareKeyboard;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:ui_kit/ui_kit.dart';

import 'document_controller.dart';
import 'editor_cursor.dart';
import 'notation_painter.dart';
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

  /// The cursor a hit region addresses. A notation beat region carries string 0
  /// ("keep the cursor's string", ADR-041 section 10): [keepString] is used then.
  EditorCursor hitCursor(int i, {int keepString = 1}) {
    final slot = hitMeta[i * 4 + 3];
    final string = hitMeta[i * 4 + 1];
    return EditorCursor(
      bar: hitMeta[i * 4],
      string: string == 0 ? keepString : string,
      voice: hitMeta[i * 4 + 2],
      beat: slot < 0 ? null : beats[slot].index,
    );
  }

  /// Hit region at a point in system coordinates, in the core's order (ADR-041
  /// section 10): a region with a string of [voice], one with a string of any voice,
  /// a region of [voice], any region. On TAB every region has a string, so this is
  /// "[voice] first, then any".
  int? hitAt(Offset p, {int voice = 0}) {
    int? pick({required bool head, required bool own}) {
      for (var i = 0; i < hitCount; i++) {
        if (!hitRect(i).contains(p)) continue;
        if (head && hitMeta[i * 4 + 1] == 0) continue;
        if (own && hitMeta[i * 4 + 2] != voice) continue;
        return i;
      }
      return null;
    }

    return pick(head: true, own: true) ??
        pick(head: true, own: false) ??
        pick(head: false, own: true) ??
        pick(head: false, own: false);
  }

  /// The beat region (string 0) of a cursor on a notation stave, if the cursor is
  /// on a beat in this system.
  int? beatHitOf(EditorCursor c) {
    if (c.beat == null) return null;
    for (var i = 0; i < hitCount; i++) {
      final slot = hitMeta[i * 4 + 3];
      if (hitMeta[i * 4 + 1] != 0 ||
          slot < 0 ||
          hitMeta[i * 4] != c.bar ||
          hitMeta[i * 4 + 2] != c.voice ||
          beats[slot].index != c.beat) {
        continue;
      }
      return i;
    }
    return null;
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
  late List<ShownTrackDto> _shown;
  ScoreOutlineDto? _outline;
  final Map<int, ScoreSystemDto> _systems = {};

  /// Staves per system: a notation stave (if on) and a TAB stave per shown track.
  int get _staveCount =>
      _shown.fold(0, (n, t) => n + (t.notation ? 1 : 0) + (t.tab ? 1 : 0));

  /// The outline is recomputed and the system cache cleared only when revision,
  /// shown staves, width or zoom change (projection cache, cross-platform-ui
  /// section 4).
  ScoreOutlineDto _outlineFor(EditorState s, double width) {
    final shown = s.shownStaves;
    final staves = shown.map((t) => '${t.track}${t.notation ? 'n' : ''}');
    // The service identity too: another document can have the same revision and
    // track ids (found by the 6c-3 screenshot tool; the key had no document).
    final key =
        '${identityHashCode(s.service)}|${s.summary.revision}|'
        '${staves.join(',')}|$width|${widget.zoom}';
    if (_key != key || _outline == null) {
      _metrics = measureTabMetrics(width: width, zoom: widget.zoom);
      _shown = shown;
      _outline = s.service.scoreOutlineWith(shown, _metrics);
      _systems.clear();
      _key = key;
    }
    return _outline!;
  }

  ScoreSystemDto _system(EditorState s, int index) {
    final cached = _systems[index];
    if (cached != null) return cached;
    for (final sys in s.service.scoreSystemsWith(
      _shown,
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
        final staves = _staveCount;
        final labelled = _shown.length > 1;
        return ColoredBox(
          color: colors.paper,
          // One list item per stave, not per system (ADR-031 6b-2d): a system of
          // many staves is taller than the screen, and only the staves in view are
          // built and painted.
          child: ListView.builder(
            key: const ValueKey('tab-systems'),
            itemCount: outline.systems.length * staves,
            itemBuilder: (context, i) {
              final sys = _system(s, i ~/ staves);
              final k = i % staves;
              if (k >= sys.staves.length) return const SizedBox.shrink();
              final stave = sys.staves[k];
              final height = stave.system.height;
              // The track name goes on the track's first stave only.
              final first = k == 0 || sys.staves[k - 1].track != stave.track;
              return Padding(
                // The gap between systems follows the last stave.
                padding: EdgeInsets.only(bottom: k == staves - 1 ? gap : 0),
                child: SizedBox(
                  // The top of the system is the top of its first stave.
                  key: k == 0 ? ValueKey('system-${sys.index}') : null,
                  width: width,
                  height: height,
                  child: Stack(
                    children: [
                      Positioned.fill(
                        child: _StaveView(
                          index: sys.index,
                          stave: stave,
                          state: s,
                          colors: colors,
                          fontSize: fontSize,
                          label: labelled && first ? names[stave.track] : null,
                        ),
                      ),
                      // The playback cursor at the same x on every stave of the
                      // system (bar x-positions are equal in every stave), so
                      // together they span the system.
                      if (cursorBuilder != null)
                        Positioned.fill(
                          child: cursorBuilder(
                            _tickToX(stave.system, played),
                            height,
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

  /// A notation stave. Read-only (ADR-038 decision 7, ADR-041 section 10): a tap
  /// selects a beat, and on a notehead its string, from the core's hit regions;
  /// editing stays on the TAB stave. The cursor and the selection are painted here
  /// too, over the beat regions.
  Widget _notation(WidgetRef ref, bool active) {
    final system = stave.system;
    final cursorHit = active ? system.beatHitOf(state.cursor) : null;
    return GestureDetector(
      key: ValueKey('notation-$index-${stave.track}'),
      behavior: HitTestBehavior.opaque,
      onTapDown: (d) {
        final editor = ref.read(editorProvider.notifier);
        final hit = system.hitAt(
          d.localPosition,
          voice: active ? state.cursor.voice : 0,
        );
        if (!active) editor.selectTrack(stave.track);
        if (hit == null) return;
        final current = ref.read(editorProvider).cursor;
        final target = system.hitCursor(hit, keepString: current.string);
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
                painter: NotationPainter(staff: stave.notation, colors: colors),
              ),
            ),
          ),
          if (active) ...[
            Positioned.fill(
              child: CustomPaint(
                key: ValueKey('notation-selection-$index'),
                painter: SelectionPainter(
                  rects: system.selectionRects(state.selection),
                  color: colors.cursor,
                ),
              ),
            ),
            Positioned.fill(
              child: CustomPaint(
                key: ValueKey('notation-cursor-$index'),
                painter: CursorPainter(
                  rect: cursorHit == null ? null : system.hitRect(cursorHit),
                  color: colors.cursor,
                ),
              ),
            ),
          ],
          if (label != null) _label(active),
        ],
      ),
    );
  }

  Widget _label(bool active) => Positioned(
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
  );

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final system = stave.system;
    final active = stave.track == state.activeTrack;
    if (stave.kind == StaveKind.notation) return _notation(ref, active);
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
          if (label != null) _label(active),
        ],
      ),
    );
  }
}
