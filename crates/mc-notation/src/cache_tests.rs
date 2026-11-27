//! ADR-041 L9: the bar-plan cache with notation. Unit tests, because the key mutations
//! use the `#[cfg(test)]` hook in `layout::key_mutation`.

use crate::layout::key_mutation::SKIP;
use crate::{
    LayoutCache, LayoutMetrics, ShownTrack, StaffConfiguration, outline_score_cached_with,
    outline_score_with,
};
use mc_score::{BeatRef, FrettedInstrument, InstrumentSpec, MasterBar, Score, TrackId};
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning};
use mc_theory::{BaseValue, KeySignature, Mode, NoteValue, Tempo, TimeSignature};

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) % n
    }
}

fn pos(string: u8, fret: u8) -> FretPosition {
    FretPosition {
        string: StringIndex::new(string).unwrap(),
        fret: Fret(fret),
    }
}

fn key(f: i8) -> MasterBar {
    MasterBar::new(
        TimeSignature::COMMON,
        KeySignature::new(f, Mode::Major).unwrap(),
    )
}

fn score(bars: usize) -> Score {
    let mut s = Score::new(
        "Cache",
        InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
        MasterBar::default(),
        Tempo::from_bpm(100).unwrap(),
    )
    .unwrap();
    s.insert_master_bars(1, bars - 1, MasterBar::default())
        .unwrap();
    s
}

fn put(s: &mut Score, t: TrackId, bar: usize, value: NoteValue, notes: &[(u8, u8)], tie: bool) {
    let index = s.track(t).unwrap().bar(bar).unwrap().voices[0].beats.len();
    let (beat, _) = s.insert_beat(t, bar, 0, index, value).unwrap();
    let r = BeatRef {
        track: t,
        bar,
        voice: 0,
        beat,
    };
    for &(st, fr) in notes {
        let (id, _) = s.set_note(r, pos(st, fr)).unwrap();
        if tie {
            s.set_tie(r, id, true).unwrap();
        }
    }
}

fn all(s: &Score, staves: StaffConfiguration) -> Vec<ShownTrack> {
    s.tracks()
        .map(|t| ShownTrack {
            track: t.id(),
            staves,
        })
        .collect()
}

const BOTH: StaffConfiguration = StaffConfiguration::NOTATION_AND_TAB;

/// One random edit of a kind the cache key must notice; returns its name.
fn edit(
    s: &mut Score,
    shown: &mut Vec<ShownTrack>,
    m: &mut LayoutMetrics,
    rng: &mut Rng,
) -> &'static str {
    let bars = s.master_bar_count();
    let tracks: Vec<TrackId> = s.tracks().map(|t| t.id()).collect();
    let t = tracks[rng.below(tracks.len() as u64) as usize];
    let bar = rng.below(bars as u64) as usize;
    let q = NoteValue::plain(BaseValue::QUARTER);
    match rng.below(11) {
        0 => {
            let st = 1 + rng.below(6) as u8;
            let fr = rng.below(12) as u8;
            let _ = s.track(t).unwrap().bar(bar).unwrap().voices[0].beats.len();
            put(s, t, bar, q, &[(st, fr)], false);
            "note"
        }
        1 => {
            let beats = &s.track(t).unwrap().bar(bar).unwrap().voices[0].beats;
            if let Some(b) = beats.first() {
                let r = BeatRef {
                    track: t,
                    bar,
                    voice: 0,
                    beat: b.id,
                };
                let _ = s.set_beat_value(r, NoteValue::plain(BaseValue::EIGHTH));
            }
            "duration"
        }
        2 => {
            // A tie into the first beat of the bar from the previous bar.
            if bar > 0 {
                let prev = s.track(t).unwrap().bar(bar - 1).unwrap().voices[0]
                    .beats
                    .last()
                    .cloned();
                if let Some(p) = prev.and_then(|p| p.notes.first().map(|n| n.position())) {
                    let _ = s.track(t).unwrap().bar(bar).unwrap().voices[0]
                        .beats
                        .is_empty()
                        .then(|| put(s, t, bar, q, &[(p.string.get(), p.fret.0)], true));
                }
            }
            "tie"
        }
        3 => {
            let _ = s.set_master_bar(bar, key(rng.below(15) as i8 - 7));
            "key signature"
        }
        4 => {
            let mut mb = s.master_bars()[bar].clone();
            mb.time_signature = [
                TimeSignature::COMMON,
                TimeSignature::new(3, 4).unwrap(),
                TimeSignature::new(6, 8).unwrap(),
            ][rng.below(3) as usize];
            let _ = s.set_master_bar(bar, mb);
            "time signature"
        }
        5 => {
            // An edit that changes the sounding pitch, so the accidentals change.
            let beats = s.track(t).unwrap().bar(bar).unwrap().voices[0]
                .beats
                .clone();
            if let Some(b) = beats.first()
                && let Some(n) = b.notes.first()
            {
                let r = BeatRef {
                    track: t,
                    bar,
                    voice: 0,
                    beat: b.id,
                };
                let p = n.position();
                let _ = s.remove_note(r, n.id);
                let _ = s.set_note(r, pos(p.string.get(), (p.fret.0 + 1) % 12));
            }
            "accidental"
        }
        6 => {
            let mut i = s.track(t).unwrap().fretted().clone();
            i.capo = Capo(rng.below(4) as u8);
            let _ = s.set_instrument(t, InstrumentSpec::Fretted(i));
            "capo"
        }
        7 => {
            let mut i = s.track(t).unwrap().fretted().clone();
            i.tuning = if rng.below(2) == 0 {
                Tuning::standard_guitar()
            } else {
                Tuning::new(
                    ["E4", "B3", "G3", "D3", "A2", "D2"]
                        .map(|p| p.parse().unwrap())
                        .to_vec(),
                )
                .unwrap()
            };
            let _ = s.set_instrument(t, InstrumentSpec::Fretted(i));
            "tuning"
        }
        8 => {
            if tracks.len() < 3 && rng.below(2) == 0 {
                let (id, _) = s
                    .add_track(
                        "Added",
                        InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
                    )
                    .unwrap();
                shown.push(ShownTrack {
                    track: id,
                    staves: BOTH,
                });
                "track added"
            } else {
                if s.remove_track(t).is_ok() {
                    shown.retain(|x| x.track != t);
                }
                "track removed"
            }
        }
        9 => {
            if let Some(x) = shown.iter_mut().find(|x| x.track == t) {
                x.staves = if x.staves.notation {
                    StaffConfiguration::TAB
                } else {
                    BOTH
                };
            }
            "staff configuration"
        }
        _ => {
            m.width = [700.0, 1000.0, 1300.0][rng.below(3) as usize];
            "metrics"
        }
    }
}

