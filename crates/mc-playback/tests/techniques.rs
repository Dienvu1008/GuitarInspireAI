//! ADR-035 TQ-4: the render table of playing techniques. Every combination of the five
//! note flags (32) is compiled at three written values and checked against the table:
//! velocity of the winning technique (string-percussive > dead > palm mute > let ring >
//! normal), the accent boost with its cap, and the note-off as the earlier of the
//! technique limit and the beat end (let ring: the end of the played bar).

use mc_playback::{CompileOptions, PerformanceEvent, Velocity, compile};
use mc_score::fixtures::techniques;
use mc_score::{
    BarContent, Beat, BeatId, FrettedInstrument, InstrumentSpec, MasterBar, Note, NoteEffects,
    NoteId, NoteKind, Score, ScoreParts, TempoChange, TrackData, TrackId, Voice,
};
use mc_theory::fretted::{Fret, FretPosition, StringIndex};
use mc_theory::{BaseValue, NoteValue, Rational, Tempo};

/// One 4/4 bar holding one beat of `value` with one note on string 1, fret 5.
fn one_note(value: NoteValue, effects: NoteEffects) -> Score {
    let beat = Beat {
        id: BeatId::from_raw(1),
        value,
        notes: vec![Note {
            id: NoteId::from_raw(2),
            kind: NoteKind::Fretted(FretPosition {
                string: StringIndex::new(1).unwrap(),
                fret: Fret(5),
            }),
            tie: false,
            effects,
        }],
        stroke: None,
    };
    Score::from_parts(ScoreParts {
        master_bars: vec![MasterBar::default()],
        tracks: vec![TrackData {
            id: TrackId::from_raw(1000),
            name: "T".into(),
            instrument: InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
            bars: vec![BarContent {
                voices: vec![Voice { beats: vec![beat] }],
            }],
        }],
        tempo_map: vec![TempoChange {
            bar: 0,
            offset: Rational::ZERO,
            tempo: Tempo::from_bpm(120).unwrap(),
        }],
    })
    .expect("valid")
}

/// (on tick, off tick, velocity) of every note of the compiled score, in order.
fn notes(score: &Score) -> (Vec<(i64, i64, u8)>, i64) {
    let tl = compile(score, &CompileOptions::new()).unwrap().timeline;
    let tpw = tl.ticks_per_whole().get() as i64;
    let mut open: Vec<(u8, i64, u8)> = Vec::new();
    let mut out = Vec::new();
    for e in tl.events() {
        match e.event {
            PerformanceEvent::NoteOn { key, velocity, .. } => {
                open.push((key.get(), e.tick.0, velocity.get()))
            }
            PerformanceEvent::NoteOff { key, .. } => {
                let i = open.iter().position(|o| o.0 == key.get()).expect("paired");
                let (_, on, v) = open.remove(i);
                out.push((on, e.tick.0, v));
            }
            _ => {}
        }
    }
    out.sort();
    (out, tpw)
}

fn effects(bits: u8) -> NoteEffects {
    NoteEffects {
        let_ring: bits & 1 != 0,
        dead: bits & 2 != 0,
        palm_mute: bits & 4 != 0,
        string_percussive: bits & 8 != 0,
        accent: bits & 16 != 0,
    }
}

/// The ADR-035 table, written out independently of the compiler.
fn expected(e: NoteEffects, beat_len: i64, bar_len: i64, tpw: i64) -> (i64, u8) {
    // (velocity, note-off) of the winning technique, in precedence order.
    let (base, off): (u8, i64) = if e.string_percussive {
        (70, (tpw / 64).min(beat_len))
    } else if e.dead {
        (40, (tpw / 64).min(beat_len))
    } else if e.palm_mute {
        (80, (tpw / 16).min(beat_len))
    } else if e.let_ring {
        (96, bar_len)
    } else {
        (96, beat_len)
    };
    let v = if e.accent { (base + 24).min(127) } else { base };
    (off, v)
}

#[test]
fn tq4_every_flag_combination_follows_the_render_table() {
    let values = [
        NoteValue::plain(BaseValue::WHOLE),
        NoteValue::plain(BaseValue::QUARTER),
        // Shorter than the palm-mute limit (1/16): the beat end wins.
        NoteValue::plain(BaseValue::from_exponent(5).unwrap()),
    ];
    let mut checked = 0;
    for value in values {
        for bits in 0..32u8 {
            let e = effects(bits);
            let (got, tpw) = notes(&one_note(value, e));
            assert_eq!(got.len(), 1, "one note for {e:?}");
            let len = value.length();
            let beat_len = tpw * len.numerator() / len.denominator();
            let (off, v) = expected(e, beat_len, tpw, tpw);
            assert_eq!(got[0], (0, off, v), "{e:?} at {value:?}");
            checked += 1;
        }
    }
    assert_eq!(checked, 96);
}

