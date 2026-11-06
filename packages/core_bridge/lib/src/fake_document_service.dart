import 'dart:async';
import 'dart:typed_data';

import 'document_service.dart';
import 'generated/api/document.dart' show RecoveryEntryDto;
import 'generated/api/layout.dart';
import 'generated/api/types.dart';

/// In-memory stand-in for the Rust document session, for widget tests.
///
/// It reproduces the interaction contract (revisions, gapless events, undo and redo,
/// coalescing, rejected commands leave the document unchanged, a layout with one fret
/// glyph and one hit region per string for every beat). It does not implement musical
/// rules: it rejects only what it cannot address (unknown track, bar or beat) and
/// commands outside its small subset. The shared contract suite runs against this fake
/// and the real service so the two cannot drift.
final class FakeDocumentService implements DocumentService {
  FakeDocumentService({int bars = 4})
    : _state = _State(
        tracks: [
          _Track(
            1,
            'Guitar',
            _standard(),
            List.generate(bars, (_) => [<_Beat>[]]),
          ),
        ],
        bars: bars,
      );

  static InstrumentDto _standard() => const InstrumentDto(
    tuning: ['E4', 'B3', 'G3', 'D3', 'A2', 'E2'],
    fretCount: 24,
    capo: 0,
  );

  _State _state;
  final List<_State> _undo = [];
  final List<_State> _redo = [];
  int? _lastCoalesceKey;
  int _revision = 0;
  int _seq = 0;
  int _nextId = 100;
  int _nextCommand = 1;
  String? _location;
  _State? _saved;
  final StreamController<EventDto> _events =
      StreamController<EventDto>.broadcast();

  bool get _dirty => !_state.sameAs(_saved);

  @override
  DocumentSummaryDto summary() => DocumentSummaryDto(
    revision: _revision,
    dirty: _dirty,
    canUndo: _undo.isNotEmpty,
    canRedo: _redo.isNotEmpty,
    location: _location,
    unknownContent: const [],
    tracks: [
      for (final t in _state.tracks)
        TrackSummaryDto(id: t.id, name: t.name, instrument: t.instrument),
    ],
    masterBars: List.generate(
      _state.bars,
      (_) => MasterBarDto(
        numerator: 4,
        denominator: 4,
        pickup: false,
        repeatStart: false,
        alternateEndings: Uint8List(0),
      ),
    ),
    diagnostics: const [],
  );

  _Track _track(int id) => _state.tracks.firstWhere(
    (t) => t.id == id,
    orElse: () => throw ApiError(rule: 'not-found', message: 'track $id'),
  );

  List<_Beat> _voice(int track, int bar, int voice) {
    final t = _track(track);
    if (bar < 0 ||
        bar >= t.bars.length ||
        voice < 0 ||
        voice >= t.bars[bar].length) {
      throw ApiError(rule: 'not-found', message: 'bar $bar, voice $voice');
    }
    return t.bars[bar][voice];
  }

  _Beat _beat(BeatAddressDto a) => _voice(a.track, a.bar, a.voice).firstWhere(
    (b) => b.id == a.beat,
    orElse: () => throw ApiError(rule: 'not-found', message: 'beat ${a.beat}'),
  );

  @override
  Future<OutcomeDto> execute(
    CommandDto command, {
    int? coalesceKey,
    int? expectedRevision,
  }) async {
    if (expectedRevision != null && expectedRevision != _revision) {
      throw ApiError(rule: 'E-01', message: 'stale revision');
    }
    final before = _state.copy();
    final next = _state.copy();
    final previous = _state;
    _state = next;
    final List<EventKindDto> kinds;
    try {
      kinds = _apply(command);
    } catch (_) {
      _state = previous;
      rethrow;
    }
    final coalesce =
        coalesceKey != null &&
        coalesceKey == _lastCoalesceKey &&
        _undo.isNotEmpty;
    if (!coalesce) _undo.add(before);
    _redo.clear();
    _lastCoalesceKey = coalesceKey;
    return _commit(kinds, _nextCommand++);
  }

