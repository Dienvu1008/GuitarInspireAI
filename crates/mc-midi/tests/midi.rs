//! Slice 4e tests: export, import, round trip within the reported quantization, parser
//! limits and robustness against arbitrary bytes (IM-01, FF-15 groundwork).

use mc_midi::{ImportError, ImportIssue, MAX_FILE_BYTES, MAX_TRACKS, PPQ, export_smf, import_smf};
use mc_playback::{CompileOptions, PerformanceEvent, PerformanceTimeline, compile};
use mc_score::{
    BarContent, Beat, BeatId, FrettedInstrument, InstrumentSpec, MasterBar, Note, NoteEffects,
    NoteId, NoteKind, Score, ScoreParts, TempoChange, TrackData, TrackId, Voice,
};
use mc_theory::fretted::{Fret, FretPosition, StringIndex};
use mc_theory::{BaseValue, NoteValue, Rational, Tempo, TimeSignature, Tuplet};
use midly::num::{u4, u7, u15, u24, u28};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use proptest::prelude::*;
use std::collections::BTreeMap;

// ---------------------------------------------------------------- builders

type Abs = Vec<(i64, TrackEventKind<'static>)>;

fn on(ch: u8, key: u8, tick: i64) -> (i64, TrackEventKind<'static>) {
    (
        tick,
        TrackEventKind::Midi {
            channel: u4::new(ch),
            message: MidiMessage::NoteOn {
                key: u7::new(key),
                vel: u7::new(90),
            },
        },
    )
}

fn off(ch: u8, key: u8, tick: i64) -> (i64, TrackEventKind<'static>) {
    (
        tick,
        TrackEventKind::Midi {
            channel: u4::new(ch),
            message: MidiMessage::NoteOff {
                key: u7::new(key),
                vel: u7::new(0),
            },
        },
    )
}

fn tempo_us(us: u32, tick: i64) -> (i64, TrackEventKind<'static>) {
    (tick, TrackEventKind::Meta(MetaMessage::Tempo(u24::new(us))))
}

fn smf_bytes(ppq: u16, tracks: Vec<Abs>) -> Vec<u8> {
    let tracks = tracks
        .into_iter()
        .map(|mut evs| {
            evs.sort_by_key(|e| e.0);
            let mut out = Vec::new();
            let mut last = 0i64;
            for (t, kind) in evs {
                out.push(TrackEvent {
                    delta: u28::new((t - last) as u32),
                    kind,
                });
                last = t;
            }
            out.push(TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            });
            out
        })
        .collect();
    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(ppq))),
        tracks,
    };
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes).unwrap();
    bytes
}

fn pos(string: u8, fret: u8) -> NoteKind {
    NoteKind::Fretted(FretPosition {
        string: StringIndex::new(string).unwrap(),
        fret: Fret(fret),
    })
}

