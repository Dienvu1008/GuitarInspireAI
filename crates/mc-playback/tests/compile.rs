//! Slice 4a tests: PB-09 determinism, timeline variants of PB-02/PB-04/PB-07, the
//! golden canonical_text, the note on/off pairing property, and D-05.

use mc_playback::{
    CompileOptions, PerformanceEvent, PerformanceTimeline, PlaybackWarning, TempoFactor, Tick,
    compile,
};
use mc_score::fixtures::{Shape, TYPICAL, reference};
use mc_score::{
    BarContent, Beat, BeatId, FrettedInstrument, InstrumentSpec, MasterBar, Note, NoteEffects,
    NoteId, NoteKind, Score, ScoreParts, TempoChange, TrackData, TrackId, Voice,
};
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning};
use mc_theory::{BaseValue, NoteValue, Rational, Tempo};
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------- builders

fn note(id: u64, string: u8, fret: u8, tie: bool, effects: NoteEffects) -> Note {
    Note {
        id: NoteId::from_raw(id),
        kind: NoteKind::Fretted(FretPosition {
            string: StringIndex::new(string).unwrap(),
            fret: Fret(fret),
        }),
        tie,
        effects,
    }
}

fn beat(id: u64, value: NoteValue, notes: Vec<Note>) -> Beat {
    Beat {
        id: BeatId::from_raw(id),
        value,
        notes,
        stroke: None,
    }
}

fn plain(base: BaseValue) -> NoteValue {
    NoteValue::plain(base)
}

/// One standard-guitar track; `bars` is the content per master bar (one voice each
/// here unless a voice vec is given). `master` lets the caller set repeats etc.
fn score(master: Vec<MasterBar>, bars: Vec<BarContent>) -> Score {
    score_tuned(master, bars, Tuning::standard_guitar(), Capo(0))
}

// Track ids share the global id space with beats and notes (H-01), so they live at
// 1000+ to stay clear of the small beat/note ids the tests use.
const TRACK: u64 = 1000;

fn score_tuned(master: Vec<MasterBar>, bars: Vec<BarContent>, tuning: Tuning, capo: Capo) -> Score {
    let parts = ScoreParts {
        master_bars: master,
        tracks: vec![TrackData {
            id: TrackId::from_raw(TRACK),
            name: "Guitar".into(),
            instrument: InstrumentSpec::Fretted(FrettedInstrument {
                tuning,
                fret_count: 24,
                capo,
            }),
            bars,
        }],
        tempo_map: vec![TempoChange {
            bar: 0,
            offset: Rational::ZERO,
            tempo: Tempo::from_bpm(120).unwrap(),
        }],
    };
    Score::from_parts(parts).expect("valid score")
}

fn bar(voices: Vec<Vec<Beat>>) -> BarContent {
    BarContent {
        voices: voices.into_iter().map(|beats| Voice { beats }).collect(),
    }
}

fn eff(let_ring: bool, dead: bool) -> NoteEffects {
    NoteEffects {
        let_ring,
        dead,
        ..NoteEffects::default()
    }
}

// ---------------------------------------------------------------- PB-09

#[test]
fn pb09_compiling_twice_is_byte_identical() {
    for shape in [
        Shape {
            tracks: 1,
            bars: 50,
            beats: 8,
            notes_per_beat: 2,
        },
        TYPICAL,
    ] {
        let s = reference(shape);
        let a = compile(&s, &CompileOptions::new()).unwrap();
        let b = compile(&s, &CompileOptions::new()).unwrap();
        assert_eq!(a.timeline, b.timeline);
        assert_eq!(
            a.timeline.canonical_text(),
            b.timeline.canonical_text(),
            "canonical text must be stable (PB-09)"
        );
    }
}

// ---------------------------------------------------------------- note pairing

/// Every note on has exactly one later-or-equal note off on the same channel and key,
/// with no overlapping pair on that (channel, key) in between.
#[test]
fn every_note_on_has_one_matching_note_off() {
    let timelines = [
        compile(&reference(TYPICAL), &CompileOptions::new())
            .unwrap()
            .timeline,
        compile(&mc_score::fixtures::feature_rich(), &CompileOptions::new())
            .unwrap()
            .timeline,
    ];
    for tl in timelines {
        assert_pairs_balance(&tl);
    }
}

