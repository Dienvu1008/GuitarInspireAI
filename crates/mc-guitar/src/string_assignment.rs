//! String assignment for monophonic import (ADR-023): turn a sequence of sounding
//! MIDI pitches into `(string, fret)` positions on a fretted instrument. A pure Guitar
//! service (GR-05); the MIDI adapter `mc-midi` calls it and owns no music rule itself.
//!
//! The heuristic is greedy and deterministic: for each note, consider every string that
//! can produce the exact sounding pitch within the fret range, and pick the one whose
//! fret is closest to the previous note's fret (so a melody stays in one hand position),
//! breaking ties toward the lower fret and then the higher string number. A note that no
//! string can produce (outside the instrument's range in every position) is reported and
//! left unassigned; the caller decides what to do (skip it, in mc-midi).

use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning, pitch_at};

/// A candidate position for one pitch, with its string and fret.
fn positions_for_pitch(
    tuning: &Tuning,
    capo: Capo,
    fret_count: u8,
    midi: i32,
) -> impl Iterator<Item = FretPosition> + '_ {
    (1..=tuning.string_count()).flat_map(move |s| {
        let string = StringIndex::new(s).expect("1-based");
        (0..=fret_count).filter_map(move |f| {
            let pos = FretPosition {
                string,
                fret: Fret(f),
            };
            (pitch_at(tuning, capo, pos) == Some(midi)).then_some(pos)
        })
    })
}

/// The result of assigning one pitch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assigned {
    /// The chosen position.
    At(FretPosition),
    /// No string of this instrument can sound the pitch within the fret range.
    OutOfRange { midi: i32 },
    /// The pitch is playable, but every string that can sound it is already used by
    /// another note of the same chord.
    NoFreeString { midi: i32 },
}

/// Assign the pitches of one chord (sounding together) to distinct strings. `anchor` is
/// the fret of the previous chord, if any, so the hand stays in position. Pitches are
/// handled from highest to lowest (ties by input order); the result is in input order.
/// Equal pitches must be removed by the caller (one string cannot sound a unison twice).
pub fn assign_group(
    tuning: &Tuning,
    capo: Capo,
    fret_count: u8,
    anchor: Option<u8>,
    pitches: &[i32],
) -> Vec<Assigned> {
    let mut order: Vec<usize> = (0..pitches.len()).collect();
    order.sort_by(|&a, &b| pitches[b].cmp(&pitches[a]).then(a.cmp(&b)));
    let mut used: Vec<u8> = Vec::new();
    let mut out = vec![Assigned::OutOfRange { midi: 0 }; pitches.len()];
    for i in order {
        let midi = pitches[i];
        let all: Vec<FretPosition> = positions_for_pitch(tuning, capo, fret_count, midi).collect();
        let best = all
            .iter()
            .filter(|p| !used.contains(&p.string.get()))
            .min_by_key(|p| {
                let dist = anchor.map_or(0, |a| (p.fret.0 as i32 - a as i32).unsigned_abs());
                (dist, p.fret.0, u8::MAX - p.string.get())
            })
            .copied();
        out[i] = match best {
            Some(pos) => {
                used.push(pos.string.get());
                Assigned::At(pos)
            }
            None if all.is_empty() => Assigned::OutOfRange { midi },
            None => Assigned::NoFreeString { midi },
        };
    }
    out
}