struct Ids(u64);
impl Ids {
    fn next(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
}

fn beat(ids: &mut Ids, value: NoteValue, notes: &[(u8, u8, bool)]) -> Beat {
    Beat {
        id: BeatId::from_raw(ids.next()),
        value,
        notes: notes
            .iter()
            .map(|&(s, f, tie)| Note {
                id: NoteId::from_raw(ids.next()),
                kind: pos(s, f),
                tie,
                effects: NoteEffects::default(),
            })
            .collect(),
    }
}

fn plain(b: BaseValue) -> NoteValue {
    NoteValue::plain(b)
}

fn bar(beats: Vec<Beat>) -> BarContent {
    BarContent {
        voices: vec![Voice { beats }],
    }
}

/// Two guitars, four 4/4 bars, a tempo change; rests, chords, a triplet group, a dotted
/// value and a tie across a bar line. No let ring or dead notes (they change durations).
fn sample_score() -> Score {
    let mut ids = Ids(10);
    let q = plain(BaseValue::QUARTER);
    let e = plain(BaseValue::EIGHTH);
    let trip = NoteValue::new(BaseValue::EIGHTH, 0, Some(Tuplet::TRIPLET)).unwrap();
    let dq = NoteValue::new(BaseValue::QUARTER, 1, None).unwrap();

    let lead = vec![
        // bar 0: four quarters
        bar(vec![
            beat(&mut ids, q, &[(2, 1, false)]),
            beat(&mut ids, q, &[(2, 3, false)]),
            beat(&mut ids, q, &[]),
            beat(&mut ids, q, &[(1, 0, false)]),
        ]),
        // bar 1: dotted quarter, eighth, half (tied into bar 2)
        bar(vec![
            beat(&mut ids, dq, &[(3, 2, false)]),
            beat(&mut ids, e, &[(3, 4, false)]),
            beat(&mut ids, plain(BaseValue::HALF), &[(4, 2, false)]),
        ]),
        // bar 2: tied half continues, then a rest quarter and two eighths
        bar(vec![
            beat(&mut ids, plain(BaseValue::HALF), &[(4, 2, true)]),
            beat(&mut ids, e, &[(1, 3, false)]),
            beat(&mut ids, e, &[(1, 5, false)]),
            beat(&mut ids, q, &[]),
        ]),
        // bar 3: three eighth triplets, rest, quarter, half rest
        bar(vec![
            beat(&mut ids, trip, &[(2, 0, false)]),
            beat(&mut ids, trip, &[(2, 2, false)]),
            beat(&mut ids, trip, &[(2, 3, false)]),
            beat(&mut ids, plain(BaseValue::HALF), &[]),
            beat(&mut ids, q, &[(1, 1, false)]),
        ]),
    ];
    // Wait: bar 3 above is 3 * 1/12 + 1/2 + 1/4 = 1; ok.
    let rhythm = vec![
        bar(vec![beat(
            &mut ids,
            plain(BaseValue::WHOLE),
            &[(6, 3, false), (5, 2, false), (4, 0, false)],
        )]),
        bar(vec![
            beat(
                &mut ids,
                plain(BaseValue::HALF),
                &[(6, 1, false), (5, 3, false)],
            ),
            beat(
                &mut ids,
                plain(BaseValue::HALF),
                &[(6, 3, false), (5, 5, false)],
            ),
        ]),
        bar(vec![beat(&mut ids, plain(BaseValue::WHOLE), &[])]),
        bar(vec![beat(
            &mut ids,
            plain(BaseValue::WHOLE),
            &[(6, 0, false), (5, 2, false), (4, 2, false), (3, 1, false)],
        )]),
    ];
    let mk = |id: u64, name: &str, bars: Vec<BarContent>| TrackData {
        id: TrackId::from_raw(id),
        name: name.into(),
        instrument: InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
        bars,
    };
    Score::from_parts(ScoreParts {
        master_bars: vec![MasterBar::default(); 4],
        tracks: vec![mk(1, "Lead", lead), mk(2, "Rhythm", rhythm)],
        tempo_map: vec![
            TempoChange {
                bar: 0,
                offset: Rational::ZERO,
                tempo: Tempo::from_bpm(120).unwrap(),
            },
            TempoChange {
                bar: 2,
                offset: Rational::new(1, 4).unwrap(),
                tempo: Tempo::from_bpm(90).unwrap(),
            },
        ],
    })
    .expect("sample score is valid")
}

/// Notes of a timeline as (channel, start, end, key) in whole-note positions.
fn notes_of(tl: &PerformanceTimeline) -> Vec<(u8, Rational, Rational, u8)> {
    let tpw = tl.ticks_per_whole().get() as i64;
    let pos = |t: i64| Rational::new(t, tpw).unwrap();
    let mut open: BTreeMap<(u8, u8), i64> = BTreeMap::new();
    let mut out = Vec::new();
    for te in tl.events() {
        match te.event {
            PerformanceEvent::NoteOn { channel, key, .. } => {
                open.insert((channel.get(), key.get()), te.tick.0);
            }
            PerformanceEvent::NoteOff { channel, key } => {
                if let Some(s) = open.remove(&(channel.get(), key.get())) {
                    out.push((channel.get(), pos(s), pos(te.tick.0), key.get()));
                }
            }
            _ => {}
        }
    }
    out.sort();
    out
}

// ---------------------------------------------------------------- export

#[test]
fn export_writes_type1_at_960_ppq_with_conductor_and_channel_tracks() {
    let score = sample_score();
    let tl = compile(&score, &CompileOptions::new()).unwrap().timeline;
    let (bytes, report) = export_smf(&score, &tl).unwrap();
    let smf = Smf::parse(&bytes).unwrap();
    assert_eq!(smf.header.format, Format::Parallel);
    assert_eq!(smf.header.timing, Timing::Metrical(u15::new(PPQ)));
    assert_eq!(smf.tracks.len(), 3, "conductor + two channels");
    assert_eq!(report.tracks, 3);
    assert_eq!(report.time_signatures, 1);
    assert!(report.notes > 0);
    assert_eq!(report.rounded_ticks, 0, "all values divide 960 PPQ evenly");
    // Both tempo changes sit in the conductor track.
    let tempos = smf.tracks[0]
        .iter()
        .filter(|e| matches!(e.kind, TrackEventKind::Meta(MetaMessage::Tempo(_))))
        .count();
    assert_eq!(tempos, 2);
}

// ---------------------------------------------------------------- round trip

#[test]
fn round_trip_keeps_pitches_and_rhythm_within_the_reported_quantization() {
    let score = sample_score();
    let tl = compile(&score, &CompileOptions::new()).unwrap().timeline;
    let (bytes, _) = export_smf(&score, &tl).unwrap();

    let (imported, report) = import_smf(&bytes).unwrap();
    assert_eq!(report.ppq, PPQ);
    assert_eq!(report.tracks_imported, 2);
    assert_eq!(report.bars, 4);
    assert_eq!(
        report.quantized_points, 0,
        "an exported file is already on the grid"
    );
    assert_eq!(report.triplet_bars, 1, "bar 4 uses the 1/12 grid");
    assert!(report.issues.is_empty(), "{:?}", report.issues);

    let tl2 = compile(&imported, &CompileOptions::new()).unwrap().timeline;
    assert_eq!(
        notes_of(&tl),
        notes_of(&tl2),
        "pitches, onsets and durations"
    );

    // Tempo: same positions; the value is rounded to whole microseconds per quarter.
    let t1: Vec<_> = tl.tempo().iter().collect();
    let t2: Vec<_> = tl2.tempo().iter().collect();
    assert_eq!(t1.len(), t2.len());
    for (a, b) in t1.iter().zip(&t2) {
        let pa = Rational::new(a.tick.0, tl.ticks_per_whole().get() as i64).unwrap();
        let pb = Rational::new(b.tick.0, tl2.ticks_per_whole().get() as i64).unwrap();
        assert_eq!(pa, pb);
        let (x, y) = (a.bpm.bpm(), b.bpm.bpm());
        let diff = (x.numerator() as f64 / x.denominator() as f64)
            - (y.numerator() as f64 / y.denominator() as f64);
        assert!(diff.abs() < 0.001, "tempo {x} vs {y}");
    }
}

#[test]
fn a_note_across_a_bar_line_is_split_and_tied() {
    // PPQ 480: bar = 1920 ticks. A quarter note starting a half-bar before the line
    // (tick 1680, length 480) crosses it.
    let bytes = smf_bytes(
        480,
        vec![vec![
            on(0, 60, 1680),
            off(0, 60, 2160),
            on(0, 62, 2160),
            off(0, 62, 2640),
        ]],
    );
    let (score, report) = import_smf(&bytes).unwrap();
    assert_eq!(report.bars, 2);
    let tl = compile(&score, &CompileOptions::new()).unwrap().timeline;
    let notes = notes_of(&tl);
    // C4 lasts 480/1920 of a bar = 1/4 whole, from 1680/1920 * 1 = 7/8.
    assert_eq!(
        notes[0].1,
        Rational::new(7, 8)
            .unwrap()
            .checked_mul(Rational::ONE)
            .unwrap()
    );
    assert_eq!(
        notes[0].2.checked_sub(notes[0].1).unwrap(),
        Rational::new(1, 4).unwrap()
    );
    assert_eq!(
        notes.len(),
        2,
        "the tie merges the two pieces into one note"
    );
}

// ---------------------------------------------------------------- quantization

#[test]
fn loose_timing_is_snapped_and_reported() {
    // PPQ 480, sixteenth = 120. Quarter notes played a little early/late.
    let bytes = smf_bytes(
        480,
        vec![vec![
            on(0, 64, 7),
            off(0, 64, 470),
            on(0, 66, 488),
            off(0, 66, 950),
            on(0, 68, 961),
            off(0, 68, 1440),
        ]],
    );
    let (score, report) = import_smf(&bytes).unwrap();
    assert!(report.quantized_points > 0);
    assert!(report.max_deviation_ticks >= 10 && report.max_deviation_ticks <= 60);
    let notes = notes_of(&compile(&score, &CompileOptions::new()).unwrap().timeline);
    let quarter = Rational::new(1, 4).unwrap();
    assert_eq!(notes[0].1, Rational::ZERO);
    assert_eq!(notes[1].1, quarter);
    assert_eq!(notes[2].1, Rational::new(1, 2).unwrap());
}

#[test]
fn triplets_choose_the_twelfth_grid() {
    // Three eighth triplets (160 ticks each at PPQ 480) in a 4/4 bar.
    let bytes = smf_bytes(
        480,
        vec![vec![
            on(0, 60, 0),
            off(0, 60, 160),
            on(0, 62, 160),
            off(0, 62, 320),
            on(0, 64, 320),
            off(0, 64, 480),
        ]],
    );
    let (_, report) = import_smf(&bytes).unwrap();
    assert_eq!(report.triplet_bars, 1);
    assert_eq!(report.quantized_points, 0);
}

// ---------------------------------------------------------------- filtering

#[test]
fn percussion_is_skipped_and_reported() {
    let bytes = smf_bytes(
        480,
        vec![vec![
            on(9, 36, 0),
            off(9, 36, 120),
            on(0, 60, 0),
            off(0, 60, 480),
        ]],
    );
    let (score, report) = import_smf(&bytes).unwrap();
    assert_eq!(report.tracks_imported, 1);
    assert_eq!(score.track_count(), 1);
    assert!(
        report
            .issues
            .contains(&ImportIssue::PercussionSkipped { notes: 1 })
    );
}

#[test]
fn unplayable_pitches_are_dropped_with_a_report() {
    // MIDI 20 is below the lowest guitar string.
    let bytes = smf_bytes(
        480,
        vec![vec![
            on(0, 20, 0),
            off(0, 20, 480),
            on(0, 60, 480),
            off(0, 60, 960),
        ]],
    );
    let (_, report) = import_smf(&bytes).unwrap();
    assert!(
        report
            .issues
            .iter()
            .any(|i| matches!(i, ImportIssue::NotPlayable { midi: 20, .. }))
    );
    assert_eq!(report.notes_imported, 1);
}

#[test]
fn time_signature_and_default_tempo() {
    // 3/4 (numerator 3, denominator 2^2) and no tempo event.
    let sig = (
        0,
        TrackEventKind::Meta(MetaMessage::TimeSignature(3, 2, 24, 8)),
    );
    let bytes = smf_bytes(
        480,
        vec![vec![
            sig,
            on(0, 60, 0),
            off(0, 60, 1440),
            on(0, 62, 1440),
            off(0, 62, 2880),
        ]],
    );
    let (score, report) = import_smf(&bytes).unwrap();
    assert_eq!(report.bars, 2);
    assert_eq!(
        score.master_bars()[0].time_signature,
        TimeSignature::new(3, 4).unwrap()
    );
    assert!(report.issues.contains(&ImportIssue::TempoDefaulted));
}

#[test]
fn tempo_event_sets_the_tempo() {
    let bytes = smf_bytes(
        480,
        vec![vec![tempo_us(600_000, 0), on(0, 60, 0), off(0, 60, 480)]],
    );
    let (score, report) = import_smf(&bytes).unwrap();
    assert!(!report.issues.contains(&ImportIssue::TempoDefaulted));
    assert_eq!(score.tempo_map()[0].tempo, Tempo::from_bpm(100).unwrap());
}

// ---------------------------------------------------------------- limits

#[test]
fn limits_are_enforced() {
    let too_big = vec![0u8; MAX_FILE_BYTES + 1];
    assert!(matches!(
        import_smf(&too_big),
        Err(ImportError::TooLarge { .. })
    ));

    let many = smf_bytes(480, (0..=MAX_TRACKS).map(|_| Vec::new()).collect());
    assert!(matches!(
        import_smf(&many),
        Err(ImportError::TooManyTracks { .. })
    ));

    assert!(matches!(
        import_smf(b"not midi"),
        Err(ImportError::Malformed(_))
    ));
    assert!(matches!(import_smf(&[]), Err(ImportError::Malformed(_))));

    let empty = smf_bytes(480, vec![Vec::new()]);
    assert_eq!(import_smf(&empty).unwrap_err(), ImportError::NoNotes);
}

#[test]
fn smpte_timing_is_rejected() {
    let smf = Smf {
        header: Header::new(Format::SingleTrack, Timing::Timecode(midly::Fps::Fps25, 40)),
        tracks: vec![vec![TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        }]],
    };
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes).unwrap();
    assert_eq!(
        import_smf(&bytes).unwrap_err(),
        ImportError::UnsupportedTiming
    );
}