#[test]
fn tq4_accent_is_capped_and_applies_to_the_winner() {
    assert_eq!(Velocity::NORMAL.accented().get(), 120);
    assert_eq!(Velocity::PALM_MUTE.accented().get(), 104);
    assert_eq!(Velocity::DEAD.accented().get(), 64);
    assert_eq!(Velocity::STRING_PERCUSSIVE.accented().get(), 94);
    // The cap: anything that would pass 127 stops there.
    assert_eq!(Velocity::try_new(120).unwrap().accented().get(), 127);
}

#[test]
fn tq4_the_compiler_keeps_every_stored_flag() {
    // Compiling never changes the score: all flags stay stored when another wins.
    let s = techniques();
    let before = s.clone();
    let _ = compile(&s, &CompileOptions::new()).unwrap();
    assert_eq!(s, before);
    let combos = s
        .tracks()
        .flat_map(|t| t.bars().flat_map(|b| b.voices[0].beats.clone()))
        .flat_map(|b| b.notes)
        .filter(|n| {
            let e = n.effects;
            (e.dead && e.let_ring) || (e.palm_mute && e.let_ring) || (e.string_percussive && e.dead)
        })
        .count();
    assert_eq!(
        combos, 3,
        "the fixture holds the three precedence combinations"
    );
}

#[test]
fn tq4_the_techniques_fixture_renders_as_the_table_says() {
    let (got, tpw) = notes(&techniques());
    let q = tpw / 4;
    let e = tpw / 8;
    // Bar 0: accented chord (5 notes, 120, a quarter); palm mute eighths (80 and 104,
    // each 1/16); accented string-percussive chord (94, 1/64); dead + let ring (40,
    // 1/64). Bar 1 starts at tpw.
    let at = |on: i64| -> Vec<(i64, u8)> {
        got.iter()
            .filter(|n| n.0 == on)
            .map(|n| (n.1 - n.0, n.2))
            .collect()
    };
    assert_eq!(at(0), vec![(q, 120); 5]);
    assert_eq!(at(q), vec![(tpw / 16, 80)]);
    assert_eq!(at(q + e), vec![(tpw / 16, 104)]);
    assert_eq!(at(2 * q), vec![(tpw / 64, 94); 5]);
    assert_eq!(at(3 * q), vec![(tpw / 64, 40)]);
    // Bar 1: palm mute + let ring (80, 1/16); rest with a stroke (no note); partly
    // muted chord (one 80 at 1/16, one 96 an eighth); string-percussive + dead (70,
    // 1/64); let ring to the end of the bar (96).
    let bar1 = tpw;
    assert_eq!(at(bar1), vec![(tpw / 16, 80)]);
    assert!(at(bar1 + q).is_empty(), "a stroke on a rest plays nothing");
    let mut partly = at(bar1 + q + e);
    partly.sort();
    assert_eq!(partly, vec![(tpw / 16, 80), (e, 96)]);
    assert_eq!(at(bar1 + 2 * q), vec![(tpw / 64, 70)]);
    assert_eq!(
        at(bar1 + 3 * q),
        vec![(q, 96)],
        "let ring rings to the bar end"
    );
}

#[test]
fn tq4_a_stroke_changes_no_timing() {
    // The same score with and without strokes compiles to the same timeline (no strum
    // spread in 6a, ADR-035 decision 8.3).
    let with = techniques();
    let parts_without = {
        let mut s = with.clone();
        let t = s.track_at(0).unwrap().id();
        for bar in 0..s.master_bar_count() {
            let ids: Vec<BeatId> = s.track(t).unwrap().bar(bar).unwrap().voices[0]
                .beats
                .iter()
                .map(|b| b.id)
                .collect();
            for id in ids {
                s.set_beat_stroke(
                    mc_score::BeatRef {
                        track: t,
                        bar,
                        voice: 0,
                        beat: id,
                    },
                    None,
                )
                .unwrap();
            }
        }
        s
    };
    let a = compile(&with, &CompileOptions::new()).unwrap().timeline;
    let b = compile(&parts_without, &CompileOptions::new())
        .unwrap()
        .timeline;
    assert_eq!(a.canonical_text(), b.canonical_text());
}