  List<EventKindDto> _apply(CommandDto c) {
    switch (c) {
      case CommandDto_InsertBeat(
        :final track,
        :final bar,
        :final voice,
        :final index,
        :final duration,
      ):
        final v = _voice(track, bar, voice);
        if (index < 0 || index > v.length) {
          throw ApiError(rule: 'E-04', message: 'beat index $index');
        }
        v.insert(index, _Beat(_nextId++, duration, []));
        return [
          EventKindDto.barContentChanged(
            track: track,
            start: bar,
            end: bar + 1,
          ),
        ];
      case CommandDto_RemoveBeat(:final beat):
        final b = _beat(beat);
        _voice(beat.track, beat.bar, beat.voice).remove(b);
        return [_changed(beat)];
      case CommandDto_SetNote(:final beat, :final string, :final fret):
        final b = _beat(beat);
        final i = b.notes.indexWhere((n) => n.string == string);
        if (i >= 0) {
          final n = b.notes[i];
          b.notes[i] = NoteDto(
            id: n.id,
            string: n.string,
            fret: fret,
            tie: n.tie,
            letRing: n.letRing,
            dead: n.dead,
          );
        } else {
          b.notes.add(
            NoteDto(
              id: _nextId++,
              string: string,
              fret: fret,
              tie: false,
              letRing: false,
              dead: false,
            ),
          );
          b.notes.sort((x, y) => x.string.compareTo(y.string));
        }
        return [_changed(beat)];
      case CommandDto_RemoveNote(:final beat, :final note):
        final b = _beat(beat);
        final before = b.notes.length;
        b.notes.removeWhere((n) => n.id == note);
        if (b.notes.length == before) {
          throw ApiError(rule: 'not-found', message: 'note $note');
        }
        return [_changed(beat)];
      case CommandDto_AddTrack(:final name, :final instrument):
        final id = _nextId++;
        _state.tracks.add(
          _Track(
            id,
            name,
            instrument,
            List.generate(_state.bars, (_) => [<_Beat>[]]),
          ),
        );
        return [EventKindDto.trackAdded(track: id)];
      case CommandDto_RenameTrack(:final track, :final name):
        _track(track).name = name;
        return [EventKindDto.trackChanged(track: track)];
      case CommandDto_InsertMasterBars(:final at, :final count):
        if (at < 0 || at > _state.bars || count <= 0) {
          throw ApiError(rule: 'E-04', message: 'insert $count at $at');
        }
        for (final t in _state.tracks) {
          t.bars.insertAll(at, List.generate(count, (_) => [<_Beat>[]]));
        }
        _state.bars += count;
        return [
          EventKindDto.masterBarsInserted(at: at, count: count),
          const EventKindDto.tempoMapChanged(),
        ];
      default:
        throw ApiError(rule: 'unsupported', message: 'FakeDocumentService: $c');
    }
  }

  EventKindDto _changed(BeatAddressDto a) => EventKindDto.barContentChanged(
    track: a.track,
    start: a.bar,
    end: a.bar + 1,
  );

  OutcomeDto _commit(List<EventKindDto> kinds, int? correlation) {
    _revision++;
    final events = [
      for (final k in kinds)
        EventDto(
          seq: ++_seq,
          revision: _revision,
          correlation: correlation,
          kind: k,
        ),
    ];
    for (final e in events) {
      _events.add(e);
    }
    return OutcomeDto(revision: _revision, events: events);
  }

  @override
  Future<OutcomeDto> undo() async {
    if (_undo.isEmpty) throw ApiError(rule: 'E-05', message: 'nothing to undo');
    _redo.add(_state);
    _state = _undo.removeLast();
    _lastCoalesceKey = null;
    return _commit([const EventKindDto.historyApplied()], null);
  }

  @override
  Future<OutcomeDto> redo() async {
    if (_redo.isEmpty) throw ApiError(rule: 'E-05', message: 'nothing to redo');
    _undo.add(_state);
    _state = _redo.removeLast();
    _lastCoalesceKey = null;
    return _commit([const EventKindDto.historyApplied()], null);
  }

  @override
  Future<OutcomeDto> save() async {
    if (_location == null) {
      throw ApiError(rule: 'save-location', message: 'not saved yet');
    }
    return saveAs(_location!);
  }

  @override
  Future<OutcomeDto> saveAs(String path) async {
    _location = path;
    _saved = _state.copy();
    return _commit([const EventKindDto.documentSaved()], null);
  }

  @override
  List<BeatDto> beats(int track, int bar, int voice) => [
    for (final b in _voice(track, bar, voice))
      BeatDto(id: b.id, duration: b.duration, notes: List.of(b.notes)),
  ];

  @override
  int voiceCount(int track, int bar) {
    final t = _track(track);
    return bar >= 0 && bar < t.bars.length ? t.bars[bar].length : 0;
  }

  /// A fixed voicing set for widget tests: "x32010" (open C shape) regardless of the
  /// symbol, plus a barre. A bad symbol (empty) throws, to exercise the error path.
  @override
  ChordResultDto chordVoicings(int track, String symbol, {int max = 5}) {
    _track(track); // validate the track exists
    if (symbol.trim().isEmpty) {
      throw ApiError(rule: 'chord', message: 'empty chord symbol');
    }
    final open = VoicingDto(
      frets: Int32List.fromList([0, 1, 0, 2, 3, -1]), // string 1..6 (x32010)
      chartText: 'x32010',
      fingers: 3,
      lowestFret: 1,
      highestFret: 3,
    );
    final barre = VoicingDto(
      frets: Int32List.fromList([3, 5, 5, 5, 3, -1]),
      chartText: 'x35553',
      fingers: 3,
      lowestFret: 3,
      highestFret: 5,
    );
    return ChordResultDto(
      symbol: symbol,
      tones: const ['C', 'E', 'G'],
      voicings: [open, barre].take(max).toList(),
    );
  }

