import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// The in-app beat clipboard (ADR-030 section 1). It lives at app level, not per
/// document, so beats copied in one document can be pasted into another in the same
/// running app. It holds plain values (strings and frets, not pitches) and is never
/// written to the operating-system clipboard or to a file.
final beatClipboardProvider =
    NotifierProvider<BeatClipboard, List<ClipBeatDto>?>(BeatClipboard.new);

final class BeatClipboard extends Notifier<List<ClipBeatDto>?> {
  @override
  List<ClipBeatDto>? build() => null;

  void set(List<ClipBeatDto> beats) => state = List.unmodifiable(beats);
}