fn assert_pairs_balance(tl: &PerformanceTimeline) {
    // Track open count per (channel, key); must never exceed 1 and end at 0.
    let mut open: BTreeMap<(u8, u8), i32> = BTreeMap::new();
    for te in tl.events() {
        match te.event {
            PerformanceEvent::NoteOn { channel, key, .. } => {
                let c = open.entry((channel.get(), key.get())).or_insert(0);
                *c += 1;
                assert!(
                    *c <= 1,
                    "overlapping note on for ch {} key {} at tick {}",
                    channel.get(),
                    key.get(),
                    te.tick.0
                );
            }
            PerformanceEvent::NoteOff { channel, key } => {
                let c = open.entry((channel.get(), key.get())).or_insert(0);
                *c -= 1;
                assert!(*c >= 0, "note off without a matching note on");
            }
            _ => {}
        }
    }
    for (k, v) in &open {
        assert_eq!(*v, 0, "unbalanced note for ch/key {k:?} (count {v})");
    }
}

// ---------------------------------------------------------------- PB-04 (mute)

#[test]
fn pb04_mute_omits_notes_without_changing_timing() {
    // Two tracks, one bar, one quarter note each.
    let m = vec![MasterBar::default()];
    let mk_track = |tid: u64, string: u8| TrackData {
        id: TrackId::from_raw(tid),
        name: format!("T{tid}"),
        instrument: InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
        bars: vec![bar(vec![vec![beat(
            tid + 1,
            plain(BaseValue::WHOLE),
            vec![note(tid + 2, string, 0, false, eff(false, false))],
        )]])],
    };
    let parts = ScoreParts {
        master_bars: m,
        tracks: vec![mk_track(2000, 6), mk_track(3000, 1)],
        tempo_map: vec![TempoChange {
            bar: 0,
            offset: Rational::ZERO,
            tempo: Tempo::from_bpm(120).unwrap(),
        }],
    };
    let s = Score::from_parts(parts).unwrap();

    let full = compile(&s, &CompileOptions::new()).unwrap().timeline;
    let mut muted = CompileOptions::new();
    muted.muted = BTreeSet::from([TrackId::from_raw(3000)]);
    let muted = compile(&s, &muted).unwrap().timeline;

    // Same timing (end tick and played bars identical), fewer note events.
    assert_eq!(full.end(), muted.end());
    assert_eq!(full.played_bars(), muted.played_bars());
    let count_ons = |tl: &PerformanceTimeline| {
        tl.events()
            .iter()
            .filter(|e| matches!(e.event, PerformanceEvent::NoteOn { .. }))
            .count()
    };
    assert_eq!(count_ons(&full), 2);
    assert_eq!(count_ons(&muted), 1, "muted track contributes no note on");
}

// ---------------------------------------------------------------- PB-02 (overfull)

#[test]
fn pb02_overfull_bar_extends_its_played_length() {
    // A 4/4 bar whose single voice holds 9/8 (one whole + one eighth): E(m) = 9/8.
    let m = vec![MasterBar::default()];
    let b = bar(vec![vec![
        beat(
            1,
            plain(BaseValue::WHOLE),
            vec![note(2, 6, 0, false, eff(false, false))],
        ),
        beat(
            3,
            plain(BaseValue::EIGHTH),
            vec![note(4, 6, 2, false, eff(false, false))],
        ),
    ]]);
    let s = score(m, vec![b]);
    let tl = compile(&s, &CompileOptions::new()).unwrap().timeline;
    // End tick = 9/8 of a whole note. tpw includes 8, so end = tpw * 9 / 8.
    let tpw = tl.ticks_per_whole().get() as i64;
    assert_eq!(tl.end(), Tick(tpw * 9 / 8));
    assert_eq!(tl.played_bars().len(), 1);
    assert_eq!(tl.played_bars()[0].length, Tick(tpw * 9 / 8));
}

// ---------------------------------------------------------------- PB-07 (repeat of overfull)