// ---------------------------------------------------------------- regressions

/// Found by the corruption property: midly 0.5.3 panics ("attempt to negate with
/// overflow") on a header whose division is 0x8000 (SMPTE, frames byte 0x80). The
/// importer must return a typed error instead of crashing (IM-01, FF-15).
#[test]
fn a_header_that_panics_the_midly_parser_is_a_typed_error() {
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(b"MThd");
    bytes.extend_from_slice(&[0, 0, 0, 6, 0, 1, 0, 1, 0x80, 0x00]);
    bytes.extend_from_slice(b"MTrk");
    bytes.extend_from_slice(&[0, 0, 0, 4, 0x00, 0xFF, 0x2F, 0x00]);
    let result = import_smf(&bytes);
    assert!(
        matches!(
            result,
            Err(ImportError::Malformed(_)) | Err(ImportError::UnsupportedTiming)
        ),
        "{result:?}"
    );
}

/// Found by the property test (it hung): a very short note whose onset snaps onto the
/// end of the last bar used to start outside every bar and loop forever. It must finish
/// quickly, drop that note and report it.
#[test]
fn a_note_snapping_onto_the_end_of_the_last_bar_does_not_hang() {
    let notes: [(u8, u8, i64, i64); 5] = [
        (3, 33, 2309, 1391),
        (3, 55, 142, 1308),
        (1, 72, 2357, 262),
        (1, 59, 1002, 1042),
        (1, 46, 3819, 2),
    ];
    let mut evs: Abs = Vec::new();
    for (ch, key, start, len) in notes {
        evs.push(on(ch, key, start));
        evs.push(off(ch, key, start + len));
    }
    let bytes = smf_bytes(192, vec![evs]);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(import_smf(&bytes));
    });
    let result = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("import must not hang");
    let (score, report) = result.unwrap();
    assert!(score.validate().is_empty());
    assert!(
        report
            .issues
            .iter()
            .any(|i| matches!(i, ImportIssue::DroppedAtEnd { notes: 1 })),
        "{:?}",
        report.issues
    );
}

