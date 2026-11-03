import 'package:flutter/services.dart';

/// Q-15: digits typed into the same beat within [coalesceWindowMs] ms form one
/// undo step (same `coalesceKey`). A cursor move or a non-digit key ends the step.
const int coalesceWindowMs = 600;

/// Commands the TAB editor keyboard handler produces. One value per user intent.
enum EditorIntent {
  moveLeft,
  moveRight,
  moveUp,
  moveDown,
  digit0,
  digit1,
  digit2,
  digit3,
  digit4,
  digit5,
  digit6,
  digit7,
  digit8,
  digit9,
  deleteNote,
  insertBeat,
  durationWhole,
  durationHalf,
  durationQuarter,
  durationEighth,
  durationSixteenth,
  dot,
  undo,
  redo,
  newDocument,
  open,
  save,
  saveAs,
}

/// Maps a key event to an [EditorIntent], or null for unhandled keys. Tab is
/// deliberately unmapped so it keeps moving keyboard focus between controls.
EditorIntent? intentOf(
  LogicalKeyboardKey key,
  Set<LogicalKeyboardKey> pressed,
) {
  final ctrl =
      pressed.contains(LogicalKeyboardKey.controlLeft) ||
      pressed.contains(LogicalKeyboardKey.controlRight) ||
      pressed.contains(LogicalKeyboardKey.metaLeft) ||
      pressed.contains(LogicalKeyboardKey.metaRight);
  final shift =
      pressed.contains(LogicalKeyboardKey.shiftLeft) ||
      pressed.contains(LogicalKeyboardKey.shiftRight);
  if (ctrl) {
    return switch (key) {
      LogicalKeyboardKey.keyZ => EditorIntent.undo,
      LogicalKeyboardKey.keyY => EditorIntent.redo,
      LogicalKeyboardKey.keyN => EditorIntent.newDocument,
      LogicalKeyboardKey.keyO => EditorIntent.open,
      LogicalKeyboardKey.keyS when shift => EditorIntent.saveAs,
      LogicalKeyboardKey.keyS => EditorIntent.save,
      _ => null,
    };
  }
  return switch (key) {
    LogicalKeyboardKey.arrowLeft => EditorIntent.moveLeft,
    LogicalKeyboardKey.arrowRight => EditorIntent.moveRight,
    LogicalKeyboardKey.arrowUp => EditorIntent.moveUp,
    LogicalKeyboardKey.arrowDown => EditorIntent.moveDown,
    LogicalKeyboardKey.delete ||
    LogicalKeyboardKey.backspace => EditorIntent.deleteNote,
    LogicalKeyboardKey.insert => EditorIntent.insertBeat,
    LogicalKeyboardKey.digit0 ||
    LogicalKeyboardKey.numpad0 => EditorIntent.digit0,
    LogicalKeyboardKey.digit1 ||
    LogicalKeyboardKey.numpad1 => EditorIntent.digit1,
    LogicalKeyboardKey.digit2 ||
    LogicalKeyboardKey.numpad2 => EditorIntent.digit2,
    LogicalKeyboardKey.digit3 ||
    LogicalKeyboardKey.numpad3 => EditorIntent.digit3,
    LogicalKeyboardKey.digit4 ||
    LogicalKeyboardKey.numpad4 => EditorIntent.digit4,
    LogicalKeyboardKey.digit5 ||
    LogicalKeyboardKey.numpad5 => EditorIntent.digit5,
    LogicalKeyboardKey.digit6 ||
    LogicalKeyboardKey.numpad6 => EditorIntent.digit6,
    LogicalKeyboardKey.digit7 ||
    LogicalKeyboardKey.numpad7 => EditorIntent.digit7,
    LogicalKeyboardKey.digit8 ||
    LogicalKeyboardKey.numpad8 => EditorIntent.digit8,
    LogicalKeyboardKey.digit9 ||
    LogicalKeyboardKey.numpad9 => EditorIntent.digit9,
    LogicalKeyboardKey.keyW => EditorIntent.durationWhole,
    LogicalKeyboardKey.keyH => EditorIntent.durationHalf,
    LogicalKeyboardKey.keyQ => EditorIntent.durationQuarter,
    LogicalKeyboardKey.keyE => EditorIntent.durationEighth,
    LogicalKeyboardKey.keyS => EditorIntent.durationSixteenth,
    LogicalKeyboardKey.period => EditorIntent.dot,
    _ => null,
  };
}