#[test]
fn pb07_repeated_overfull_bar_plays_twice_at_the_right_starts() {
    // |: overfull bar :| played twice, then a plain bar.
    let m0 = MasterBar {
        repeat_start: true,
        repeat_end: Some(2),
        ..MasterBar::default()
    };
    let m = vec![m0, MasterBar::default()];
    let overfull = bar(vec![vec![
        beat(
            1,
            plain(BaseValue::WHOLE),
            vec![note(2, 6, 0, false, eff(false, false))],
        ),
        beat(
            3,
            plain(BaseValue::EIGHTH),
            vec![note(4, 6, 2, false, eff(false, false))],
        ),
    ]]);
    let plain_bar = bar(vec![vec![beat(
        5,
        plain(BaseValue::WHOLE),
        vec![note(6, 1, 0, false, eff(false, false))],
    )]]);
    let s = score(m, vec![overfull, plain_bar]);
    let tl = compile(&s, &CompileOptions::new()).unwrap().timeline;
    let tpw = tl.ticks_per_whole().get() as i64;
    let bar_len = tpw * 9 / 8;
    // played bars: master 0 at 0, master 0 again at 9/8, master 1 at 18/8.
    let pbs = tl.played_bars();
    assert_eq!(pbs.len(), 3);
    assert_eq!((pbs[0].master_bar, pbs[0].start), (0, Tick(0)));
    assert_eq!((pbs[1].master_bar, pbs[1].start), (0, Tick(bar_len)));
    assert_eq!((pbs[2].master_bar, pbs[2].start), (1, Tick(2 * bar_len)));
    // first_played_start_of_master_bar returns the FIRST occurrence.
    assert_eq!(tl.first_played_start_of_master_bar(0), Some(Tick(0)));
    assert_eq!(
        tl.first_played_start_of_master_bar(1),
        Some(Tick(2 * bar_len))
    );
    assert_eq!(tl.first_played_start_of_master_bar(2), None);
}

// ---------------------------------------------------------------- D-05

#[test]
fn d05_pitch_above_midi_range_is_warned_and_omitted() {
    // A very high string 1 (E10 = MIDI 136) is above 127; the low E is in range.
    let m = vec![MasterBar::default()];
    let b = bar(vec![vec![beat(
        1,
        plain(BaseValue::WHOLE),
        vec![
            note(2, 1, 0, false, eff(false, false)), // string 1 open = E10, out of range
            note(3, 6, 0, false, eff(false, false)), // string 6 open = E2, in range
        ],
    )]]);
    let high = Tuning::new(vec![
        "E10".parse().unwrap(),
        "B3".parse().unwrap(),
        "G3".parse().unwrap(),
        "D3".parse().unwrap(),
        "A2".parse().unwrap(),
        "E2".parse().unwrap(),
    ])
    .unwrap();
    let s = score_tuned(m, vec![b], high, Capo(0));
    let out = compile(&s, &CompileOptions::new()).unwrap();
    // String 1 open is E9 (semitone 136) -> out of range, omitted and warned.
    assert!(
        out.warnings
            .iter()
            .any(|w| matches!(w, PlaybackWarning::PitchOutOfRange { .. }))
    );
    let note_ons = out
        .timeline
        .events()
        .iter()
        .filter(|e| matches!(e.event, PerformanceEvent::NoteOn { .. }))
        .count();
    assert_eq!(note_ons, 1, "only the in-range low E sounds");
}

// ---------------------------------------------------------------- tie and let ring

#[test]
fn tied_notes_become_one_sustained_note() {
    // Two quarter notes on string 6 fret 0, the second tied: one note on, one note off
    // at the end of the second beat.
    let m = vec![MasterBar::default()];
    let b = bar(vec![vec![
        beat(
            1,
            plain(BaseValue::QUARTER),
            vec![note(2, 6, 0, false, eff(false, false))],
        ),
        beat(
            3,
            plain(BaseValue::QUARTER),
            vec![note(4, 6, 0, true, eff(false, false))],
        ),
    ]]);
    let s = score(m, vec![b]);
    let tl = compile(&s, &CompileOptions::new()).unwrap().timeline;
    let ons: Vec<_> = tl
        .events()
        .iter()
        .filter(|e| matches!(e.event, PerformanceEvent::NoteOn { .. }))
        .collect();
    let offs: Vec<_> = tl
        .events()
        .iter()
        .filter(|e| matches!(e.event, PerformanceEvent::NoteOff { .. }))
        .collect();
    assert_eq!(ons.len(), 1, "a tie does not retrigger");
    assert_eq!(offs.len(), 1);
    let tpw = tl.ticks_per_whole().get() as i64;
    assert_eq!(
        offs[0].tick,
        Tick(tpw / 2),
        "off at the end of the second quarter"
    );
}

