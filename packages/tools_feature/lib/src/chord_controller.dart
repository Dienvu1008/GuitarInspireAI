import 'package:core_bridge/core_bridge.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Looks up chord voicings for a track. The app overrides it to call the active
/// document's `chordVoicings` with the editor's active track (its tuning + capo, T8);
/// tests inject a fake. Throws [ApiError] for a bad symbol or unknown track.
typedef ChordLookupFn = ChordResultDto Function(String symbol, {int max});

final chordLookupProvider = Provider<ChordLookupFn>(
  (ref) => throw UnimplementedError('override chordLookupProvider'),
);

/// Inserts a voicing into the score at the cursor. The app wires it to issue ordinary
/// `setNote` commands through the editor (GR-05: no music rule in Dart). `null` means
/// insertion is unavailable (no cursor); the view disables the action.
typedef InsertVoicingFn = Future<void> Function(VoicingDto voicing);

final insertVoicingProvider = Provider<InsertVoicingFn?>((ref) => null);

/// What the chord finder shows: the query, the result (or null before a search), and an
/// error message for a rejected symbol.
final class ChordState {
  const ChordState({this.symbol = '', this.result, this.message});

  final String symbol;
  final ChordResultDto? result;
  final String? message;

  ChordState copyWith({
    String? symbol,
    ChordResultDto? result,
    String? message,
    bool clearResult = false,
    bool clearMessage = false,
  }) => ChordState(
    symbol: symbol ?? this.symbol,
    result: clearResult ? null : (result ?? this.result),
    message: clearMessage ? null : (message ?? this.message),
  );
}

final chordProvider = NotifierProvider<ChordController, ChordState>(
  ChordController.new,
);

/// Drives the chord finder. No musical rule here: it passes the symbol to the core and
/// shows the returned voicings (GR-05).
final class ChordController extends Notifier<ChordState> {
  @override
  ChordState build() => const ChordState();

  /// Look up [symbol]. An empty symbol clears the result; a rejected symbol becomes a
  /// message, leaving the previous result in place is avoided (cleared for clarity).
  void find(String symbol, {int max = 5}) {
    final trimmed = symbol.trim();
    if (trimmed.isEmpty) {
      state = const ChordState();
      return;
    }
    try {
      final result = ref.read(chordLookupProvider)(trimmed, max: max);
      state = ChordState(symbol: trimmed, result: result);
    } on ApiError catch (e) {
      state = ChordState(symbol: trimmed, message: e.message);
    }
  }

  /// Insert [voicing] at the cursor, if insertion is wired. Returns whether it ran.
  Future<bool> insert(VoicingDto voicing) async {
    final insert = ref.read(insertVoicingProvider);
    if (insert == null) return false;
    await insert(voicing);
    return true;
  }
}
