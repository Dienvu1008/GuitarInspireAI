//! ADR-040 section 6, T5: on every fixture, the written semitone of a note is its
//! sounding semitone (`pitch_at`, capo included once) plus the instrument's octave, so
//! a doubled capo or octave fails. Runs with `--features fixtures` (CI uses
//! `--all-features`).
#![cfg(feature = "fixtures")]

use mc_score::fixtures::{SMALL, TYPICAL, feature_rich, reference, techniques};
use mc_score::{InstrumentSpec, Score};
use mc_theory::fretted::{Capo, pitch_at};
use mc_theory::notation::Transposition;

/// (notes checked, notes on a track with a capo).
fn check(score: &Score) -> (usize, usize) {
    let (mut notes, mut with_capo) = (0, 0);
    for (b, master) in score.master_bars().iter().enumerate() {
        for track in score.tracks() {
            let spec = track.instrument();
            let InstrumentSpec::Fretted(f) = spec;
            assert_eq!(spec.written_transposition(), Transposition::OCTAVE_UP);
            for voice in &track.bar(b).unwrap().voices {
                for beat in &voice.beats {
                    for n in &beat.notes {
                        let sounding = pitch_at(&f.tuning, f.capo, n.position()).unwrap();
                        // The capo is in the sounding pitch exactly once.
                        let open = pitch_at(&f.tuning, Capo(0), n.position()).unwrap();
                        assert_eq!(sounding, open + f.capo.0 as i32);
                        let written = spec
                            .written_transposition()
                            .written(master.key.spell(sounding))
                            .unwrap();
                        assert_eq!(written.semitone(), sounding + 12, "bar {b}");
                        notes += 1;
                        if f.capo.0 > 0 {
                            with_capo += 1;
                        }
                    }
                }
            }
        }
    }
    (notes, with_capo)
}

#[test]
fn t5_written_is_sounding_plus_one_octave_on_every_fixture() {
    let mut total = 0;
    for s in [
        reference(SMALL),
        reference(TYPICAL),
        feature_rich(),
        techniques(),
    ] {
        total += check(&s).0;
    }
    assert!(total > 10_000, "fixtures cover enough notes: {total}");
    // feature_rich has a drop-D track with capo 2 and notes on it.
    let (_, with_capo) = check(&feature_rich());
    assert!(with_capo > 0);
}

#[test]
fn t5_capo_example_of_adr_039() {
    // Standard tuning, first string open: E4 sounding, E5 written; with capo 2, F#4
    // sounding, F#5 written. TAB stays 0 in both.
    use mc_theory::KeySignature;
    use mc_theory::fretted::{Fret, FretPosition, StringIndex, Tuning};
    let t = Tuning::standard_guitar();
    let open_e = FretPosition {
        string: StringIndex::new(1).unwrap(),
        fret: Fret(0),
    };
    let key = KeySignature::C_MAJOR;
    for (capo, sounding, written) in [(0, "E4", "E5"), (2, "F#4", "F#5")] {
        let s = pitch_at(&t, Capo(capo), open_e).unwrap();
        assert_eq!(key.spell(s).to_string(), sounding);
        let w = Transposition::OCTAVE_UP.written(key.spell(s)).unwrap();
        assert_eq!(w.to_string(), written);
    }
}