#[test]
fn dead_note_is_short_and_quiet() {
    let m = vec![MasterBar::default()];
    let b = bar(vec![vec![beat(
        1,
        plain(BaseValue::QUARTER),
        vec![note(2, 6, 0, false, eff(false, true))],
    )]]);
    let s = score(m, vec![b]);
    let tl = compile(&s, &CompileOptions::new()).unwrap().timeline;
    let tpw = tl.ticks_per_whole().get() as i64;
    let on = tl
        .events()
        .iter()
        .find_map(|e| match e.event {
            PerformanceEvent::NoteOn { velocity, .. } => Some(velocity),
            _ => None,
        })
        .unwrap();
    assert_eq!(on.get(), 40, "dead note velocity");
    let off = tl
        .events()
        .iter()
        .find(|e| matches!(e.event, PerformanceEvent::NoteOff { .. }))
        .unwrap();
    assert_eq!(
        off.tick,
        Tick(tpw / 64),
        "dead note lasts 1/64 of a whole note"
    );
}

// ---------------------------------------------------------------- golden canonical text

/// A fixed small score whose canonical timeline is committed as a golden file. It
/// exercises a repeat, two voices, a tie across beats, a dead note, a let-ring note and
/// a tempo change, so the golden locks the event format and ordering (PB-09 contract).
/// Regenerate after an intended change with:
///   cargo test -p mc-playback -- --ignored update_golden
fn golden_score() -> Score {
    // |: bar0 (two voices) :| bar1, with a tempo change mid bar1.
    let m0 = MasterBar {
        repeat_start: true,
        repeat_end: Some(2),
        ..MasterBar::default()
    };
    let m = vec![m0, MasterBar::default()];

    // Bar 0 voice 0: quarter (string 6 fret 0), quarter tied, half let-ring.
    let v0 = vec![
        beat(
            1,
            plain(BaseValue::QUARTER),
            vec![note(2, 6, 0, false, eff(false, false))],
        ),
        beat(
            3,
            plain(BaseValue::QUARTER),
            vec![note(4, 6, 0, true, eff(false, false))],
        ),
        beat(
            5,
            plain(BaseValue::HALF),
            vec![note(6, 3, 5, false, eff(true, false))],
        ),
    ];
    // Bar 0 voice 1: whole rest then nothing; one dead note on string 1.
    let v1 = vec![beat(
        7,
        plain(BaseValue::WHOLE),
        vec![note(8, 1, 2, false, eff(false, true))],
    )];
    let bar0 = bar(vec![v0, v1]);

    // Bar 1: a single whole note.
    let bar1 = bar(vec![vec![beat(
        9,
        plain(BaseValue::WHOLE),
        vec![note(10, 2, 1, false, eff(false, false))],
    )]]);

    let parts = ScoreParts {
        master_bars: m,
        tracks: vec![TrackData {
            id: TrackId::from_raw(TRACK),
            name: "Guitar".into(),
            instrument: InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
            bars: vec![bar0, bar1],
        }],
        tempo_map: vec![
            TempoChange {
                bar: 0,
                offset: Rational::ZERO,
                tempo: Tempo::from_bpm(120).unwrap(),
            },
            TempoChange {
                bar: 1,
                offset: Rational::new(1, 4).unwrap(),
                tempo: Tempo::new(Rational::new(141, 1).unwrap()).unwrap(),
            },
        ],
    };
    Score::from_parts(parts).expect("valid golden score")
}

