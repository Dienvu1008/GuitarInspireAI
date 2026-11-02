import 'generated/api/layout.dart';
import 'generated/api/types.dart';

/// One open document. All musical rules are applied by the Rust core; this interface
/// only sends commands and reads projections (ADR-002). Implementations: the real one
/// backed by `mc-ffi`, and `FakeDocumentService` for widget tests. Both pass the same
/// contract test suite so the fake cannot drift.
abstract interface class DocumentService {
  /// Tracks, master bars, history and save state at the current revision.
  DocumentSummaryDto summary();

  /// Applies a command. Throws [ApiError] when the core rejects it; the document is
  /// then unchanged. Commands with the same [coalesceKey] form one undo step.
  Future<OutcomeDto> execute(CommandDto command, {int? coalesceKey, int? expectedRevision});

  Future<OutcomeDto> undo();

  Future<OutcomeDto> redo();

  /// Saves to the current location. Throws [ApiError] with rule `save-location` for a
  /// document that was never saved, or `U2` when unknown content would be lost.
  Future<OutcomeDto> save();

  Future<OutcomeDto> saveAs(String path);

  /// Beats of one voice of one bar, in order.
  List<BeatDto> beats(int track, int bar, int voice);

  int voiceCount(int track, int bar);

  /// TAB layout of a track at the current revision (synchronous; see ADR-018).
  TabLayoutDto tabLayout(int track, MetricsDto metrics);

  /// Invalidation events of this document, gapless by `seq` (ADR-006).
  Stream<EventDto> get events;

  /// Releases the document. The service must not be used afterwards.
  void close();
}

/// Creates and opens documents.
abstract interface class DocumentFactory {
  DocumentService create({int bars = 4});

  Future<DocumentService> open(String path);
}

/// Line kind codes used in [SystemDto.lineKinds].
abstract final class LineKind {
  static const int string = 0;
  static const int bar = 1;
  static const int thickBar = 2;
  static const int stem = 3;
  static const int beam = 4;
  static const int volta = 5;
}