#[test]
fn l9_cached_equals_fresh_after_every_change() {
    let mut kinds = std::collections::BTreeSet::new();
    for seed in 0..60u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
        let mut s = score(4);
        let t = s.track_at(0).unwrap().id();
        for bar in 0..4 {
            for _ in 0..rng.below(4) {
                put(
                    &mut s,
                    t,
                    bar,
                    NoteValue::plain(BaseValue::QUARTER),
                    &[(1 + rng.below(6) as u8, rng.below(12) as u8)],
                    false,
                );
            }
        }
        let mut shown = all(&s, BOTH);
        let mut m = LayoutMetrics::default();
        let mut cache = LayoutCache::new();
        for step in 0..25 {
            let kind = edit(&mut s, &mut shown, &mut m, &mut rng);
            kinds.insert(kind);
            if shown.is_empty() {
                shown = all(&s, BOTH);
            }
            let cached = outline_score_cached_with(&s, &shown, &m, &mut cache).unwrap();
            let fresh = outline_score_with(&s, &shown, &m).unwrap();
            assert_eq!(cached, fresh, "seed {seed} step {step} after {kind}");
        }
    }
    assert_eq!(kinds.len(), 12, "every change kind ran: {kinds:?}");
}

/// Bar 0 F#, tied into bar 1 and on into bar 2; bar 3 separate. All high on the staff,
/// so a spelling change moves the reach.
fn chain() -> (Score, TrackId) {
    let mut s = score(4);
    let t = s.track_at(0).unwrap().id();
    let w = NoteValue::plain(BaseValue::WHOLE);
    // First string fret 14: E4 + 14 = F#5 sounding (written F#6, ledger lines).
    put(&mut s, t, 0, w, &[(1, 14)], false);
    put(&mut s, t, 1, w, &[(1, 14)], true);
    put(&mut s, t, 2, w, &[(1, 14)], true);
    put(&mut s, t, 3, w, &[(1, 3)], false);
    (s, t)
}

#[test]
fn l9_a_tie_chain_edited_at_its_start_replans_exactly_the_chain() {
    let (mut s, _) = chain();
    let m = LayoutMetrics::default();
    let shown = all(&s, BOTH);
    let mut cache = LayoutCache::new();
    outline_score_cached_with(&s, &shown, &m, &mut cache).unwrap();
    assert_eq!(cache.last_measured(), 4);
    // F major spells the start Gb: the whole chain follows.
    s.set_master_bar(0, key(-1)).unwrap();
    let cached = outline_score_cached_with(&s, &shown, &m, &mut cache).unwrap();
    assert_eq!(
        cache.last_measured(),
        3,
        "bars 0, 1 and 2 re-planned, bar 3 kept"
    );
    assert_eq!(cached, outline_score_with(&s, &shown, &m).unwrap());
}