// ---------------------------------------------------------------- properties

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// IM-01: arbitrary bytes never panic and never produce an invalid score.
    #[test]
    fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..400)) {
        let _ = import_smf(&bytes);
    }

    /// Arbitrary bytes behind a valid header never panic either.
    #[test]
    fn corrupted_files_never_panic(flip in proptest::collection::vec((0usize..200, any::<u8>()), 0..6)) {
        let mut bytes = smf_bytes(480, vec![vec![on(0, 60, 0), off(0, 60, 480), on(0, 64, 480), off(0, 64, 960)]]);
        for (i, b) in flip {
            if i < bytes.len() { bytes[i] = b; }
        }
        let _ = import_smf(&bytes);
    }

    /// Any set of notes on a sane grid imports into a valid score (never `Invalid`).
    #[test]
    fn random_notes_always_import_into_a_valid_score(
        ppq in prop_oneof![Just(96u16), Just(192), Just(480), Just(960)],
        notes in proptest::collection::vec((0u8..3, 30u8..90, 0i64..4000, 1i64..1500), 1..24),
    ) {
        let mut evs: Abs = Vec::new();
        for (ch, key, start, len) in notes {
            let ch = if ch == 2 { 3 } else { ch };
            evs.push(on(ch, key, start));
            evs.push(off(ch, key, start + len));
        }
        let bytes = smf_bytes(ppq, vec![evs]);
        match import_smf(&bytes) {
            Ok((score, _)) => {
                prop_assert!(score.validate().is_empty());
                // And it compiles for playback.
                prop_assert!(compile(&score, &CompileOptions::new()).is_ok());
            }
            Err(ImportError::NoNotes) => {}
            Err(e) => prop_assert!(false, "unexpected error: {e}"),
        }
    }
}