#[test]
fn golden_canonical_text_matches() {
    let out = compile(&golden_score(), &CompileOptions::new()).unwrap();
    let got = out.timeline.canonical_text();
    // No file I/O in a domain crate's tests (FF-03): the golden is embedded. On a
    // mismatch the actual text is printed; review it and replace golden/timeline.txt
    // only if the change is intended.
    let expected = include_str!("golden/timeline.txt").replace("\r\n", "\n");
    if got != expected {
        eprintln!("--- actual canonical timeline ---\n{got}--- end ---");
        panic!("golden timeline differs; actual text printed above");
    }
}

// ---------------------------------------------------------------- tick <-> sample

#[test]
fn tick_to_sample_is_floor_and_factor_scales_inversely() {
    // One 4/4 bar at 120 BPM: a whole note lasts 2 seconds. tpw ticks = a whole note.
    let m = vec![MasterBar::default()];
    let b = bar(vec![vec![beat(
        1,
        plain(BaseValue::WHOLE),
        vec![note(2, 6, 0, false, eff(false, false))],
    )]]);
    let tl = compile(&score(m, vec![b]), &CompileOptions::new())
        .unwrap()
        .timeline;
    let whole = Tick(tl.ticks_per_whole().get() as i64);

    for (sr, expected_2s) in [(44100u32, 88200u64), (48000, 96000), (96000, 192000)] {
        // At 120 BPM a whole note is 2 s; samples = 2 * sample_rate.
        let s = tl.tick_to_sample(whole, sr, TempoFactor::NORMAL);
        assert_eq!(s, expected_2s, "whole note at {sr} Hz, 1.0x");
        // Half tempo (0.25x..=2.0x): 0.5x doubles the time. factor 50 -> 2x samples.
        let slow = tl.tick_to_sample(whole, sr, TempoFactor::new(50).unwrap());
        assert_eq!(slow, expected_2s * 2, "0.5x doubles the duration");
        // 2.0x halves it.
        let fast = tl.tick_to_sample(whole, sr, TempoFactor::new(200).unwrap());
        assert_eq!(fast, expected_2s / 2, "2.0x halves the duration");
    }

    // Floor rounding and round trip: sample_to_tick(tick_to_sample(t)) <= t.
    for &t in &[0i64, 1, 7, whole.0 / 3, whole.0, whole.0 * 2] {
        for sr in [44100u32, 48000, 96000] {
            for f in [25u16, 100, 200] {
                let factor = TempoFactor::new(f).unwrap();
                let s = tl.tick_to_sample(Tick(t), sr, factor);
                let back = tl.sample_to_tick(s, sr, factor);
                assert!(
                    back.0 <= t.min(tl.end().0),
                    "round trip t={t} sr={sr} f={f}: back {} > {t}",
                    back.0
                );
            }
        }
    }
}

/// Two 4/4 bars; the second starts at 240 BPM (twice as fast as the 120 BPM first bar).
fn two_tempo_score() -> Score {
    let bar_of = |id: u64| {
        bar(vec![vec![beat(
            id,
            plain(BaseValue::WHOLE),
            vec![note(id + 1, 6, 0, false, eff(false, false))],
        )]])
    };
    Score::from_parts(ScoreParts {
        master_bars: vec![MasterBar::default(); 2],
        tracks: vec![TrackData {
            id: TrackId::from_raw(TRACK),
            name: "Guitar".into(),
            instrument: InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
            bars: vec![bar_of(1), bar_of(3)],
        }],
        tempo_map: vec![
            TempoChange {
                bar: 0,
                offset: Rational::ZERO,
                tempo: Tempo::from_bpm(120).unwrap(),
            },
            TempoChange {
                bar: 1,
                offset: Rational::ZERO,
                tempo: Tempo::from_bpm(240).unwrap(),
            },
        ],
    })
    .unwrap()
}