#[test]
fn l9_a_note_edit_without_ties_replans_at_most_two_bars() {
    let (mut s, t) = chain();
    let m = LayoutMetrics::default();
    let shown = all(&s, BOTH);
    let mut cache = LayoutCache::new();
    outline_score_cached_with(&s, &shown, &m, &mut cache).unwrap();
    put(
        &mut s,
        t,
        3,
        NoteValue::plain(BaseValue::QUARTER),
        &[(2, 1)],
        false,
    );
    outline_score_cached_with(&s, &shown, &m, &mut cache).unwrap();
    assert!(cache.last_measured() <= 2, "{}", cache.last_measured());
    assert!(cache.last_measured() >= 1);
}

/// With one key component left out, a change only it detects gives a stale outline.
/// Narrow systems (one bar each), so every bar sets its own box height.
fn caught(part: &'static str, change: impl Fn(&mut Score, &mut Vec<ShownTrack>)) {
    // 200 units: since EP1 (narrower notation columns) 260 fit two whole notes.
    let m = LayoutMetrics {
        width: 200.0,
        ..LayoutMetrics::default()
    };
    caught_with(part, m, true, change);
}

/// [`caught`] at metrics `m`; `one_bar` asserts the one-bar-per-system precondition.
fn caught_with(
    part: &'static str,
    m: LayoutMetrics,
    one_bar: bool,
    change: impl Fn(&mut Score, &mut Vec<ShownTrack>),
) {
    let (mut s, _) = chain();
    let mut shown = all(&s, BOTH);
    let mut cache = LayoutCache::new();
    let before = outline_score_cached_with(&s, &shown, &m, &mut cache).unwrap();
    assert!(
        !one_bar || before.systems.iter().all(|x| x.end_bar - x.first_bar == 1),
        "the precondition: one bar per system"
    );
    change(&mut s, &mut shown);
    SKIP.with(|k| k.set(Some(part)));
    let cached = outline_score_cached_with(&s, &shown, &m, &mut cache).unwrap();
    SKIP.with(|k| k.set(None));
    let fresh = outline_score_with(&s, &shown, &m).unwrap();
    assert_ne!(cached, fresh, "leaving out `{part}` is not caught");
}

#[test]
fn l9_every_key_component_is_needed() {
    // master: a key change that adds a key signature mid-system.
    caught("master", |s, _| {
        s.set_master_bar(3, key(-6)).unwrap();
    });
    // previous: bar 1's time signature changes, so bar 2 must show one. (Bar 2, not
    // bar 3: one bar per system leaves the last system at natural width, where the
    // missing time signature would not change the outline.)
    caught("previous", |s, _| {
        let mut mb = s.master_bars()[1].clone();
        mb.time_signature = TimeSignature::new(3, 4).unwrap();
        s.set_master_bar(1, mb).unwrap();
    });
    // contents: a note edit.
    caught("contents", |s, _| {
        let t = s.track_at(0).unwrap().id();
        put(
            s,
            t,
            3,
            NoteValue::plain(BaseValue::EIGHTH),
            &[(1, 20)],
            false,
        );
    });
    // instruments: a capo raises the high notes.
    caught("instruments", |s, _| {
        let t = s.track_at(0).unwrap().id();
        let mut i = s.track(t).unwrap().fretted().clone();
        i.capo = Capo(3);
        s.set_instrument(t, InstrumentSpec::Fretted(i)).unwrap();
    });
    // staves: notation switched off. With short columns the notation reserve (the
    // sharp and the ledger lines of F#6) sets the widths, so they change; at the
    // default column width a whole note's duration width hides it (EP1).
    let short_columns = LayoutMetrics {
        width: 260.0,
        min_column_width: 4.0,
        ..LayoutMetrics::default()
    };
    caught_with("staves", short_columns, false, |_, shown| {
        shown[0].staves = StaffConfiguration::TAB;
    });
    // entering: the chain's spelling changes (F#6 to Gb6 written, one position up),
    // which only the tie into bar 2 carries: bar 2's content, master bar and previous
    // master bar (bar 1, C major) are unchanged.
    caught("entering", |s, _| {
        s.set_master_bar(0, key(-1)).unwrap();
    });
    // leaving (EP5): bar 1's note is no longer tied, so no tie leaves bar 0 and bar 0's
    // high note loses the tie's reach. Bar 0's content, master bars and entering ties
    // are unchanged.
    caught("leaving", |s, _| {
        let t = s.track_at(0).unwrap().id();
        let beat = &s.track(t).unwrap().bar(1).unwrap().voices[0].beats[0];
        let (beat_id, note) = (beat.id, beat.notes[0].id);
        let r = mc_score::BeatRef {
            track: t,
            bar: 1,
            voice: 0,
            beat: beat_id,
        };
        s.set_tie(r, note, false).unwrap();
    });
}
