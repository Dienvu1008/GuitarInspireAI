//! Notation staff tests (ADR-041 section 8): L2 to L6 and L10.

mod check;
mod random;
mod samples;
mod svg;

use mc_notation::{
    LayoutMetrics, ScoreLayout, ShownTrack, StaffConfiguration, layout_score_systems,
    layout_score_with, outline_score_with,
};
use mc_score::Score;

pub fn both(score: &Score) -> Vec<ShownTrack> {
    score
        .tracks()
        .map(|t| ShownTrack {
            track: t.id(),
            staves: StaffConfiguration::NOTATION_AND_TAB,
        })
        .collect()
}

pub fn layout(score: &Score, m: &LayoutMetrics) -> ScoreLayout {
    layout_score_with(score, &both(score), m).expect("layout")
}

fn inputs() -> Vec<(&'static str, Score, bool)> {
    samples::all()
}

fn metrics_variants() -> Vec<LayoutMetrics> {
    vec![
        LayoutMetrics::default(),
        LayoutMetrics {
            width: 520.0,
            ..LayoutMetrics::default()
        },
    ]
}

#[test]
fn l2_l3_l4_s11_inputs_and_fixtures_have_no_violation() {
    for (name, score, _) in inputs() {
        for m in metrics_variants() {
            let l = layout(&score, &m);
            let r = check::check(&score, &l);
            assert!(r.notes > 0 || name == "rests", "{name}: no notes checked");
            assert!(
                r.violations.is_empty(),
                "{name} width {}:\n{}",
                m.width,
                r.summary()
            );
        }
    }
}

#[test]
fn l2_l3_l4_random_scores_have_no_violation() {
    let m = LayoutMetrics::default();
    let (mut notes, mut checks) = (0, 0);
    for seed in 0..300 {
        let score = random::random_score(seed);
        let l = layout(&score, &m);
        let r = check::check(&score, &l);
        notes += r.notes;
        checks += r.rule_checks;
        assert!(r.violations.is_empty(), "seed {seed}:\n{}", r.summary());
    }
    assert!(
        notes > 3000 && checks > 20000,
        "{notes} notes, {checks} checks"
    );
}

