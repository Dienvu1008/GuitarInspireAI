//! A deterministic reference library (feature `fixtures`), for the format 1.2 fixtures
//! and tests in other crates (ADR-034 section 8). It uses every part of the IR the
//! stored form must carry: both lane shapes, every selector, every technique as a
//! step flag and as a span, directions, gestures (with non-ASCII names), holds of all
//! or some strings, rests and gaps, dotted and tuplet values, a meter, and the meta.
//! One pattern is removed after it is defined, so the id counter is ahead of the ids.

use crate::{Direction, PatternLibrary, Strike, StringSelector, TechniqueSet};

const TRAVIS: &str = r#""name": "Travis"
"length": 1
"description": "Corpus pattern 2: thumb on the bass and the alternate bass, fingers on strings 1 and 2"
"tags": "corpus", "fingerstyle"
"origin": "text"
thumb: Bass:4 Alt:4 Bass:4 Alt:4
fingers: 1:8 2:8 @3/8 1:8 @5/8 2:8 @7/8 1:8
thumb ~#1..3
"#;

const PALM_MUTED: &str = r#""name": "Palm-muted bass"
"length": 1
meter 4/4
"origin": "grid"
strum: >Bass:8 Bass:8 Bass:8 Bass:8 >Bass:8 Bass:8 Bass:8 Bass:8
strum pm#0..7
"#;

const GESTURES: &str = r#""name": "Gestures and holds"
"length": 3/2
meter 3/4
"tags": "Bùm-kịch"
strum: Bùm:4 Kịch:8 D Strum:8 _:8 XL:4 _(Bass):4 -:8 U ~Strum(6..4)+Treble(2):8t U !3:8t D pm(sp(Treble+1)):8t
strum >#0..1
strum ~#3
"#;

const VALUES: &str = r#""name": "Values"
"length": 2
"origin": "tap"
thumb: Bass:4. Alt:8 Bass:2 @3/2 Alt:16(5:4)
fingers: -:2 D Strum:4.. @1 U Treble(4):8 6:2
fingers sp#2
"#;

const REMOVED: &str = r#""name": "Removed"
"length": 1/4
strum: Bass:4
"#;

/// The reference library. Building it never fails.
pub fn library() -> PatternLibrary {
    let mut lib = PatternLibrary::new();
    let gestures = [
        (
            "Bùm",
            Strike {
                techniques: TechniqueSet {
                    palm_mute: true,
                    accent: true,
                    ..TechniqueSet::default()
                },
                ..Strike::on(vec![StringSelector::Bass])
            },
        ),
        (
            "Kịch",
            Strike {
                direction: Some(Direction::Down),
                techniques: TechniqueSet {
                    string_percussive: true,
                    ..TechniqueSet::default()
                },
                ..Strike::on(vec![StringSelector::Strum])
            },
        ),
        (
            "XL",
            Strike {
                techniques: TechniqueSet {
                    accent: true,
                    let_ring: true,
                    ..TechniqueSet::default()
                },
                ..Strike::on(vec![StringSelector::Bass, StringSelector::TREBLE])
            },
        ),
    ];
    for (name, strike) in gestures {
        lib.define_gesture(name, strike)
            .expect("the reference gestures are valid");
    }
    for text in [TRAVIS, REMOVED, PALM_MUTED, GESTURES, VALUES] {
        lib.define_pattern_text(text)
            .unwrap_or_else(|e| panic!("reference pattern: {e:?}\n{text}"));
    }
    let removed = lib
        .patterns()
        .find(|p| p.name() == "Removed")
        .map(|p| p.id())
        .expect("defined above");
    lib.remove_pattern(removed).expect("unused");
    lib
}
