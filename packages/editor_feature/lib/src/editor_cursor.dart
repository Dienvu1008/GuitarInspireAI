import 'package:flutter/foundation.dart';

/// Cursor position in the active track. `beat` is the index in the voice, or `null`
/// for the append slot after the last beat. Pure UI state; addressing commands from it
/// is done in Phase 3d.
@immutable
final class EditorCursor {
  const EditorCursor({required this.bar, required this.voice, required this.string, this.beat});

  static const start = EditorCursor(bar: 0, voice: 0, string: 1);

  final int bar;
  final int voice;

  /// 1-based string number (string 1 is the highest, Q-07).
  final int string;
  final int? beat;

  @override
  bool operator ==(Object other) =>
      other is EditorCursor &&
      other.bar == bar &&
      other.voice == voice &&
      other.string == string &&
      other.beat == beat;

  @override
  int get hashCode => Object.hash(bar, voice, string, beat);

  @override
  String toString() =>
      'bar ${bar + 1}, string $string, ${beat == null ? 'end' : 'beat ${beat! + 1}'}';
}