/// Assign a monophonic sequence of sounding MIDI pitches to positions on `tuning` with
/// `capo` and `fret_count` frets. Deterministic; see the module docs for the rule.
pub fn assign_sequence(
    tuning: &Tuning,
    capo: Capo,
    fret_count: u8,
    pitches: impl IntoIterator<Item = i32>,
) -> Vec<Assigned> {
    let mut out = Vec::new();
    let mut prev_fret: Option<u8> = None;
    for midi in pitches {
        let best = positions_for_pitch(tuning, capo, fret_count, midi).min_by(|a, b| {
            let key = |p: &FretPosition| {
                let dist = prev_fret.map_or(0, |pf| (p.fret.0 as i32 - pf as i32).unsigned_abs());
                // Closest to the previous fret, then the lower fret, then the higher
                // string number (toward thinner strings). All deterministic.
                (dist, p.fret.0, u8::MAX - p.string.get())
            };
            key(a).cmp(&key(b))
        });
        match best {
            Some(pos) => {
                prev_fret = Some(pos.fret.0);
                out.push(Assigned::At(pos));
            }
            None => out.push(Assigned::OutOfRange { midi }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn std_tuning() -> Tuning {
        Tuning::standard_guitar()
    }

    #[test]
    fn open_low_e_goes_to_string_six_fret_zero() {
        // E2 = MIDI 40.
        let a = assign_sequence(&std_tuning(), Capo(0), 24, [40]);
        assert_eq!(
            a,
            [Assigned::At(FretPosition {
                string: StringIndex::new(6).unwrap(),
                fret: Fret(0),
            })]
        );
    }

    #[test]
    fn a_melody_stays_near_the_previous_position() {
        // A2 (45) then B2 (47): the first note has no predecessor so it takes the lowest
        // fret (string 5 fret 0); the second then stays in that hand position at string 5
        // fret 2, rather than jumping to string 6 fret 7.
        let a = assign_sequence(&std_tuning(), Capo(0), 24, [45, 47]);
        assert_eq!(
            a,
            [
                Assigned::At(FretPosition {
                    string: StringIndex::new(5).unwrap(),
                    fret: Fret(0),
                }),
                Assigned::At(FretPosition {
                    string: StringIndex::new(5).unwrap(),
                    fret: Fret(2),
                }),
            ]
        );
    }

    #[test]
    fn a_high_run_stays_on_upper_strings_after_a_high_start() {
        // Start high so there is a previous fret to stay near: E4 (64) is string 1 open;
        // F4 (65) then stays at string 1 fret 1, not string 2 fret 6.
        let a = assign_sequence(&std_tuning(), Capo(0), 24, [69, 70]);
        // A4 (69) alone prefers the lowest fret: string 1 fret 5 vs string 2 fret 10 ->
        // string 1 fret 5. A#4 (70) then stays at string 1 fret 6.
        assert_eq!(
            a,
            [
                Assigned::At(FretPosition {
                    string: StringIndex::new(1).unwrap(),
                    fret: Fret(5),
                }),
                Assigned::At(FretPosition {
                    string: StringIndex::new(1).unwrap(),
                    fret: Fret(6),
                }),
            ]
        );
    }

    #[test]
    fn the_first_note_prefers_the_lowest_fret() {
        // A2 (45) alone: candidates are string 6 fret 5 and string 5 fret 0. With no
        // previous note, the lower fret wins: string 5 fret 0.
        let a = assign_sequence(&std_tuning(), Capo(0), 24, [45]);
        assert_eq!(
            a,
            [Assigned::At(FretPosition {
                string: StringIndex::new(5).unwrap(),
                fret: Fret(0),
            })]
        );
    }

    #[test]
    fn out_of_range_pitch_is_reported() {
        // MIDI 20 is far below the lowest string; MIDI 120 far above fret 24 of string 1.
        let a = assign_sequence(&std_tuning(), Capo(0), 24, [20, 120]);
        assert_eq!(
            a,
            [
                Assigned::OutOfRange { midi: 20 },
                Assigned::OutOfRange { midi: 120 },
            ]
        );
    }

    #[test]
    fn a_chord_uses_distinct_strings() {
        // C major open: E2 G2? use C3 E3 G3 C4 E4 = 48 52 55 60 64.
        let a = assign_group(&std_tuning(), Capo(0), 24, None, &[48, 52, 55, 60, 64]);
        let strings: Vec<u8> = a
            .iter()
            .map(|x| match x {
                Assigned::At(p) => p.string.get(),
                other => panic!("unassigned: {other:?}"),
            })
            .collect();
        let mut sorted = strings.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 5, "no string used twice: {strings:?}");
    }

    #[test]
    fn too_many_notes_for_the_strings_are_reported() {
        // Seven different pitches cannot fit on six strings.
        let a = assign_group(
            &std_tuning(),
            Capo(0),
            24,
            None,
            &[40, 45, 50, 55, 59, 64, 67],
        );
        assert!(a.iter().any(|x| matches!(x, Assigned::NoFreeString { .. })));
    }

    #[test]
    fn capo_shifts_the_positions() {
        // With capo 2, E2 (40) is no longer reachable as an open string (lowest sounding
        // is F#2 = 42), so it is out of range; F#2 is string 6 fret 0.
        let a = assign_sequence(&std_tuning(), Capo(2), 24, [40, 42]);
        assert_eq!(a[0], Assigned::OutOfRange { midi: 40 });
        assert_eq!(
            a[1],
            Assigned::At(FretPosition {
                string: StringIndex::new(6).unwrap(),
                fret: Fret(0),
            })
        );
    }
}