#[test]
fn tempo_changes_are_integrated_piecewise() {
    let tl = compile(&two_tempo_score(), &CompileOptions::new())
        .unwrap()
        .timeline;
    let tpw = tl.ticks_per_whole().get() as i64;
    let f = TempoFactor::NORMAL;
    for sr in [44100u32, 48000, 96000] {
        // Bar 1 at 120 BPM lasts 2 s; bar 2 at 240 BPM lasts 1 s.
        assert_eq!(tl.tick_to_sample(Tick(tpw), sr, f), 2 * sr as u64);
        assert_eq!(tl.tick_to_sample(Tick(2 * tpw), sr, f), 3 * sr as u64);
        // Halfway into bar 2 is 0.5 s after the tempo change.
        assert_eq!(
            tl.tick_to_sample(Tick(tpw + tpw / 2), sr, f),
            2 * sr as u64 + sr as u64 / 2
        );
        // The inverse follows the same segments.
        assert_eq!(tl.sample_to_tick(2 * sr as u64, sr, f), Tick(tpw));
        assert_eq!(tl.sample_to_tick(3 * sr as u64, sr, f), Tick(2 * tpw));
        assert_eq!(
            tl.sample_to_tick(2 * sr as u64 + sr as u64 / 2, sr, f),
            Tick(tpw + tpw / 2)
        );
    }
    // The user factor scales every segment: 0.5x doubles the whole duration.
    let slow = TempoFactor::new(50).unwrap();
    assert_eq!(tl.tick_to_sample(Tick(2 * tpw), 48000, slow), 6 * 48000);
}

#[test]
fn piecewise_conversion_is_monotone_and_floor_consistent() {
    let tl = compile(&two_tempo_score(), &CompileOptions::new())
        .unwrap()
        .timeline;
    let end = tl.end().0;
    for sr in [44100u32, 48000] {
        for fh in [25u16, 75, 100, 200] {
            let f = TempoFactor::new(fh).unwrap();
            let mut prev = 0u64;
            for t in (0..=end).step_by(7) {
                let s = tl.tick_to_sample(Tick(t), sr, f);
                assert!(s >= prev, "monotone at tick {t}");
                prev = s;
                let back = tl.sample_to_tick(s, sr, f);
                assert!(back.0 <= t, "round trip {t} -> {s} -> {}", back.0);
            }
        }
    }
}

#[test]
fn event_index_at_boundaries() {
    let tl = compile(
        &reference(Shape {
            tracks: 1,
            bars: 2,
            beats: 8,
            notes_per_beat: 1,
        }),
        &CompileOptions::new(),
    )
    .unwrap()
    .timeline;
    assert_eq!(tl.event_index_at(Tick(0)), 0);
    // Past the end returns the event count.
    let past = Tick(tl.end().0 + 1000);
    assert_eq!(tl.event_index_at(past), tl.events().len());
    // Lower bound: index points at the first event with tick >= target.
    let idx = tl.event_index_at(Tick(1));
    assert!(tl.events()[idx].tick.0 >= 1);
    if idx > 0 {
        assert!(tl.events()[idx - 1].tick.0 < 1);
    }
}

#[test]
#[should_panic(expected = "negative tick")]
fn event_index_at_rejects_negative_ticks() {
    let tl = compile(
        &reference(Shape {
            tracks: 1,
            bars: 1,
            beats: 8,
            notes_per_beat: 1,
        }),
        &CompileOptions::new(),
    )
    .unwrap()
    .timeline;
    let _ = tl.event_index_at(Tick(-1));
}

// ---------------------------------------------------------------- program and channel

#[test]
fn program_change_on_every_channel_at_tick_zero() {
    let s = reference(Shape {
        tracks: 2,
        bars: 1,
        beats: 8,
        notes_per_beat: 1,
    });
    let tl = compile(&s, &CompileOptions::new()).unwrap().timeline;
    let programs: Vec<_> = tl
        .events()
        .iter()
        .filter(|e| matches!(e.event, PerformanceEvent::ProgramChange { .. }))
        .collect();
    assert_eq!(programs.len(), 2);
    for p in programs {
        assert_eq!(p.tick, Tick(0));
        if let PerformanceEvent::ProgramChange { program, .. } = p.event {
            assert_eq!(program.get(), 25);
        }
    }
    // Channels 0 and 1, never 9.
    let chans: BTreeSet<u8> = tl.channels().iter().map(|c| c.channel.get()).collect();
    assert_eq!(chans, BTreeSet::from([0, 1]));
}