  /// One played bar per master bar, each 960 ticks (no repeats in the fake).
  @override
  List<PlayedBarDto> playedBars() => [
    for (var b = 0; b < _state.bars; b++)
      PlayedBarDto(
        order: b,
        masterBar: b,
        startTick: b * 960,
        lengthTicks: 960,
      ),
  ];

  /// One system, one fixed-width column per beat; enough for widget tests.
  @override
  TabLayoutDto tabLayout(int track, MetricsDto metrics) {
    final t = _track(track);
    final strings = t.instrument.tuning.length;
    final glyphs = <GlyphDto>[];
    final rects = <double>[];
    final meta = <int>[];
    final hitBeats = <HitBeatDto>[];
    final bars = <BarLayoutDto>[];
    var x = metrics.marginLeft + metrics.headerWidth;
    final top = metrics.aboveStaff;
    for (var bar = 0; bar < t.bars.length; bar++) {
      final start = x;
      final beats = t.bars[bar][0];
      for (var k = 0; k < beats.length; k++) {
        final w = metrics.minColumnWidth;
        for (final n in beats[k].notes) {
          glyphs.add(
            GlyphDto(
              kind: GlyphKindDto.fret(
                fret: n.fret,
                tied: n.tie,
                dead: n.dead,
                letRing: n.letRing,
              ),
              x: x + w / 2,
              y: top + (n.string - 1) * metrics.stringSpacing,
              width: metrics.digitWidth,
            ),
          );
        }
        hitBeats.add(HitBeatDto(id: beats[k].id, bar: bar, voice: 0, index: k));
        for (var s = 1; s <= strings; s++) {
          rects.addAll([
            x,
            top + (s - 1.5) * metrics.stringSpacing,
            w,
            metrics.stringSpacing,
          ]);
          meta.addAll([bar, s, 0, hitBeats.length - 1]);
        }
        x += w;
      }
      x += beats.isEmpty ? metrics.emptyBarWidth : metrics.barPadding;
      bars.add(
        BarLayoutDto(bar: bar, x: start, width: x - start, diagnostic: false),
      );
    }
    final lines = <double>[];
    final kinds = <int>[];
    for (var s = 0; s < strings; s++) {
      final y = top + s * metrics.stringSpacing;
      lines.addAll([metrics.marginLeft, y, x, y]);
      kinds.add(LineKind.string);
    }
    final height =
        top + (strings - 1) * metrics.stringSpacing + metrics.belowStaff;
    return TabLayoutDto(
      track: track,
      strings: strings,
      width: metrics.width,
      height: height,
      systems: [
        SystemDto(
          index: 0,
          y: 0,
          height: height,
          staffTop: top,
          bars: bars,
          lines: Float32List.fromList(lines),
          lineKinds: Uint8List.fromList(kinds),
          glyphs: glyphs,
          hitRects: Float32List.fromList(rects),
          hitMeta: Int32List.fromList(meta),
          beats: hitBeats,
        ),
      ],
    );
  }

  @override
  Stream<EventDto> get events => _events.stream;

  @override
  bool autosaveDue() => _dirty;

  @override
  Future<void> writeAutosave(String dir) async {
    // No-op in the fake; just mark the autosave done.
  }

  @override
  void close() {
    unawaited(_events.close());
  }
}

/// Fake [DocumentFactory] creating [FakeDocumentService]s.
final class FakeDocumentFactory implements DocumentFactory {
  final Map<String, FakeDocumentService> _files = {};

  @override
  DocumentService create({int bars = 4}) =>
      _Tracked(FakeDocumentService(bars: bars), _files);

  @override
  Future<DocumentService> open(String path) async {
    final doc = _files[path];
    if (doc == null) throw ApiError(rule: 'open', message: 'no file $path');
    return doc;
  }

  /// Recovery entries per directory; tests seed it to simulate an earlier crash.
  final Map<String, List<RecoveryEntryDto>> recovery = {};

  @override
  List<RecoveryEntryDto> recoveryEntries(String dir) =>
      List.unmodifiable(recovery[dir] ?? const <RecoveryEntryDto>[]);

