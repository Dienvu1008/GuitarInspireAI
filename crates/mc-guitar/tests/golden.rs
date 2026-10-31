//! Phase 2 golden set runner (G-02, G-03). Cases live in `golden/chords.txt`
//! (format described at the top of that file). All mismatches are collected and
//! reported together. Cases marked `?` are ambiguous interpretations that are listed
//! for review; they must still produce the expected result.

use mc_guitar::{
    BassRule, ConstraintViolation, IdentifyOptions, RetuneMode, SearchOptions, TuningPreset,
    Voicing, VoicingConstraints, find_voicings, identify, plan_retune, positions_of,
    scale_positions,
};
use mc_theory::fretted::{Capo, Tuning};
use mc_theory::{NoteName, ScaleFormula, chord_symbol};
use std::collections::BTreeMap;

const GOLDEN: &str = include_str!("golden/chords.txt");

#[derive(Default)]
struct Opts {
    tuning: Option<Tuning>,
    capo: u8,
    keys: BTreeMap<String, String>,
}

impl Opts {
    fn tuning(&self) -> Tuning {
        self.tuning.clone().unwrap_or_else(Tuning::standard_guitar)
    }

    fn num(&self, key: &str) -> Option<u8> {
        self.keys
            .get(key)
            .map(|v| v.parse().expect("numeric option"))
    }

    fn constraints(&self) -> VoicingConstraints {
        let d = VoicingConstraints::default();
        VoicingConstraints {
            max_span: self.num("span").unwrap_or(d.max_span),
            max_fingers: self.num("fingers").unwrap_or(d.max_fingers),
            allow_open_strings: self.keys.get("open").is_none_or(|v| v != "no"),
            min_fret: self.num("min").unwrap_or(d.min_fret),
            max_fret: self.num("max").unwrap_or(d.max_fret),
            allow_omitted_fifth: self.keys.get("omit").is_none_or(|v| v != "no"),
            min_sounding: d.min_sounding,
            bass: match self.keys.get("bass").map(String::as_str) {
                Some("any") => BassRule::AnyChordTone,
                _ => BassRule::ChordBass,
            },
        }
    }
}

/// Splits leading `key=value` tokens off the argument list.
fn split_opts(args: &str) -> (Opts, Vec<&str>) {
    let mut o = Opts::default();
    let mut rest = Vec::new();
    for t in args.split_whitespace() {
        match t.split_once('=') {
            Some(("tuning", v)) if rest.is_empty() => {
                o.tuning = Some(TuningPreset::from_name(v).expect("preset").tuning());
            }
            Some(("capo", v)) if rest.is_empty() => o.capo = v.parse().expect("capo"),
            Some((k, v)) if rest.is_empty() => {
                o.keys.insert(k.to_string(), v.to_string());
            }
            _ => rest.push(t),
        }
    }
    (o, rest)
}

fn tuning_with_capo(spec: &str) -> (Tuning, Capo) {
    let (name, capo) = spec.split_once('@').unwrap_or((spec, "0"));
    (
        TuningPreset::from_name(name).expect("preset").tuning(),
        Capo(capo.parse().expect("capo")),
    )
}

fn shape_list(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

fn violation_name(v: ConstraintViolation) -> &'static str {
    match v {
        ConstraintViolation::Span { .. } => "span",
        ConstraintViolation::Fingers { .. } => "fingers",
        ConstraintViolation::OpenString => "open",
        ConstraintViolation::FretBelowMin { .. } => "below",
        ConstraintViolation::FretAboveMax { .. } => "above",
        ConstraintViolation::TooFewStrings { .. } => "strings",
    }
}

