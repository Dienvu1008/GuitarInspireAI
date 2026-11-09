import 'dart:async';
import 'dart:typed_data';

import 'document_service.dart';
import 'fake_document_service.dart';
import 'generated/api/midi.dart' show ExportReportDto, ExportResult, ImportReportDto;
import 'generated/api/types.dart'
    show ApiError, PlaybackStateDto, PositionDto;
import 'playback_service.dart';

/// In-memory stand-in for the Rust playback engine, for widget tests (no audio device).
///
/// It reproduces the transport contract: play sets the position moving, stop and seek
/// change it, mute and tempo factor are session state that persist across play/stop, the
/// position stream emits while playing, and `fromMasterBar` out of range throws. It does
/// not synthesize sound and advances the tick by a fixed amount per emitted frame. The
/// shared contract suite runs against this fake and the real service.
final class FakePlaybackService implements PlaybackService {
  FakePlaybackService({this.masterBars = 4, this.ticksPerBar = 960});

  final int masterBars;
  final int ticksPerBar;

  int _tick = 0;
  bool _playing = false;
  PlaybackStateDto _state = PlaybackStateDto.stopped;
  final Set<int> _muted = {};
  final Set<int> _soloed = {};
  final Map<int, ({int volume, int pan})> _mix = {};

  /// Every accepted [setTrackMix] call, in order (tests count commands with it).
  final List<({int track, int volume, int pan})> mixCalls = [];
  int _tempoFactor = TempoFactorSteps.normal;
  String? _error;
  Timer? _ticker;
  final StreamController<PositionDto> _positions =
      StreamController<PositionDto>.broadcast();

  int get tempoFactor => _tempoFactor;
  bool isMuted(int track) => _muted.contains(track);
  bool isSoloed(int track) => _soloed.contains(track);

  /// The mix the fake holds for [track]; the defaults when none was set.
  ({int volume, int pan}) mixOf(int track) =>
      _mix[track] ??
      (volume: MixRanges.volumeDefault, pan: MixRanges.panDefault);

  int get _endTick => masterBars * ticksPerBar;

  PositionDto _snapshot() =>
      PositionDto(tick: _tick, playing: _playing, state: _state);

  void _emit() {
    if (!_positions.isClosed) _positions.add(_snapshot());
  }

  /// Test hook: when set, the first Play fails with this message instead of starting,
  /// mirroring the lazy SoundFont load failing on the control thread (Q10). Cleared
  /// after it fires once.
  String? failOnPlay;

  @override
  Future<void> play({int? fromMasterBar}) async {
    if (fromMasterBar != null) {
      if (fromMasterBar < 0 || fromMasterBar >= masterBars) {
        _state = PlaybackStateDto.stopped;
        throw ApiError(
          rule: 'transport',
          message: 'master bar $fromMasterBar is past the end',
        );
      }
      _tick = fromMasterBar * ticksPerBar;
    }
    final fail = failOnPlay;
    if (fail != null) {
      failOnPlay = null;
      failWith(fail);
      throw ApiError(rule: 'transport', message: fail);
    }
    _playing = true;
    _state = PlaybackStateDto.playing;
    _ticker ??= Timer.periodic(const Duration(milliseconds: 16), (_) {
      // Advance proportional to the tempo factor; stop at the end.
      _tick += (ticksPerBar ~/ 60) * _tempoFactor ~/ 100;
      if (_tick >= _endTick) {
        _tick = _endTick;
        _playing = false;
        _state = PlaybackStateDto.stopped;
        _ticker?.cancel();
        _ticker = null;
      }
      _emit();
    });
    _emit();
  }

  @override
  Future<void> stop() async {
    _playing = false;
    _state = PlaybackStateDto.stopped;
    _ticker?.cancel();
    _ticker = null;
    _emit();
  }

  @override
  Future<void> seek(int tick) async {
    _tick = tick.clamp(0, _endTick);
    _emit();
  }

  @override
  Future<void> setMuted(int track, bool muted) async {
    if (muted) {
      _muted.add(track);
    } else {
      _muted.remove(track);
    }
    _emit();
  }

  @override
  Future<void> setSoloed(int track, bool soloed) async {
    if (soloed) {
      _soloed.add(track);
    } else {
      _soloed.remove(track);
    }
    _emit();
  }

  @override
  Future<void> setTrackMix(
    int track, {
    required int volume,
    required int pan,
  }) async {
    // Same rejection as the core: typed error, nothing changes.
    if (volume < MixRanges.volumeMin || volume > MixRanges.volumeMax) {
      throw ApiError(
        rule: 'transport',
        message: 'invalid mix: Volume out of range: $volume',
      );
    }
    if (pan < MixRanges.panMin || pan > MixRanges.panMax) {
      throw ApiError(
        rule: 'transport',
        message: 'invalid mix: Pan out of range: $pan',
      );
    }
    _mix[track] = (volume: volume, pan: pan);
    mixCalls.add((track: track, volume: volume, pan: pan));
    _emit();
  }

  @override
  Future<void> setTempoFactor(int hundredths) async {
    _tempoFactor = TempoFactorSteps.snap(hundredths);
    _emit();
  }

  @override
  Future<void> refreshAfterEdit() async {
    // Nothing to recompile in the fake; a real engine would swap the timeline.
  }

  @override
  PositionDto position() => _snapshot();

  @override
  String? get lastError => _error;

  /// Test hook: make the fake report a playback failure (missing SoundFont, lost device).
  void failWith(String message) {
    _error = message;
    _playing = false;
    _state = PlaybackStateDto.failed;
    _emit();
  }

  @override
  Stream<PositionDto> get positions => _positions.stream;

  @override
  void close() {
    _ticker?.cancel();
    unawaited(_positions.close());
  }
}

/// Fake [PlaybackFactory]: one fake service per document.
final class FakePlaybackFactory implements PlaybackFactory {
  FakePlaybackFactory({this.masterBars = 4});

  final int masterBars;
  final List<FakePlaybackService> services = [];

  @override
  PlaybackService forDocument(DocumentService document) {
    final s = FakePlaybackService(masterBars: masterBars);
    services.add(s);
    return s;
  }
}

/// Fake [MidiService]: import/export stubs that return an empty document and a trivial
/// report, enough for UI wiring tests.
final class FakeMidiService implements MidiService {
  FakeMidiService({this.masterBars = 4});

  final int masterBars;

  @override
  Future<MidiImport> importMidi(List<int> bytes) async => MidiImport(
    document: FakeDocumentService(bars: masterBars),
    report: const ImportReportDto(
      tracksImported: 1,
      notesImported: 0,
      bars: 1,
      quantizedPoints: 0,
      tripletBars: 0,
      issues: [],
    ),
  );

  @override
  Future<ExportResult> exportMidi(DocumentService document) async => ExportResult(
    bytes: Uint8List.fromList(const [0x4d, 0x54, 0x68, 0x64]),
    report: const ExportReportDto(
      tracks: 2,
      notes: 0,
      roundedTicks: 0,
      tempoClamped: 0,
      timeSignatures: 1,
    ),
  );
}