  /// Returns a new document; the fake keeps no autosave content.
  @override
  Future<DocumentService> openFromRecovery(String dir, String projectId) async {
    final found = (recovery[dir] ?? const <RecoveryEntryDto>[]).any(
      (e) => e.projectId == projectId,
    );
    if (!found) {
      throw ApiError(rule: 'not-found', message: 'recovery for $projectId');
    }
    return create();
  }

  @override
  void discardRecovery(String dir, String projectId) =>
      recovery[dir]?.removeWhere((e) => e.projectId == projectId);

  /// Directories whose session marker is present. Tests seed it to simulate a crash;
  /// it starts empty, so a fresh fake behaves like a first, clean start.
  final Set<String> sessionMarkers = {};

  /// Modification time of source files in Unix millis, for the newer-than-source
  /// rule. A source missing from the map counts as gone (always offered), as in core.
  final Map<String, int> sourceModified = {};

  @override
  bool beginSession(String dir) => !sessionMarkers.add(dir);

  @override
  void endSession(String dir) => sessionMarkers.remove(dir);

  /// Mirrors `mc_app::recovery::offered`; the Rust contract test checks the real one.
  @override
  List<RecoveryEntryDto> offeredRecovery(String dir, {required bool unclean}) {
    if (!unclean) return const [];
    return List.unmodifiable(
      recoveryEntries(dir).where((e) {
        final src = e.source;
        if (src == null) return true;
        final modified = sourceModified[src];
        return modified == null || e.timestamp > modified;
      }),
    );
  }
}

/// Remembers documents by save location so [FakeDocumentFactory.open] can return them.
final class _Tracked implements DocumentService {
  _Tracked(this._inner, this._files);

  final FakeDocumentService _inner;
  final Map<String, FakeDocumentService> _files;

  @override
  Future<OutcomeDto> saveAs(String path) async {
    final out = await _inner.saveAs(path);
    _files[path] = _inner;
    return out;
  }

  @override
  DocumentSummaryDto summary() => _inner.summary();
  @override
  Future<OutcomeDto> execute(
    CommandDto command, {
    int? coalesceKey,
    int? expectedRevision,
  }) => _inner.execute(
    command,
    coalesceKey: coalesceKey,
    expectedRevision: expectedRevision,
  );
  @override
  Future<OutcomeDto> undo() => _inner.undo();
  @override
  Future<OutcomeDto> redo() => _inner.redo();
  @override
  Future<OutcomeDto> save() => _inner.save();
  @override
  List<BeatDto> beats(int track, int bar, int voice) =>
      _inner.beats(track, bar, voice);
  @override
  int voiceCount(int track, int bar) => _inner.voiceCount(track, bar);
  @override
  List<PlayedBarDto> playedBars() => _inner.playedBars();
  @override
  ChordResultDto chordVoicings(int track, String symbol, {int max = 5}) =>
      _inner.chordVoicings(track, symbol, max: max);
  @override
  TabLayoutDto tabLayout(int track, MetricsDto metrics) =>
      _inner.tabLayout(track, metrics);
  @override
  Stream<EventDto> get events => _inner.events;
  @override
  bool autosaveDue() => _inner.autosaveDue();
  @override
  Future<void> writeAutosave(String dir) => _inner.writeAutosave(dir);
  @override
  void close() => _inner.close();
}

final class _Beat {
  _Beat(this.id, this.duration, this.notes);
  final int id;
  final DurationDto duration;
  final List<NoteDto> notes;
  _Beat copy() => _Beat(id, duration, List.of(notes));
}

final class _Track {
  _Track(this.id, this.name, this.instrument, this.bars);
  final int id;
  String name;
  final InstrumentDto instrument;
  final List<List<List<_Beat>>> bars;
  _Track copy() => _Track(id, name, instrument, [
    for (final bar in bars)
      [
        for (final v in bar) [for (final b in v) b.copy()],
      ],
  ]);
}

final class _State {
  _State({required this.tracks, required this.bars});
  final List<_Track> tracks;
  int bars;

  _State copy() =>
      _State(tracks: [for (final t in tracks) t.copy()], bars: bars);

  bool sameAs(_State? other) {
    if (other == null ||
        other.bars != bars ||
        other.tracks.length != tracks.length) {
      return false;
    }
    for (var i = 0; i < tracks.length; i++) {
      final a = tracks[i];
      final b = other.tracks[i];
      if (a.id != b.id || a.name != b.name) return false;
      for (var bar = 0; bar < a.bars.length; bar++) {
        for (var v = 0; v < a.bars[bar].length; v++) {
          final x = a.bars[bar][v];
          final y = b.bars[bar][v];
          if (x.length != y.length) return false;
          for (var k = 0; k < x.length; k++) {
            if (x[k].id != y[k].id ||
                x[k].notes.toString() != y[k].notes.toString()) {
              return false;
            }
          }
        }
      }
    }
    return true;
  }
}