/// The random scores exercise what the checks look at (no vacuous property).
#[test]
#[ignore = "diagnosis: prints one random score's notation facts (S11_SEED)"]
fn print_random_facts() {
    let seed: u64 = option_env!("S11_SEED")
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    let score = random::random_score(seed);
    let l = layout(&score, &LayoutMetrics::default());
    println!("{}", check::check(&score, &l).summary());
    for sys in &l.systems {
        for st in sys
            .staves
            .iter()
            .filter(|s| s.kind == mc_notation::StaveKind::Notation)
        {
            for n in &st.layout.notation.facts.notes {
                println!(
                    "bar {} v{} b{} {} pos {} head x {:.2}..{:.2} ledgers {:?}",
                    n.bar,
                    n.voice,
                    n.beat,
                    n.written,
                    n.position,
                    n.head.x,
                    n.head.right(),
                    n.ledgers
                        .iter()
                        .map(|l| (l.x1, l.x2, l.y1))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}

#[test]
fn random_scores_cover_the_rules() {
    let m = LayoutMetrics::default();
    let (mut acc, mut ties, mut seconds, mut tuplets, mut two_voices, mut ledgers, mut keys) =
        (0, 0, 0, 0, 0, 0, 0);
    for seed in 0..300 {
        let score = random::random_score(seed);
        let l = layout(&score, &m);
        for sys in &l.systems {
            for st in &sys.staves {
                let f = &st.layout.notation.facts;
                acc += f.notes.iter().filter(|n| n.accidental.is_some()).count();
                ties += st.layout.notation.ties.len();
                seconds += f.notes.iter().filter(|n| n.displaced).count();
                tuplets += f.tuplets.len();
                two_voices += f.stems.iter().filter(|s| s.voice == 1).count();
                ledgers += f.notes.iter().filter(|n| !n.ledgers.is_empty()).count();
                keys += f.key_signs;
            }
        }
    }
    for (what, n) in [
        ("accidentals", acc),
        ("ties", ties),
        ("seconds", seconds),
        ("tuplets", tuplets),
        ("second-voice stems", two_voices),
        ("ledger lines", ledgers),
        ("key signs", keys),
    ] {
        assert!(n >= 20, "only {n} {what}");
    }
}

#[test]
fn l10_windowed_layout_equals_the_full_layout_and_runs_repeat() {
    let m = LayoutMetrics {
        width: 520.0,
        ..LayoutMetrics::default()
    };
    for (name, score, _) in inputs() {
        let full = layout(&score, &m);
        assert_eq!(full, layout(&score, &m), "{name}: two runs differ");
        let outline = outline_score_with(&score, &both(&score), &m).unwrap();
        assert_eq!(outline.systems.len(), full.systems.len());
        for i in 0..outline.systems.len() {
            let one = layout_score_systems(&score, &outline, i..i + 1, &m).unwrap();
            assert_eq!(one[0], full.systems[i], "{name}: system {i}");
        }
    }
}

// ------------------------------------------------------------------ L5 breakages

/// One input with every feature a breakage needs, laid out and checked clean.
fn clean(name: &str) -> (Score, ScoreLayout) {
    let (_, score, _) = inputs().into_iter().find(|(n, _, _)| *n == name).unwrap();
    let l = layout(&score, &LayoutMetrics::default());
    assert!(check::check(&score, &l).violations.is_empty());
    (score, l)
}

/// Applies `corrupt` to the first notation staff where it changes something, and
/// expects a violation of `rule`.
fn expect_caught(
    name: &str,
    rule: &str,
    corrupt: impl Fn(&mut mc_notation::NotationStaff) -> bool,
) {
    let (score, mut l) = clean(name);
    let mut changed = false;
    'outer: for sys in &mut l.systems {
        for st in &mut sys.staves {
            if st.kind == mc_notation::StaveKind::Notation && corrupt(&mut st.layout.notation) {
                changed = true;
                break 'outer;
            }
        }
    }
    assert!(
        changed,
        "{rule}: the breakage found nothing to corrupt in {name}"
    );
    let r = check::check(&score, &l);
    assert!(
        r.has(rule),
        "{rule} not caught in {name}; got:\n{}",
        r.summary()
    );
}

#[test]
fn l5_stem_rule() {
    expect_caught("scale", "stem-direction", |n| {
        n.facts.stems.first_mut().map(|s| s.up = !s.up).is_some()
    });
}

#[test]
fn l5_no_bar_reset() {
    expect_caught("keys", "accidental", |n| {
        n.facts
            .notes
            .iter_mut()
            .find(|x| x.accidental.is_some())
            .map(|x| x.accidental = None)
            .is_some()
    });
}

#[test]
fn l5_accidental_touches() {
    expect_caught("keys", "accidental-clearance", |n| {
        n.facts
            .notes
            .iter_mut()
            .find(|x| x.accidental_box.is_some())
            .map(|x| {
                let head = x.head;
                let b = x.accidental_box.as_mut().unwrap();
                b.x = head.x - b.w / 2.0;
            })
            .is_some()
    });
}

#[test]
fn l5_no_seconds() {
    expect_caught("chords", "second-sides", |n| {
        n.facts
            .notes
            .iter_mut()
            .find(|x| x.displaced)
            .map(|x| x.displaced = false)
            .is_some()
    });
}

#[test]
fn l5_dot_on_line() {
    expect_caught("tuplets", "dot-in-space", |n| {
        let top = n.lines_top;
        n.facts
            .notes
            .iter_mut()
            .find(|x| !x.dots.is_empty())
            .map(|x| x.dots[0].1 = top)
            .is_some()
    });
}

#[test]
fn l5_short_ledger() {
    expect_caught("scale", "ledger-extension", |n| {
        n.facts
            .notes
            .iter_mut()
            .find(|x| !x.ledgers.is_empty())
            .map(|x| {
                let head = x.head;
                x.ledgers[0].x2 = head.right();
            })
            .is_some()
    });
}

#[test]
fn l5_no_headroom() {
    expect_caught("scale", "inside-staff-box", |n| {
        n.glyphs
            .first_mut()
            .map(|g| g.y = -5.0 * n.staff_space)
            .is_some()
    });
}

#[test]
fn l5_no_tuplet_number() {
    expect_caught("tuplets", "tuplet-number", |n| {
        n.facts.tuplets.pop().is_some()
    });
}

#[test]
fn l5_tuplet_touches() {
    expect_caught("tuplets", "tuplet-clearance", |n| {
        let head = n.facts.notes.first().map(|x| x.head);
        match (head, n.facts.tuplets.first_mut()) {
            (Some(h), Some(t)) => {
                t.rect.x = h.x;
                t.rect.y = h.y;
                true
            }
            _ => false,
        }
    });
}

#[test]
fn l5_tuplet_by_window() {
    expect_caught("tuplets", "beam-groups", |n| {
        let Some(i) = n.facts.beams.iter().position(|b| b.beats.len() > 2) else {
            return false;
        };
        let mut g = n.facts.beams[i].clone();
        let rest = g.beats.split_off(2);
        n.facts.beams[i].beats = g.beats;
        g.beats = rest;
        n.facts.beams.push(g);
        true
    });
}

#[test]
fn l5_no_bar_rest() {
    // feature_rich has bars without beats (the `rests` input has rests as beats).
    expect_caught("feature_rich", "bar-rest", |n| {
        n.facts.bar_rests.pop().is_some()
    });
}

#[test]
fn l5_double_key() {
    expect_caught("keys", "key-signature", |n| {
        if n.facts.key_signs == 0 {
            return false;
        }
        n.facts.key_signs += 1;
        true
    });
}

// ------------------------------------------------------------------ L6 boundary

/// ADR-040: `mc-notation` takes spellings and accidentals from `mc-theory` only.
#[test]
fn l6_no_spelling_or_accidental_rule_in_mc_notation() {
    let sources = [
        ("layout.rs", include_str!("../../src/layout.rs")),
        ("engrave.rs", include_str!("../../src/engrave.rs")),
        ("semantic.rs", include_str!("../../src/semantic.rs")),
        ("display.rs", include_str!("../../src/display.rs")),
        ("staff.rs", include_str!("../../src/staff.rs")),
    ];
    let forbidden = [
        "from_semitone_sharp",
        "from_semitone_flat",
        "natural_semitone",
        ".alteration(",
        "with_semitone",
        "AccidentalState",
        "fn spell",
    ];
    for (file, text) in sources {
        // Comments may name the rules; code may not use them.
        let code: String = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for f in forbidden {
            assert!(!code.contains(f), "{file} uses {f}");
        }
        // Only the semantic layer calls the mc-theory rules.
        if file != "semantic.rs" {
            for f in ["bar_accidentals", "tie_continuation", ".spell("] {
                assert!(!code.contains(f), "{file} calls {f}");
            }
        }
    }
}

// ------------------------------------------------------------------ L10 goldens

fn golden_of(name: &str) -> &'static str {
    match name {
        "scale" => include_str!("golden/scale.txt"),
        "keys" => include_str!("golden/keys.txt"),
        "chords" => include_str!("golden/chords.txt"),
        "rhythm" => include_str!("golden/rhythm.txt"),
        "tuplets" => include_str!("golden/tuplets.txt"),
        "rests" => include_str!("golden/rests.txt"),
        "feature_rich" => include_str!("golden/feature_rich.txt"),
        _ => "",
    }
}

#[test]
fn l10_canonical_layouts_match_the_goldens() {
    let m = LayoutMetrics::default();
    let mut failed = Vec::new();
    for (name, score, _) in inputs() {
        let golden = golden_of(name);
        if name == "techniques" || name == "small" {
            continue;
        }
        let text = svg::canonical(&layout(&score, &m));
        if text != golden.replace("\r\n", "\n") {
            println!("----- BEGIN {name} -----\n{text}----- END {name} -----");
            failed.push(name);
        }
    }
    assert!(
        failed.is_empty(),
        "goldens differ (printed above): {failed:?}"
    );
}

#[test]
#[ignore = "prints the review SVGs at 2x (L12); split them with the BEGIN/END markers"]
fn print_review_svgs() {
    let m = svg::scaled(2.0);
    for (name, score, _) in inputs() {
        if name == "small" {
            continue;
        }
        let l = layout(&score, &m);
        println!(
            "----- BEGIN {name}.svg -----\n{}----- END {name}.svg -----",
            svg::render(&l)
        );
    }
}
