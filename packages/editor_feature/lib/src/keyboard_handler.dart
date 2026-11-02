import 'dart:async';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'document_controller.dart';
import 'editor_cursor.dart';
import 'key_bindings.dart';

/// Handles keyboard input for the TAB editor. No musical rules here — intents
/// become commands that the core validates (GR-05).
final class KeyboardHandler {
  KeyboardHandler(this._ref);

  final WidgetRef _ref;

  // Multi-digit fret entry state (Q-15).
  int? _pendingFret;
  int? _pendingBeatIndex;
  int? _pendingBar;
  int? _pendingVoice;
  Timer? _coalesceTimer;
  int? _coalesceKey;

  DocumentController get _editor => _ref.read(editorProvider.notifier);
  EditorState get _state => _ref.read(editorProvider);

  KeyEventResult handleKey(KeyEvent event) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }
    final pressed = HardwareKeyboard.instance.logicalKeysPressed;
    final intent = intentOf(event.logicalKey, pressed);
    if (intent == null) return KeyEventResult.ignored;
    _handle(intent);
    return KeyEventResult.handled;
  }

  void _handle(EditorIntent intent) {
    final s = _state;
    switch (intent) {
      case EditorIntent.undo:
        _cancelCoalesce();
        unawaited(_editor.undo());
        return;
      case EditorIntent.redo:
        _cancelCoalesce();
        unawaited(_editor.redo());
        return;
      case EditorIntent.newDocument:
        _cancelCoalesce();
        _editor.newDocument();
        return;
      case EditorIntent.open:
        _cancelCoalesce();
        unawaited(_editor.open());
        return;
      case EditorIntent.save:
        _cancelCoalesce();
        unawaited(_editor.save());
        return;
      case EditorIntent.saveAs:
        _cancelCoalesce();
        unawaited(_editor.saveAs());
        return;
      case EditorIntent.digit0:
      case EditorIntent.digit1:
      case EditorIntent.digit2:
      case EditorIntent.digit3:
      case EditorIntent.digit4:
      case EditorIntent.digit5:
      case EditorIntent.digit6:
      case EditorIntent.digit7:
      case EditorIntent.digit8:
      case EditorIntent.digit9:
        _enterDigit(intent, s);
        return;
      default:
        break;
    }
    // All navigation and duration intents cancel fret accumulation.
    _cancelCoalesce();
    switch (intent) {
      case EditorIntent.moveLeft:
        _moveCursor(_prevBeat(s));
      case EditorIntent.moveRight:
        _moveCursor(_nextBeat(s));
      case EditorIntent.moveUp:
        _moveCursor(
          EditorCursor(
            bar: s.cursor.bar,
            voice: s.cursor.voice,
            string: (s.cursor.string - 1).clamp(1, _strings(s)),
            beat: s.cursor.beat,
          ),
        );
      case EditorIntent.moveDown:
        _moveCursor(
          EditorCursor(
            bar: s.cursor.bar,
            voice: s.cursor.voice,
            string: (s.cursor.string + 1).clamp(1, _strings(s)),
            beat: s.cursor.beat,
          ),
        );
      case EditorIntent.deleteNote:
        _deleteNote(s);
      case EditorIntent.insertBeat:
        _insertBeat(s);
      case EditorIntent.durationWhole:
        _setBeatDuration(s, 0);
      case EditorIntent.durationHalf:
        _setBeatDuration(s, 1);
      case EditorIntent.durationQuarter:
        _setBeatDuration(s, 2);
      case EditorIntent.durationEighth:
        _setBeatDuration(s, 3);
      case EditorIntent.durationSixteenth:
        _setBeatDuration(s, 4);
      case EditorIntent.dot:
        _toggleDot(s);
      default:
        break;
    }
  }

  // ---------------------------------------------------------------- digit entry

  void _enterDigit(EditorIntent digit, EditorState s) {
    final d = digit.index - EditorIntent.digit0.index; // 0..9
    // Are we still accumulating digits for the same beat?
    final samebeat =
        _pendingBar == s.cursor.bar &&
        _pendingVoice == s.cursor.voice &&
        _pendingBeatIndex == s.cursor.beat;
    if (_pendingFret != null && samebeat) {
      final candidate = _pendingFret! * 10 + d;
      if (candidate <= 99) {
        _pendingFret = candidate;
        _coalesceTimer?.cancel();
        _coalesceTimer = Timer(
          const Duration(milliseconds: coalesceWindowMs),
          _flushDigit,
        );
        _commitFret(s);
        return;
      }
      // Overflow: start fresh with the new digit.
    }
    _cancelCoalesce();
    _pendingFret = d;
    _pendingBar = s.cursor.bar;
    _pendingVoice = s.cursor.voice;
    _pendingBeatIndex = s.cursor.beat;
    _coalesceKey = DateTime.now().millisecondsSinceEpoch;
    _coalesceTimer = Timer(
      const Duration(milliseconds: coalesceWindowMs),
      _flushDigit,
    );
    _commitFret(s);
  }

  void _commitFret(EditorState s) {
    final fret = _pendingFret!;
    final bar = s.cursor.bar;
    final voice = s.cursor.voice;
    // Ensure there is a beat to write into; insert one if the cursor is at the
    // append slot.
    if (s.cursor.beat == null) {
      // Insert a quarter beat, then set the note on it.
      final index = s.service.beats(s.activeTrack, bar, voice).length;
      unawaited(
        _editor.execute(
          CommandDto.insertBeat(
            track: s.activeTrack,
            bar: bar,
            voice: voice,
            index: index,
            duration: const DurationDto(base: 2, dots: 0),
          ),
          coalesceKey: _coalesceKey,
        ),
      );
      // The insert emits an event; the cursor moves to beat `index` after refresh.
      _pendingBeatIndex = index;
      _ref
          .read(editorProvider.notifier)
          .moveCursor(
            EditorCursor(
              bar: bar,
              voice: voice,
              string: s.cursor.string,
              beat: index,
            ),
          );
    }
    final beats = s.service.beats(s.activeTrack, bar, voice);
    final beatIndex = s.cursor.beat ?? 0;
    if (beatIndex >= beats.length) return;
    final beatId = beats[beatIndex].id;
    unawaited(
      _editor.execute(
        CommandDto.setNote(
          beat: BeatAddressDto(
            track: s.activeTrack,
            bar: bar,
            voice: voice,
            beat: beatId,
          ),
          string: s.cursor.string,
          fret: fret,
        ),
        coalesceKey: _coalesceKey,
      ),
    );
  }

  void _flushDigit() {
    _pendingFret = null;
    _pendingBar = null;
    _pendingVoice = null;
    _pendingBeatIndex = null;
    _coalesceKey = null;
    _coalesceTimer = null;
  }

  void _cancelCoalesce() {
    _coalesceTimer?.cancel();
    _flushDigit();
  }

  // ---------------------------------------------------------------- navigation

  EditorCursor _nextBeat(EditorState s) {
    final beats = s.service.beats(s.activeTrack, s.cursor.bar, s.cursor.voice);
    final cur = s.cursor.beat ?? beats.length;
    if (cur + 1 < beats.length) {
      return EditorCursor(
        bar: s.cursor.bar,
        voice: s.cursor.voice,
        string: s.cursor.string,
        beat: cur + 1,
      );
    }
    // Move to the next bar.
    final bars = s.summary.masterBars.length;
    if (s.cursor.bar + 1 < bars) {
      return EditorCursor(
        bar: s.cursor.bar + 1,
        voice: s.cursor.voice,
        string: s.cursor.string,
        beat: null,
      );
    }
    return s.cursor;
  }

  EditorCursor _prevBeat(EditorState s) {
    final cur = s.cursor.beat;
    if (cur != null && cur > 0) {
      return EditorCursor(
        bar: s.cursor.bar,
        voice: s.cursor.voice,
        string: s.cursor.string,
        beat: cur - 1,
      );
    }
    if (cur == 0) {
      return EditorCursor(
        bar: s.cursor.bar,
        voice: s.cursor.voice,
        string: s.cursor.string,
        beat: null,
      );
    }
    if (s.cursor.bar > 0) {
      final prevBeats = s.service.beats(
        s.activeTrack,
        s.cursor.bar - 1,
        s.cursor.voice,
      );
      return EditorCursor(
        bar: s.cursor.bar - 1,
        voice: s.cursor.voice,
        string: s.cursor.string,
        beat: prevBeats.isEmpty ? null : prevBeats.length - 1,
      );
    }
    return s.cursor;
  }

  void _moveCursor(EditorCursor c) => _editor.moveCursor(c);

  int _strings(EditorState s) => s.summary.tracks
      .firstWhere((t) => t.id == s.activeTrack)
      .instrument
      .tuning
      .length;

  // ---------------------------------------------------------------- beat commands

  void _insertBeat(EditorState s) {
    final beats = s.service.beats(s.activeTrack, s.cursor.bar, s.cursor.voice);
    final index = (s.cursor.beat ?? beats.length);
    unawaited(
      _editor.execute(
        CommandDto.insertBeat(
          track: s.activeTrack,
          bar: s.cursor.bar,
          voice: s.cursor.voice,
          index: index,
          duration: const DurationDto(base: 2, dots: 0),
        ),
      ),
    );
  }

  void _deleteNote(EditorState s) {
    final beats = s.service.beats(s.activeTrack, s.cursor.bar, s.cursor.voice);
    final beatIndex = s.cursor.beat;
    if (beatIndex == null || beatIndex >= beats.length) return;
    final beat = beats[beatIndex];
    final note = beat.notes.firstWhere(
      (n) => n.string == s.cursor.string,
      orElse: () =>
          beat.notes.isEmpty ? throw Exception('no notes') : beat.notes.first,
    );
    if (beat.notes.isEmpty) return;
    unawaited(
      _editor.execute(
        CommandDto.removeNote(
          beat: BeatAddressDto(
            track: s.activeTrack,
            bar: s.cursor.bar,
            voice: s.cursor.voice,
            beat: beat.id,
          ),
          note: note.id,
        ),
      ),
    );
  }

  void _setBeatDuration(EditorState s, int exponent) {
    final beats = s.service.beats(s.activeTrack, s.cursor.bar, s.cursor.voice);
    final beatIndex = s.cursor.beat;
    if (beatIndex == null || beatIndex >= beats.length) return;
    final beat = beats[beatIndex];
    unawaited(
      _editor.execute(
        CommandDto.setBeatDuration(
          beat: BeatAddressDto(
            track: s.activeTrack,
            bar: s.cursor.bar,
            voice: s.cursor.voice,
            beat: beat.id,
          ),
          duration: DurationDto(base: exponent, dots: beat.duration.dots),
        ),
      ),
    );
  }

  void _toggleDot(EditorState s) {
    final beats = s.service.beats(s.activeTrack, s.cursor.bar, s.cursor.voice);
    final beatIndex = s.cursor.beat;
    if (beatIndex == null || beatIndex >= beats.length) return;
    final beat = beats[beatIndex];
    final dots = beat.duration.dots == 0 ? 1 : 0;
    unawaited(
      _editor.execute(
        CommandDto.setBeatDuration(
          beat: BeatAddressDto(
            track: s.activeTrack,
            bar: s.cursor.bar,
            voice: s.cursor.voice,
            beat: beat.id,
          ),
          duration: DurationDto(base: beat.duration.base, dots: dots),
        ),
      ),
    );
  }
}
