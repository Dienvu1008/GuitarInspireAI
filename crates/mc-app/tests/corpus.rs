//! ADR-034 PE-11: every confirmed corpus pattern (pattern 7 is not a golden), written
//! in the text syntax, is applied to its voicing and checked two ways:
//!
//! - the TAB it produces equals the table of `pattern-corpus-proposed.md`, transcribed
//!   here by hand (columns are eighths of the bar; marks are `D`/`U` strokes, `PM`,
//!   `>` and `S`); and
//! - the playback timeline compiled from the resulting score equals a golden file.
//!
//! The DSL files live in `mc-pattern/tests/corpus/`; `mc-pattern` cannot depend on
//! `mc-playback`, so the golden checks run here. After an intended change, rewrite
//! the golden files and review the diff:
//!   cargo test -p mc-app --test corpus -- --ignored update_goldens

mod common;
use common::*;
use mc_app::pattern::PatternId;
use mc_app::types::{MasterBar, Score, TimeSignature};
use mc_app::{ApplyPattern, Command, DocumentSession, PatternCommand, ScoreCommand};
use mc_playback::{CompileOptions, compile};
use mc_score::Stroke;
use mc_theory::Rational;

type Voicing = [Option<u8>; 6];

const AM: Voicing = [Some(0), Some(1), Some(2), Some(2), Some(0), None]; // x02210
const G: Voicing = [Some(3), Some(0), Some(0), Some(0), Some(2), Some(3)]; // 320003
const C: Voicing = [Some(0), Some(1), Some(0), Some(2), Some(3), None]; // x32010

