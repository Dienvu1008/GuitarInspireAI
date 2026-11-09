import 'dart:async';

import 'package:core_bridge/core_bridge.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'clipboard.dart';
import 'document_controller.dart';
import 'editor_cursor.dart';
import 'key_bindings.dart';

/// Handles keyboard input for the TAB editor. No musical rules here: intents become
/// commands that the core validates (GR-05).
///
/// Create one handler per editor and keep it for the editor's lifetime (multi-digit
/// fret entry state lives here); call [dispose] when the editor goes away.
///
/// Core commands complete asynchronously, so key actions run one at a time in arrival
/// order, and each reads the editor state when it runs, not when the key was pressed.
/// Otherwise a digit typed right after Insert, or two digits typed quickly, would act
/// on a document that does not contain the previous key's result yet.
final class KeyboardHandler {
  KeyboardHandler(this._ref);

  final WidgetRef _ref;

  Future<void> _queue = Future<void>.value();
  bool _disposed = false;

  // Multi-digit fret entry (Q-15): digits typed into the same note within
  // [coalesceWindowMs] ms form one fret and one undo step.
  int? _pendingFret;
  EditorCursor? _pendingAt;
  int? _coalesceKey;
  Timer? _coalesceTimer;
  int _nextCoalesceKey = DateTime.now().microsecondsSinceEpoch;

  DocumentController get _editor => _ref.read(editorProvider.notifier);
  EditorState get _state => _ref.read(editorProvider);

  /// Completes when every key action received so far has finished.
  Future<void> get idle => _queue;

  void dispose() {
    _disposed = true;
    _cancelCoalesce();
  }

