//! Property tests for the Guitar context (G-01, G-02, G-03).

use mc_guitar::{
    BassRule, IdentifyOptions, IdentifyStatus, MIN_DISTINCT_PITCH_CLASSES, SearchOptions,
    TuningPreset, Voicing, VoicingConstraints, find_voicings, identify,
};
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning};
use mc_theory::{Chord, ChordQuality, NoteName, PitchClass, SpelledPitch};
use proptest::prelude::*;

fn chord() -> impl Strategy<Value = Chord> {
    (0..12i32, 0..ChordQuality::ALL.len()).prop_map(|(pc, q)| {
        Chord::new(
            NoteName::default_for(PitchClass::from_semitone(pc)),
            ChordQuality::ALL[q],
        )
    })
}

fn tuning() -> impl Strategy<Value = Tuning> {
    (0..TuningPreset::ALL.len()).prop_map(|i| TuningPreset::ALL[i].tuning())
}

fn constraints() -> impl Strategy<Value = VoicingConstraints> {
    (
        1..=5u8,
        1..=5u8,
        any::<bool>(),
        0..=5u8,
        0..=10u8,
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(span, fingers, open, min, extra, omit, any_bass)| VoicingConstraints {
                max_span: span,
                max_fingers: fingers,
                allow_open_strings: open,
                min_fret: min,
                max_fret: min + extra,
                allow_omitted_fifth: omit,
                min_sounding: 3,
                bass: if any_bass {
                    BassRule::AnyChordTone
                } else {
                    BassRule::ChordBass
                },
            },
        )
}

fn voicing() -> impl Strategy<Value = Voicing> {
    proptest::collection::vec(proptest::option::of(0..=15u8), 6)
        .prop_map(|v| Voicing::new(v.into_iter().map(|f| f.map(Fret)).collect()).unwrap())
}

fn pc_mask(chord: &Chord) -> u16 {
    chord
        .pitch_classes()
        .iter()
        .fold(0, |m, pc| m | (1 << pc.value()))
}

proptest! {
    #[test]
    fn g01_voicing_has_at_most_one_position_per_string(
        raw in proptest::collection::vec((1..=6u8, 0..=15u8), 0..8)
    ) {
        let positions: Vec<FretPosition> = raw
            .iter()
            .map(|(s, f)| FretPosition { string: StringIndex::new(*s).unwrap(), fret: Fret(*f) })
            .collect();
        let mut strings: Vec<u8> = raw.iter().map(|(s, _)| *s).collect();
        strings.sort_unstable();
        let duplicate = strings.windows(2).any(|w| w[0] == w[1]);
        match Voicing::from_positions(&positions, 6) {
            Ok(v) => {
                prop_assert!(!duplicate);
                prop_assert_eq!(v.sounding_count(), positions.len());
                for p in &positions {
                    prop_assert_eq!(v.fret(p.string), Some(p.fret));
                }
            }
            Err(_) => prop_assert!(duplicate),
        }
    }

    #[test]
    fn shape_text_roundtrips(v in voicing()) {
        let text = v.to_string();
        prop_assert_eq!(text.parse::<Voicing>().unwrap(), v);
    }

    #[test]
    fn g02_search_results_satisfy_constraints(
        chord in chord(),
        tuning in tuning(),
        c in constraints(),
        capo in 0..=3u8,
        limit in 1..=40usize,
    ) {
        let options = SearchOptions { constraints: c, max_results: limit };
        let found = find_voicings(&chord, &tuning, Capo(capo), &options);
        prop_assert!(found.len() <= limit);
        let allowed = pc_mask(&chord);
        let root = chord.root.pitch_class();
        for v in &found {
            prop_assert_eq!(c.check(v), Ok(()), "{}", v);
            let notes = v.sounding(&tuning, Capo(capo)).unwrap();
            let used = notes
                .iter()
                .fold(0u16, |m, n| m | (1 << PitchClass::from_semitone(n.semitone).value()));
            prop_assert_eq!(used & !allowed, 0, "{} sounds a non-chord tone", v);
            for i in chord.quality.intervals() {
                let pc = PitchClass::from_semitone((root.value() + i.semitones) as i32);
                if !(i.omissible_fifth && c.allow_omitted_fifth) {
                    prop_assert!(used & (1 << pc.value()) != 0, "{} lacks a required tone", v);
                }
            }
            if c.bass == BassRule::ChordBass {
                let lowest = notes.iter().map(|n| n.semitone).min().unwrap();
                prop_assert_eq!(PitchClass::from_semitone(lowest), root);
            }
        }
    }

    #[test]
    fn g03_candidates_cover_the_sounding_notes(v in voicing(), omit in any::<bool>()) {
        let t = Tuning::standard_guitar();
        let id = identify(&v, &t, Capo(0), IdentifyOptions { allow_omitted_fifth: omit }).unwrap();
        let sounding: u16 = id.pitch_classes.iter().fold(0, |m, pc| m | (1 << pc.value()));
        if id.pitch_classes.len() < MIN_DISTINCT_PITCH_CLASSES {
            prop_assert!(id.candidates.is_empty());
            prop_assert_eq!(
                id.status,
                IdentifyStatus::Insufficient { distinct: id.pitch_classes.len() }
            );
        } else {
            prop_assert_eq!(id.status == IdentifyStatus::Identified, !id.candidates.is_empty());
        }
        for c in &id.candidates {
            let mask = pc_mask(&c.chord);
            prop_assert_eq!(sounding & !mask, 0, "{} misses a sounding note", c.symbol);
            prop_assert!(sounding & (1 << c.chord.root.pitch_class().value()) != 0);
            for m in c.missing() {
                prop_assert!(omit && m.interval.omissible_fifth, "{} omits {:?}", c.symbol, m);
            }
            let bass = id.bass.unwrap();
            prop_assert_eq!(c.bass.name.pitch_class(), PitchClass::from_semitone(bass.semitone));
        }
    }

    #[test]
    fn g03_ranking_is_transposition_invariant(v in voicing(), k in 1..12i32) {
        let t = Tuning::standard_guitar();
        let shifted = Tuning::new(
            t.strings()
                .iter()
                .map(|p| SpelledPitch::from_semitone_sharp(p.semitone() + k))
                .collect(),
        )
        .unwrap();
        let a = identify(&v, &t, Capo(0), IdentifyOptions::default()).unwrap();
        let b = identify(&v, &shifted, Capo(0), IdentifyOptions::default()).unwrap();
        prop_assert_eq!(a.candidates.len(), b.candidates.len());
        for (x, y) in a.candidates.iter().zip(&b.candidates) {
            prop_assert_eq!(x.chord.quality, y.chord.quality);
            prop_assert_eq!(
                PitchClass::from_semitone(x.chord.root.pitch_class().value() as i32 + k),
                y.chord.root.pitch_class()
            );
        }
    }
}
