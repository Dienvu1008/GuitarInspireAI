//! Spike-only MusicXML of the notation staff (README section 3 step 3), for MuseScore
//! 4.7.5 to render as the N4 reference, and to re-export so its own accidental
//! decisions can be compared with ours. No `<accidental>` elements are written: the
//! reader decides from the pitches, key and its own rules.
//!
//! Guitar convention: `<clef>` G on line 2 with `<clef-octave-change>-1`, and `<pitch>`
//! is the sounding pitch (the clef puts it an octave higher on the staff).

use crate::pitch;
use mc_score::Score;
use mc_theory::fretted::pitch_at;
use std::fmt::Write;

const DIVISIONS: i64 = 960;

fn type_name(exponent: u8) -> &'static str {
    ["whole", "half", "quarter", "eighth", "16th", "32nd", "64th", "128th", "256th"][exponent as usize]
}

pub fn write(score: &Score) -> String {
    let mut s = String::new();
    let _ = writeln!(s, r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#);
    let _ = writeln!(s, r#"<score-partwise version="4.0"><part-list>"#);
    for (t, track) in score.tracks().enumerate() {
        let _ = writeln!(s, r#"<score-part id="P{}"><part-name>{}</part-name></score-part>"#, t + 1, track.name());
    }
    let _ = writeln!(s, "</part-list>");
    let masters = score.master_bars();
    for (t, track) in score.tracks().enumerate() {
        let inst = track.fretted();
        // Tie sources: the note on the same string in the beat before each tied note.
        let mut sources = std::collections::BTreeSet::new();
        for (b, bar) in track.bars().enumerate() {
            for (vi, voice) in bar.voices.iter().enumerate() {
                for (k, beat) in voice.beats.iter().enumerate() {
                    for n in beat.notes.iter().filter(|n| n.tie) {
                        let prev = if k > 0 {
                            Some((b, k - 1))
                        } else {
                            b.checked_sub(1).and_then(|pb| {
                                let len = track.bar(pb)?.voices.get(vi)?.beats.len();
                                Some((pb, len.checked_sub(1)?))
                            })
                        };
                        if let Some((pb, pk)) = prev {
                            sources.insert((pb, vi, pk, n.position().string.get()));
                        }
                    }
                }
            }
        }
        let _ = writeln!(s, r#"<part id="P{}">"#, t + 1);
        for (b, bar) in track.bars().enumerate() {
            let m = &masters[b];
            let _ = writeln!(s, r#"<measure number="{}">"#, b + 1);
            let prev = b.checked_sub(1).map(|p| &masters[p]);
            if prev.is_none_or(|p| p.key != m.key || p.time_signature != m.time_signature) {
                let _ = write!(s, "<attributes>");
                if b == 0 {
                    let _ = write!(s, "<divisions>{DIVISIONS}</divisions>");
                }
                let _ = write!(s, "<key><fifths>{}</fifths></key>", m.key.fifths());
                let _ = write!(
                    s,
                    "<time><beats>{}</beats><beat-type>{}</beat-type></time>",
                    m.time_signature.numerator(),
                    m.time_signature.denominator()
                );
                if b == 0 {
                    let _ = write!(s, "<clef><sign>G</sign><line>2</line><clef-octave-change>-1</clef-octave-change></clef>");
                }
                let _ = writeln!(s, "</attributes>");
            }
            let voices: Vec<_> = bar.voices.iter().filter(|v| !v.beats.is_empty()).collect();
            for (vi, voice) in bar.voices.iter().enumerate() {
                if voice.beats.is_empty() {
                    continue;
                }
                if vi > 0 {
                    let len = bar.voices[vi - 1].beats.iter().map(|b| b.value.length()).fold(mc_theory::Rational::ZERO, |a, l| a.checked_add(l).unwrap());
                    let _ = writeln!(s, "<backup><duration>{}</duration></backup>", len.numerator() * DIVISIONS * 4 / len.denominator());
                }
                for (k, beat) in voice.beats.iter().enumerate() {
                    let len = beat.value.length();
                    let dur = len.numerator() * DIVISIONS * 4 / len.denominator();
                    let common = |s: &mut String| {
                        let _ = write!(s, "<duration>{dur}</duration><voice>{}</voice><type>{}</type>", vi + 1, type_name(beat.value.base().exponent()));
                        for _ in 0..beat.value.dots() {
                            let _ = write!(s, "<dot/>");
                        }
                        if let Some(t) = beat.value.tuplet() {
                            let _ = write!(s, "<time-modification><actual-notes>{}</actual-notes><normal-notes>{}</normal-notes></time-modification>", t.actual(), t.normal());
                        }
                        if voices.len() > 1 {
                            let _ = write!(s, "<stem>{}</stem>", if vi == 0 { "up" } else { "down" });
                        }
                    };
                    if beat.notes.is_empty() {
                        let _ = write!(s, "<note><rest/>");
                        common(&mut s);
                        let _ = writeln!(s, "</note>");
                        continue;
                    }
                    let key = m.key;
                    for (i, n) in beat.notes.iter().enumerate() {
                        let sounding = pitch_at(&inst.tuning, inst.capo, n.position()).unwrap();
                        let p = pitch::spell(sounding, key);
                        let _ = write!(s, "<note>");
                        if i > 0 {
                            let _ = write!(s, "<chord/>");
                        }
                        let _ = write!(s, "<pitch><step>{:?}</step>", p.letter());
                        if p.alter().semitones() != 0 {
                            let _ = write!(s, "<alter>{}</alter>", p.alter().semitones());
                        }
                        let _ = write!(s, "<octave>{}</octave></pitch>", p.octave());
                        let start = sources.contains(&(b, vi, k, n.position().string.get()));
                        let _ = write!(s, "<duration>{dur}</duration>");
                        if n.tie {
                            let _ = write!(s, r#"<tie type="stop"/>"#);
                        }
                        if start {
                            let _ = write!(s, r#"<tie type="start"/>"#);
                        }
                        let _ = write!(s, "<voice>{}</voice><type>{}</type>", vi + 1, type_name(beat.value.base().exponent()));
                        for _ in 0..beat.value.dots() {
                            let _ = write!(s, "<dot/>");
                        }
                        if let Some(t) = beat.value.tuplet() {
                            let _ = write!(s, "<time-modification><actual-notes>{}</actual-notes><normal-notes>{}</normal-notes></time-modification>", t.actual(), t.normal());
                        }
                        if voices.len() > 1 {
                            let _ = write!(s, "<stem>{}</stem>", if vi == 0 { "up" } else { "down" });
                        }
                        if n.tie || start {
                            let _ = write!(s, "<notations>");
                            if n.tie {
                                let _ = write!(s, r#"<tied type="stop"/>"#);
                            }
                            if start {
                                let _ = write!(s, r#"<tied type="start"/>"#);
                            }
                            let _ = write!(s, "</notations>");
                        }
                        let _ = writeln!(s, "</note>");
                    }
                }
            }
            let _ = writeln!(s, "</measure>");
        }
        let _ = writeln!(s, "</part>");
    }
    let _ = writeln!(s, "</score-partwise>");
    s
}