struct Case {
    id: &'static str,
    text: &'static str,
    chord: &'static str,
    voicing: Voicing,
    /// Time signature of each bar the pattern is applied to.
    bars: &'static [(u8, u8)],
    /// Rows e, B, G, D, A, E and Marks; one cell per eighth, `.` for nothing new.
    tab: [&'static str; 7],
}

const CASES: [Case; 9] = [
    Case {
        id: "1",
        text: include_str!("../../mc-pattern/tests/corpus/p01-basic-fingerpicking.pattern"),
        chord: "Am",
        voicing: AM,
        bars: &[(4, 4)],
        tab: [
            ". . . . 0 . . .",
            ". . 1 . . . 1 .",
            ". 2 . 2 . 2 . 2",
            ". . . . . . . .",
            "0 . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
        ],
    },
    Case {
        id: "2",
        text: include_str!("../../mc-pattern/tests/corpus/p02-travis-picking.pattern"),
        chord: "G",
        voicing: G,
        bars: &[(4, 4)],
        tab: [
            "3 . . 3 . . . 3",
            ". 0 . . . 0 . .",
            ". . . . . . . .",
            ". . 0 . . . 0 .",
            ". . . . . . . .",
            "3 . . . 3 . . .",
            ". . . . . . . .",
        ],
    },
    Case {
        id: "3",
        text: include_str!("../../mc-pattern/tests/corpus/p03-boom-chick.pattern"),
        chord: "C",
        voicing: C,
        bars: &[(4, 4)],
        tab: [
            ". . 0 . . . 0 .",
            ". . 1 . . . 1 .",
            ". . 0 . . . 0 .",
            ". . . . 2 . . .",
            "3 . . . . . . .",
            ". . . . . . . .",
            ". . D . . . D .",
        ],
    },
    Case {
        id: "4",
        text: include_str!("../../mc-pattern/tests/corpus/p04-folk-strumming.pattern"),
        chord: "C",
        voicing: C,
        bars: &[(4, 4)],
        tab: [
            "0 . 0 0 . 0 0 0",
            "1 . 1 1 . 1 1 1",
            "0 . 0 0 . 0 0 0",
            "2 . 2 2 . 2 2 2",
            "3 . 3 . . . 3 .",
            ". . . . . . . .",
            "D . D U . U D U",
        ],
    },
    Case {
        id: "5",
        text: include_str!("../../mc-pattern/tests/corpus/p05-down-up-eighths.pattern"),
        chord: "G",
        voicing: G,
        bars: &[(4, 4)],
        tab: [
            "3 3 3 3 3 3 3 3",
            "0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0 0",
            "2 . 2 . 2 . 2 .",
            "3 . 3 . 3 . 3 .",
            "D U D U D U D U",
        ],
    },
    Case {
        id: "6",
        text: include_str!("../../mc-pattern/tests/corpus/p06-waltz.pattern"),
        chord: "G",
        voicing: G,
        bars: &[(3, 4)],
        tab: [
            ". . 3 . 3 .",
            ". . 0 . 0 .",
            ". . 0 . 0 .",
            ". . . . . .",
            ". . . . . .",
            "3 . . . . .",
            ". . D . D .",
        ],
    },
    Case {
        id: "8",
        text: include_str!("../../mc-pattern/tests/corpus/p08-palm-muted-bass.pattern"),
        chord: "G",
        voicing: G,
        bars: &[(4, 4)],
        tab: [
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            "3 3 3 3 3 3 3 3",
            "PM+> PM PM PM PM+> PM PM PM",
        ],
    },
    Case {
        id: "9",
        text: include_str!(
            "../../mc-pattern/tests/corpus/p09-string-percussive-boom-chuck.pattern"
        ),
        chord: "C",
        voicing: C,
        bars: &[(4, 4)],
        tab: [
            "0 . x . 0 0 x .",
            "1 . x . 1 1 x .",
            "0 . x . 0 0 x .",
            "2 . x . 2 2 x .",
            "3 . x . 3 . x .",
            ". . . . . . . .",
            "D . D+>+S . D U D+>+S .",
        ],
    },
    Case {
        id: "10",
        text: include_str!("../../mc-pattern/tests/corpus/p10-users-example.pattern"),
        chord: "C",
        voicing: C,
        bars: &[(2, 4), (2, 4)],
        tab: [
            ". . 0 . . . 0 0",
            ". . 1 . . . 1 1",
            ". . 0 . . . 0 0",
            ". . . . . . . .",
            "3 . . . 3 . . .",
            ". . . . . . . .",
            ". . . . . . . .",
        ],
    },
];

/// A fresh session with the case's bars, the pattern defined from its text, and the
/// pattern applied to every bar.
fn applied(case: &Case) -> DocumentSession {
    let mut s = session(case.bars.len());
    let t = first_track(&s);
    for (i, &(n, d)) in case.bars.iter().enumerate() {
        let bar = MasterBar {
            time_signature: TimeSignature::new(n, d).unwrap(),
            ..MasterBar::default()
        };
        s.execute(cmd(
            100 + i as u64,
            Command::Score(ScoreCommand::SetMasterBar { index: i, bar }),
        ))
        .unwrap();
    }
    let p: PatternId = s
        .patterns_mut()
        .define_pattern_text(case.text)
        .unwrap_or_else(|e| panic!("pattern {}: {e:?}", case.id));
    s.execute(cmd(
        200,
        Command::Pattern(PatternCommand::ApplyPattern(ApplyPattern {
            track: t,
            first: 0,
            last: case.bars.len() - 1,
            pattern: p,
            voicing: case.voicing.to_vec(),
            chord: case.chord.into(),
            fit: None,
            overwrite: false,
        })),
    ))
    .unwrap_or_else(|e| panic!("pattern {}: {e:?}", case.id));
    s
}

/// The TAB of the applied bars: seven rows (strings 1 to 6, then marks), one cell per
/// eighth, bars one after the other.
fn tab_of(score: &Score, bars: usize, cols_per_bar: usize) -> Vec<String> {
    let track = score.track_at(0).unwrap();
    let mut cells = vec![vec![".".to_string(); bars * cols_per_bar]; 7];
    let eighth = Rational::new(1, 8).unwrap();
    for b in 0..bars {
        for voice in &track.bar(b).unwrap().voices {
            let mut at = Rational::ZERO;
            for beat in &voice.beats {
                let col = at.checked_div(eighth).unwrap();
                assert_eq!(col.denominator(), 1, "beat off the eighth grid");
                let col = b * cols_per_bar + usize::try_from(col.numerator()).unwrap();
                at = at.checked_add(beat.value.length()).unwrap();
                let Some(first) = beat.notes.first() else {
                    continue;
                };
                for n in &beat.notes {
                    assert_eq!(n.effects, first.effects, "one beat, one set of marks");
                    let p = n.position();
                    let row = usize::from(p.string.get()) - 1;
                    assert_eq!(cells[row][col], ".", "two notes in one TAB cell");
                    cells[row][col] = if n.effects.string_percussive {
                        "x".into()
                    } else {
                        p.fret.0.to_string()
                    };
                }
                let mut marks = Vec::new();
                match beat.stroke {
                    Some(Stroke::Down) => marks.push("D"),
                    Some(Stroke::Up) => marks.push("U"),
                    None => {}
                }
                if first.effects.palm_mute {
                    marks.push("PM");
                }
                if first.effects.accent {
                    marks.push(">");
                }
                if first.effects.string_percussive {
                    marks.push("S");
                }
                if !marks.is_empty() {
                    cells[6][col] = marks.join("+");
                }
            }
        }
    }
    cells.into_iter().map(|row| row.join(" ")).collect()
}

fn timeline_text(s: &DocumentSession) -> String {
    compile(s.score(), &CompileOptions::new())
        .unwrap()
        .timeline
        .canonical_text()
}

fn golden(id: &str) -> &'static str {
    match id {
        "1" => include_str!("golden/corpus_p01.timeline.txt"),
        "2" => include_str!("golden/corpus_p02.timeline.txt"),
        "3" => include_str!("golden/corpus_p03.timeline.txt"),
        "4" => include_str!("golden/corpus_p04.timeline.txt"),
        "5" => include_str!("golden/corpus_p05.timeline.txt"),
        "6" => include_str!("golden/corpus_p06.timeline.txt"),
        "8" => include_str!("golden/corpus_p08.timeline.txt"),
        "9" => include_str!("golden/corpus_p09.timeline.txt"),
        "10" => include_str!("golden/corpus_p10.timeline.txt"),
        other => panic!("no golden for pattern {other}"),
    }
}

