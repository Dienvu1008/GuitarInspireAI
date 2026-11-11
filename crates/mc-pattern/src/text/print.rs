//! The canonical printer. It writes explicit values and onsets, never `unit`, and the
//! index form of spans, so the text is exact for any valid IR.

use std::fmt::Write as _;

use mc_theory::{NoteValue, Rational};

use super::PrintError;
use crate::library::PatternLibrary;
use crate::model::{
    Action, Direction, Lane, LaneId, Origin, PatternParts, StringSelector, Technique,
};

type Res<T> = Result<T, PrintError>;

pub(super) fn print(parts: &PatternParts, lib: &PatternLibrary) -> Res<String> {
    let mut out = String::new();
    let _ = writeln!(out, "\"name\": {}", quote(&parts.name));
    let _ = writeln!(out, "\"length\": {}", rational(parts.length));
    if let Some(m) = parts.meter {
        let _ = writeln!(out, "meter {}/{}", m.numerator(), m.denominator());
    }
    if !parts.meta.description.is_empty() {
        let _ = writeln!(out, "\"description\": {}", quote(&parts.meta.description));
    }
    if !parts.meta.tags.is_empty() {
        let tags: Vec<String> = parts.meta.tags.iter().map(|t| quote(t)).collect();
        let _ = writeln!(out, "\"tags\": {}", tags.join(", "));
    }
    if let Some(o) = parts.meta.origin {
        let word = match o {
            Origin::Grid => "grid",
            Origin::Text => "text",
            Origin::Tap => "tap",
        };
        let _ = writeln!(out, "\"origin\": \"{word}\"");
    }
    let bar = parts.meter.map(|m| m.length());
    for (id, lane) in parts.lanes.iter() {
        let _ = writeln!(out, "{}: {}", lane_word(id), lane_text(id, lane, bar, lib)?);
    }
    for s in &parts.spans {
        let range = if s.first == s.last {
            format!("{}", s.first)
        } else {
            format!("{}..{}", s.first, s.last)
        };
        let _ = writeln!(
            out,
            "{} {}#{range}",
            lane_word(s.lane),
            technique(s.technique)
        );
    }
    Ok(out)
}

fn lane_word(id: LaneId) -> &'static str {
    match id {
        LaneId::Strum => "strum",
        LaneId::Thumb => "thumb",
        LaneId::Fingers => "fingers",
    }
}

fn technique(t: Technique) -> &'static str {
    match t {
        Technique::Accent => ">",
        Technique::PalmMute => "pm",
        Technique::Dead => "!",
        Technique::LetRing => "~",
        Technique::StringPercussive => "sp",
    }
}

fn rational(r: Rational) -> String {
    if r.denominator() == 1 {
        r.numerator().to_string()
    } else {
        format!("{}/{}", r.numerator(), r.denominator())
    }
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn duration(v: NoteValue) -> String {
    let mut s = (1u32 << v.base().exponent()).to_string();
    for _ in 0..v.dots() {
        s.push('.');
    }
    if let Some(t) = v.tuplet() {
        if t == mc_theory::Tuplet::TRIPLET {
            s.push('t');
        } else {
            let _ = write!(s, "({}:{})", t.actual(), t.normal());
        }
    }
    s
}

fn selector(s: StringSelector) -> String {
    match s {
        StringSelector::Bass => "Bass".into(),
        StringSelector::Alt => "Alt".into(),
        StringSelector::Treble(3) => "Treble".into(),
        StringSelector::Treble(n) => format!("Treble({n})"),
        StringSelector::Absolute(i) => i.get().to_string(),
        StringSelector::Strum => "Strum".into(),
        StringSelector::StrumRange(a, b) => format!("Strum({}..{})", a.get(), b.get()),
    }
}

fn selectors(list: &[StringSelector]) -> String {
    list.iter()
        .map(|s| selector(*s))
        .collect::<Vec<_>>()
        .join("+")
}

fn lane_text(id: LaneId, lane: &Lane, bar: Option<Rational>, lib: &PatternLibrary) -> Res<String> {
    let mut items: Vec<String> = Vec::new();
    let mut prev_end = Rational::ZERO;
    for (index, step) in lane.steps.iter().enumerate() {
        let overflow = PrintError::TimeOverflow {
            lane: id,
            step: index,
        };
        let no_targets = PrintError::NoTargets {
            lane: id,
            step: index,
        };
        let sequential = step.onset == prev_end;
        // A bar check where a bar starts, only when the step follows the previous one.
        if sequential
            && index > 0
            && let Some(len) = bar
            && prev_end
                .checked_div(len)
                .is_some_and(|q| q.denominator() == 1)
        {
            items.push("|".into());
        }
        let mut s = String::new();
        if !sequential {
            let _ = write!(s, "@{} ", rational(step.onset));
        }
        match &step.action {
            Action::Strike(k) => {
                if k.targets.is_empty() {
                    return Err(no_targets);
                }
                match k.direction {
                    Some(Direction::Down) => s.push_str("D "),
                    Some(Direction::Up) => s.push_str("U "),
                    None => {}
                }
                let t = &k.techniques;
                if t.accent {
                    s.push('>');
                }
                if t.dead {
                    s.push('!');
                }
                if t.let_ring {
                    s.push('~');
                }
                if t.palm_mute {
                    s.push_str("pm(");
                }
                if t.string_percussive {
                    s.push_str("sp(");
                }
                s.push_str(&selectors(&k.targets));
                if t.string_percussive {
                    s.push(')');
                }
                if t.palm_mute {
                    s.push(')');
                }
            }
            Action::Gesture(g) => {
                let def = lib.gesture(*g).ok_or(PrintError::UnknownGesture(*g))?;
                s.push_str(def.name());
            }
            Action::Hold(None) => s.push('_'),
            Action::Hold(Some(list)) => {
                if list.is_empty() {
                    return Err(no_targets);
                }
                let _ = write!(s, "_({})", selectors(list));
            }
            Action::Rest => s.push('-'),
        }
        let _ = write!(s, ":{}", duration(step.duration));
        items.push(s);
        prev_end = step
            .onset
            .checked_add(step.duration.length())
            .ok_or(overflow)?;
    }
    Ok(items.join(" "))
}