/// Runs one case; returns the actual result text when it differs from `expected`.
fn run(kind: &str, args: &str, expected: &str) -> Result<(), String> {
    let (o, rest) = split_opts(args);
    let subject = rest.join(" ");
    let actual: String = match kind {
        "sym" => chord_symbol::parse(&subject)
            .map(|c| chord_symbol::format(&c))
            .unwrap_or_else(|e| format!("error: {e}")),
        "sym-err" => {
            return match chord_symbol::parse(&subject) {
                Err(_) => Ok(()),
                Ok(c) => Err(format!("parsed as {}", chord_symbol::format(&c))),
            };
        }
        "tones" => {
            let chord = chord_symbol::parse(&subject).map_err(|e| e.to_string())?;
            chord
                .spelled_tones()
                .ok_or("unspellable")?
                .iter()
                .map(NoteName::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        }
        "shape-err" => {
            return match subject.parse::<Voicing>() {
                Err(_) => Ok(()),
                Ok(v) => Err(format!("parsed as {v}")),
            };
        }
        "id" | "id-err" => {
            let voicing: Voicing = subject.parse().map_err(|e| format!("{e}"))?;
            let options = IdentifyOptions {
                allow_omitted_fifth: o.keys.get("omit").is_none_or(|v| v != "no"),
            };
            let result = identify(&voicing, &o.tuning(), Capo(o.capo), options);
            if kind == "id-err" {
                return match result {
                    Err(_) => Ok(()),
                    Ok(_) => Err("identified without error".into()),
                };
            }
            let id = result.map_err(|e| e.to_string())?;
            // Detected notes are lowest first and the bass is the lowest note.
            if id.notes.windows(2).any(|w| w[0].semitone > w[1].semitone)
                || id.bass != id.notes.first().copied()
            {
                return Err("notes not ordered from the bass".into());
            }
            let names: Vec<String> = id
                .candidates
                .iter()
                .map(|c| {
                    if c.missing().next().is_some() {
                        format!("{}[no5]", c.symbol)
                    } else {
                        c.symbol.clone()
                    }
                })
                .collect();
            if names.is_empty() {
                "none".into()
            } else {
                names.join(", ")
            }
        }
        "check" => {
            let voicing: Voicing = subject.parse().map_err(|e| format!("{e}"))?;
            match o.constraints().check(&voicing) {
                Ok(()) => "ok".into(),
                Err(v) => violation_name(v).into(),
            }
        }
        "find" => return run_find(&o, &subject, expected),
        "retune" => {
            let [from, to, mode] = rest[..] else {
                return Err("retune needs from, to and mode".into());
            };
            let (old, old_capo) = tuning_with_capo(from);
            let (new, new_capo) = tuning_with_capo(to);
            let mode = RetuneMode::from_name(mode).ok_or("unknown mode")?;
            match plan_retune(&old, old_capo, &new, new_capo, mode) {
                Ok(plan) => plan.fret_deltas.map_or_else(
                    || "none".into(),
                    |d| {
                        d.iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join(" ")
                    },
                ),
                Err(e) => format!("error: {e}"),
            }
        }
        "scale" | "pos" => {
            let (window, head) = rest.split_last().ok_or("missing fret window")?;
            let (lo, hi) = window.split_once('-').ok_or("window lo-hi")?;
            let frets = lo.parse::<u8>().unwrap()..=hi.parse::<u8>().unwrap();
            let root: NoteName = head[0].parse().map_err(|_| "bad note")?;
            let positions: Vec<String> = if kind == "scale" {
                let scale = ScaleFormula::from_name(head[1]).ok_or("unknown scale")?;
                scale_positions(&o.tuning(), Capo(o.capo), root.pitch_class(), scale, frets)
                    .iter()
                    .map(|n| format!("{}:{}", n.position.string.get(), n.position.fret.0))
                    .collect()
            } else {
                positions_of(&o.tuning(), Capo(o.capo), root.pitch_class(), frets)
                    .iter()
                    .map(|p| format!("{}:{}", p.string.get(), p.fret.0))
                    .collect()
            };
            if let Some(n) = expected.strip_prefix("count ") {
                if positions.len().to_string() == n {
                    return Ok(());
                }
                return Err(format!("count {}", positions.len()));
            }
            positions.join(" ")
        }
        other => return Err(format!("unknown case kind '{other}'")),
    };
    if actual == expected {
        Ok(())
    } else {
        Err(actual)
    }
}

fn run_find(o: &Opts, symbol: &str, expected: &str) -> Result<(), String> {
    let chord = chord_symbol::parse(symbol).map_err(|e| e.to_string())?;
    let tuning = o.tuning();
    let capo = Capo(o.capo);
    let constraints = o.constraints();
    let options = SearchOptions {
        constraints,
        max_results: o.num("limit").map_or(usize::MAX, usize::from),
    };
    let found = find_voicings(&chord, &tuning, capo, &options);
    let shapes: Vec<String> = found.iter().map(|v| v.to_string()).collect();
    let mut problems = Vec::new();

    // Every result satisfies the constraints (G-02) and is named back as the chord (G-03).
    let id_options = IdentifyOptions {
        allow_omitted_fifth: constraints.allow_omitted_fifth,
    };
    for v in &found {
        if let Err(e) = constraints.check(v) {
            problems.push(format!("{v} violates constraints: {e}"));
        }
        let id = identify(v, &tuning, capo, id_options).expect("same string count");
        let named = id.candidates.iter().any(|c| {
            c.chord.root.pitch_class() == chord.root.pitch_class()
                && c.chord.quality == chord.quality
        });
        if !named {
            problems.push(format!("{v} is not identified as {symbol}"));
        }
    }

    for clause in expected.split(';').map(str::trim) {
        if clause == "none" {
            if !shapes.is_empty() {
                problems.push(format!("expected none, found {}", shapes.join(" ")));
            }
        } else if let Some(list) = clause.strip_prefix("contains ") {
            for s in shape_list(list) {
                if !shapes.contains(&canonical(&s)) {
                    problems.push(format!("missing {s}"));
                }
            }
        } else if let Some(list) = clause.strip_prefix("excludes ") {
            for s in shape_list(list) {
                if shapes.contains(&canonical(&s)) {
                    problems.push(format!("unexpected {s}"));
                }
            }
        } else if let Some(s) = clause.strip_prefix("first ") {
            if shapes.first() != Some(&canonical(s)) {
                problems.push(format!("first is {:?}", shapes.first()));
            }
        } else if let Some(n) = clause.strip_prefix("count ") {
            if shapes.len().to_string() != n {
                problems.push(format!("count {}", shapes.len()));
            }
        } else {
            problems.push(format!("unknown clause '{clause}'"));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// Shape text in the form `Voicing` displays it.
fn canonical(shape: &str) -> String {
    shape
        .parse::<Voicing>()
        .map(|v| v.to_string())
        .unwrap_or_else(|_| format!("<invalid {shape}>"))
}

#[test]
fn g03_golden_set() {
    let mut cases = 0;
    let mut failures = Vec::new();
    let mut ambiguous = Vec::new();
    for (n, raw) in GOLDEN.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        cases += 1;
        let (line, flagged) = match line.strip_suffix(" ?") {
            Some(l) => (l, true),
            None => (line, false),
        };
        let (head, expected) = line.split_once(" => ").unwrap_or((line, ""));
        let (kind, args) = head.split_once(' ').unwrap_or((head, ""));
        if flagged {
            ambiguous.push(format!("line {}: {line}", n + 1));
        }
        if let Err(actual) = run(kind, args, expected.trim()) {
            failures.push(format!("line {}: {line}\n    actual: {actual}", n + 1));
        }
    }
    eprintln!(
        "golden set: {cases} cases, {} flagged ambiguous:\n  {}",
        ambiguous.len(),
        ambiguous.join("\n  ")
    );
    assert!(
        cases >= 100,
        "golden set needs at least 100 cases, has {cases}"
    );
    assert!(
        failures.is_empty(),
        "{} golden case(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