  KeyEventResult handleKey(KeyEvent event) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }
    final pressed = HardwareKeyboard.instance.logicalKeysPressed;
    final intent = intentOf(event.logicalKey, pressed);
    if (intent == null) return KeyEventResult.ignored;
    _enqueue(() => _handle(intent));
    return KeyEventResult.handled;
  }

  void _enqueue(Future<void> Function() action) {
    _queue = _queue.then((_) => _disposed ? null : action()).catchError((
      Object error,
      StackTrace stack,
    ) {
      // Rejections from the core are already status messages (DocumentController);
      // anything else is a bug and is reported, without blocking later keys.
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stack,
          library: 'editor_feature',
          context: ErrorDescription('while handling a TAB editor key'),
        ),
      );
    });
  }

  Future<void> _handle(EditorIntent intent) async {
    final digit = intent.index - EditorIntent.digit0.index;
    if (digit >= 0 && digit <= 9) {
      await _enterDigit(digit);
      return;
    }
    // Every other key ends multi-digit entry.
    _cancelCoalesce();
    final s = _state;
    switch (intent) {
      case EditorIntent.undo:
        await _editor.undo();
      case EditorIntent.redo:
        await _editor.redo();
      case EditorIntent.newDocument:
        await _editor.newDocument();
      case EditorIntent.open:
        await _editor.open();
      case EditorIntent.save:
        await _editor.save();
      case EditorIntent.saveAs:
        await _editor.saveAs();
      case EditorIntent.moveLeft:
        _editor.moveCursor(_prevBeat(s));
      case EditorIntent.moveRight:
        _editor.moveCursor(_nextBeat(s));
      case EditorIntent.moveUp:
        _editor.moveCursor(_withString(s, s.cursor.string - 1));
      case EditorIntent.moveDown:
        _editor.moveCursor(_withString(s, s.cursor.string + 1));
      case EditorIntent.deleteNote:
        await _deleteNote(s);
      case EditorIntent.insertBeat:
        await _insertBeat(s);
      case EditorIntent.durationWhole:
        await _setBeatDuration(s, 0);
      case EditorIntent.durationHalf:
        await _setBeatDuration(s, 1);
      case EditorIntent.durationQuarter:
        await _setBeatDuration(s, 2);
      case EditorIntent.durationEighth:
        await _setBeatDuration(s, 3);
      case EditorIntent.durationSixteenth:
        await _setBeatDuration(s, 4);
      case EditorIntent.dot:
        await _toggleDot(s);
      case EditorIntent.selectLeft:
        final c = _stepBeat(s, forward: false);
        if (c != null) _editor.moveCursor(c, extend: true);
      case EditorIntent.selectRight:
        final c = _stepBeat(s, forward: true);
        if (c != null) _editor.moveCursor(c, extend: true);
      case EditorIntent.clearSelection:
        _editor.moveCursor(s.cursor);
      case EditorIntent.copy:
        _copy(s);
      case EditorIntent.cut:
        await _cut(s);
      case EditorIntent.paste:
        await _paste(s);
      default:
        break;
    }
  }

  /// Edit menu entry points; they run in the same queue as keys.
  void copy() => _enqueue(() => _handle(EditorIntent.copy));
  void cut() => _enqueue(() => _handle(EditorIntent.cut));
  void paste() => _enqueue(() => _handle(EditorIntent.paste));

  // ---------------------------------------------------------- selection, clipboard

  /// The next or previous existing beat in the cursor's voice, across bars and
  /// skipping empty bars and append slots, so a Shift selection always ends on a beat.
  EditorCursor? _stepBeat(EditorState s, {required bool forward}) {
    final bars = s.summary.masterBars.length;
    final voice = s.cursor.voice;
    int len(int bar) => bar < 0 || bar >= bars
        ? 0
        : (voice < s.service.voiceCount(s.activeTrack, bar)
              ? s.service.beats(s.activeTrack, bar, voice).length
              : 0);
    EditorCursor at(int bar, int index) => EditorCursor(
      bar: bar,
      voice: voice,
      string: s.cursor.string,
      beat: index,
    );
    var bar = s.cursor.bar;
    final cur = s.cursor.beat;
    if (forward) {
      if (cur != null && cur + 1 < len(bar)) return at(bar, cur + 1);
      // Past the last beat (or from the append slot): first beat of the next bar
      // that has beats.
      for (bar = bar + 1; bar < bars; bar++) {
        if (len(bar) > 0) return at(bar, 0);
      }
      return null;
    }
    if (cur != null && cur > 0) return at(bar, cur - 1);
    if (cur == null && len(bar) > 0) return at(bar, len(bar) - 1);
    for (bar = bar - 1; bar >= 0; bar--) {
      if (len(bar) > 0) return at(bar, len(bar) - 1);
    }
    return null;
  }

  /// The selection, or the single beat under the cursor.
  (BeatSlot, BeatSlot)? _range(EditorState s) {
    final sel = s.selection;
    if (sel != null) return sel;
    final beat = s.cursor.beat;
    if (beat == null) return null;
    final slot = (bar: s.cursor.bar, voice: s.cursor.voice, index: beat);
    return (slot, slot);
  }

  static String _beats(int n) => '$n beat${n == 1 ? '' : 's'}';

  List<ClipBeatDto>? _copy(EditorState s) {
    final r = _range(s);
    if (r == null) {
      _editor.showMessage('Nothing to copy: put the cursor on a beat');
      return null;
    }
    final (from, to) = r;
    try {
      final clip = s.service.copyBeats(
        s.activeTrack,
        from.voice,
        fromBar: from.bar,
        fromIndex: from.index,
        toBar: to.bar,
        toIndex: to.index,
      );
      _ref.read(beatClipboardProvider.notifier).set(clip);
      _editor.showMessage('Copied ${_beats(clip.length)}');
      return clip;
    } on ApiError catch (e) {
      _editor.showMessage('${e.rule}: ${e.message}');
      return null;
    }
  }

  /// Copy, then remove the run in one core command (one undo step).
  Future<void> _cut(EditorState s) async {
    final clip = _copy(s);
    if (clip == null) return;
    final (from, to) = _range(s)!;
    final ok = await _editor.execute(
      CommandDto.removeBeats(
        track: s.activeTrack,
        voice: from.voice,
        fromBar: from.bar,
        fromIndex: from.index,
        toBar: to.bar,
        toIndex: to.index,
      ),
    );
    if (!ok) return;
    final left = _state.service.beats(s.activeTrack, from.bar, from.voice);
    _editor.moveCursor(
      EditorCursor(
        bar: from.bar,
        voice: from.voice,
        string: s.cursor.string,
        beat: from.index < left.length ? from.index : null,
      ),
    );
    _editor.showMessage('Cut ${_beats(clip.length)}');
  }

  /// Inserts the clipboard at the cursor (before the beat under it, or at the end of
  /// the bar on the append slot), in one core command. The core checks every note
  /// against this track and rejects the whole paste if one does not fit.
  Future<void> _paste(EditorState s) async {
    final clip = _ref.read(beatClipboardProvider);
    if (clip == null || clip.isEmpty) {
      _editor.showMessage('The clipboard is empty');
      return;
    }
    final index =
        s.cursor.beat ??
        s.service.beats(s.activeTrack, s.cursor.bar, s.cursor.voice).length;
    final ok = await _editor.execute(
      CommandDto.pasteBeats(
        track: s.activeTrack,
        bar: s.cursor.bar,
        voice: s.cursor.voice,
        index: index,
        beats: clip,
      ),
    );
    if (!ok) return;
    _editor.moveCursor(
      EditorCursor(
        bar: s.cursor.bar,
        voice: s.cursor.voice,
        string: s.cursor.string,
        beat: index,
      ),
    );
    _editor.showMessage('Pasted ${_beats(clip.length)}');
  }

  // ---------------------------------------------------------------- digit entry

  Future<void> _enterDigit(int digit) async {
    final s = _state;
    final continuing =
        _pendingFret != null &&
        _pendingAt == s.cursor &&
        _pendingFret! * 10 + digit <= 99;
    final int fret;
    if (continuing) {
      fret = _pendingFret! * 10 + digit;
    } else {
      _cancelCoalesce();
      fret = digit;
      _coalesceKey = _nextCoalesceKey++;
    }
    final key = _coalesceKey;

    final track = s.activeTrack;
    final bar = s.cursor.bar;
    final voice = s.cursor.voice;
    var beatIndex = s.cursor.beat;
    if (beatIndex == null) {
      // Cursor on the append slot: add a quarter beat first, in the same undo step.
      final index = s.service.beats(track, bar, voice).length;
      final inserted = await _editor.execute(
        CommandDto.insertBeat(
          track: track,
          bar: bar,
          voice: voice,
          index: index,
          duration: const DurationDto(base: 2, dots: 0),
        ),
        coalesceKey: key,
      );
      if (!inserted) {
        _cancelCoalesce();
        return;
      }
      beatIndex = index;
      _editor.moveCursor(
        EditorCursor(
          bar: bar,
          voice: voice,
          string: s.cursor.string,
          beat: index,
        ),
      );
    }
    final beats = _state.service.beats(track, bar, voice);
    if (beatIndex >= beats.length) {
      _cancelCoalesce();
      return;
    }
    final ok = await _editor.execute(
      CommandDto.setNote(
        beat: BeatAddressDto(
          track: track,
          bar: bar,
          voice: voice,
          beat: beats[beatIndex].id,
        ),
        string: s.cursor.string,
        fret: fret,
      ),
      coalesceKey: key,
    );
    if (!ok) {
      _cancelCoalesce();
      return;
    }
    _pendingFret = fret;
    _pendingAt = _state.cursor;
    _coalesceTimer?.cancel();
    _coalesceTimer = Timer(
      const Duration(milliseconds: coalesceWindowMs),
      _flushDigit,
    );
  }

  void _flushDigit() {
    _pendingFret = null;
    _pendingAt = null;
    _coalesceKey = null;
    _coalesceTimer = null;
  }

  void _cancelCoalesce() {
    _coalesceTimer?.cancel();
    _flushDigit();
  }

  // ---------------------------------------------------------------- navigation

  EditorCursor _withString(EditorState s, int string) => EditorCursor(
    bar: s.cursor.bar,
    voice: s.cursor.voice,
    string: string.clamp(1, _strings(s)),
    beat: s.cursor.beat,
  );

  EditorCursor _nextBeat(EditorState s) {
    final beats = s.service.beats(s.activeTrack, s.cursor.bar, s.cursor.voice);
    final cur = s.cursor.beat;
    if (cur != null) {
      // Next beat, or the append slot after the last one.
      return EditorCursor(
        bar: s.cursor.bar,
        voice: s.cursor.voice,
        string: s.cursor.string,
        beat: cur + 1 < beats.length ? cur + 1 : null,
      );
    }
    // From the append slot to the first beat of the next bar.
    if (s.cursor.bar + 1 < s.summary.masterBars.length) {
      final next = s.service.beats(
        s.activeTrack,
        s.cursor.bar + 1,
        s.cursor.voice,
      );
      return EditorCursor(
        bar: s.cursor.bar + 1,
        voice: s.cursor.voice,
        string: s.cursor.string,
        beat: next.isEmpty ? null : 0,
      );
    }
    return s.cursor;
  }

  EditorCursor _prevBeat(EditorState s) {
    final beats = s.service.beats(s.activeTrack, s.cursor.bar, s.cursor.voice);
    final cur = s.cursor.beat;
    if (cur != null && cur > 0) {
      return EditorCursor(
        bar: s.cursor.bar,
        voice: s.cursor.voice,
        string: s.cursor.string,
        beat: cur - 1,
      );
    }
    if (cur == null && beats.isNotEmpty) {
      // From the append slot to the last beat of the same bar.
      return EditorCursor(
        bar: s.cursor.bar,
        voice: s.cursor.voice,
        string: s.cursor.string,
        beat: beats.length - 1,
      );
    }
    if (s.cursor.bar > 0) {
      // To the append slot of the previous bar.
      return EditorCursor(
        bar: s.cursor.bar - 1,
        voice: s.cursor.voice,
        string: s.cursor.string,
      );
    }
    return s.cursor;
  }

  int _strings(EditorState s) => s.summary.tracks
      .firstWhere((t) => t.id == s.activeTrack)
      .instrument
      .tuning
      .length;

  // ---------------------------------------------------------------- beat commands

  BeatDto? _beatAtCursor(EditorState s) {
    final index = s.cursor.beat;
    if (index == null) return null;
    final beats = s.service.beats(s.activeTrack, s.cursor.bar, s.cursor.voice);
    return index < beats.length ? beats[index] : null;
  }

  BeatAddressDto _address(EditorState s, BeatDto beat) => BeatAddressDto(
    track: s.activeTrack,
    bar: s.cursor.bar,
    voice: s.cursor.voice,
    beat: beat.id,
  );

  Future<void> _insertBeat(EditorState s) async {
    final beats = s.service.beats(s.activeTrack, s.cursor.bar, s.cursor.voice);
    final index = s.cursor.beat ?? beats.length;
    final ok = await _editor.execute(
      CommandDto.insertBeat(
        track: s.activeTrack,
        bar: s.cursor.bar,
        voice: s.cursor.voice,
        index: index,
        duration: const DurationDto(base: 2, dots: 0),
      ),
    );
    if (ok) {
      _editor.moveCursor(
        EditorCursor(
          bar: s.cursor.bar,
          voice: s.cursor.voice,
          string: s.cursor.string,
          beat: index,
        ),
      );
    }
  }

  /// Removes the note on the cursor's string; does nothing if there is none.
  Future<void> _deleteNote(EditorState s) async {
    final beat = _beatAtCursor(s);
    if (beat == null) return;
    final note = beat.notes
        .where((n) => n.string == s.cursor.string)
        .firstOrNull;
    if (note == null) return;
    await _editor.execute(
      CommandDto.removeNote(beat: _address(s, beat), note: note.id),
    );
  }

  Future<void> _setBeatDuration(EditorState s, int exponent) async {
    final beat = _beatAtCursor(s);
    if (beat == null) return;
    await _editor.execute(
      CommandDto.setBeatDuration(
        beat: _address(s, beat),
        duration: DurationDto(base: exponent, dots: beat.duration.dots),
      ),
    );
  }

  Future<void> _toggleDot(EditorState s) async {
    final beat = _beatAtCursor(s);
    if (beat == null) return;
    await _editor.execute(
      CommandDto.setBeatDuration(
        beat: _address(s, beat),
        duration: DurationDto(
          base: beat.duration.base,
          dots: beat.duration.dots == 0 ? 1 : 0,
        ),
      ),
    );
  }
}