#[test]
fn pe11_each_corpus_pattern_produces_the_tab_of_the_corpus_document() {
    for case in &CASES {
        let s = applied(case);
        let cols = case.tab[0].split(' ').count();
        let bars = case.bars.len();
        let got = tab_of(s.score(), bars, cols / bars);
        let want: Vec<String> = case.tab.iter().map(|r| (*r).to_string()).collect();
        assert_eq!(got, want, "pattern {}: TAB rows e B G D A E Marks", case.id);
    }
}

#[test]
fn pe11_each_corpus_pattern_compiles_to_its_golden_timeline() {
    for case in &CASES {
        let got = timeline_text(&applied(case));
        let want = golden(case.id).replace("\r\n", "\n");
        if got != want {
            eprintln!(
                "--- pattern {}: actual canonical timeline ---\n{got}--- end ---",
                case.id
            );
            panic!(
                "pattern {}: golden timeline differs; actual text printed above",
                case.id
            );
        }
    }
}

#[test]
fn pe11_the_corpus_applies_the_same_way_twice() {
    // Applying is deterministic: same text, same ids, same bytes.
    for case in &CASES {
        let (a, b) = (applied(case), applied(case));
        assert_eq!(
            mc_project::score_json(a.score()),
            mc_project::score_json(b.score()),
            "pattern {}",
            case.id
        );
    }
}

/// Rewrites the golden files. Review the diff against the corpus document first.
#[test]
#[ignore = "rewrites the golden files"]
fn update_goldens() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    std::fs::create_dir_all(&dir).unwrap();
    for case in &CASES {
        let name = format!("corpus_p{:0>2}.timeline.txt", case.id);
        std::fs::write(dir.join(name), timeline_text(&applied(case))).unwrap();
    }
}
