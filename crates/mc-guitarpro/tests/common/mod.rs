//! Shared test helpers: the S10 fixtures (embedded, FF-11) and a text description of
//! a score used to compare scores and to read test failures.
#![allow(dead_code)]

use mc_guitarpro::GpVersion;
use mc_score::{Score, Stroke};

/// The 54 S10 fixtures written by TuxGuitar 2.0.1 (CC0-1.0, see the fixture README).
pub const FIXTURES: &[(&str, &[u8])] = &[
    (
        "f01-empty.gp3",
        include_bytes!("../fixtures/tuxguitar/f01-empty.gp3"),
    ),
    (
        "f01-empty.gp4",
        include_bytes!("../fixtures/tuxguitar/f01-empty.gp4"),
    ),
    (
        "f01-empty.gp5",
        include_bytes!("../fixtures/tuxguitar/f01-empty.gp5"),
    ),
    (
        "f02-one-note.gp3",
        include_bytes!("../fixtures/tuxguitar/f02-one-note.gp3"),
    ),
    (
        "f02-one-note.gp4",
        include_bytes!("../fixtures/tuxguitar/f02-one-note.gp4"),
    ),
    (
        "f02-one-note.gp5",
        include_bytes!("../fixtures/tuxguitar/f02-one-note.gp5"),
    ),
    (
        "f03-chord.gp3",
        include_bytes!("../fixtures/tuxguitar/f03-chord.gp3"),
    ),
    (
        "f03-chord.gp4",
        include_bytes!("../fixtures/tuxguitar/f03-chord.gp4"),
    ),
    (
        "f03-chord.gp5",
        include_bytes!("../fixtures/tuxguitar/f03-chord.gp5"),
    ),
    (
        "f04-durations.gp3",
        include_bytes!("../fixtures/tuxguitar/f04-durations.gp3"),
    ),
    (
        "f04-durations.gp4",
        include_bytes!("../fixtures/tuxguitar/f04-durations.gp4"),
    ),
    (
        "f04-durations.gp5",
        include_bytes!("../fixtures/tuxguitar/f04-durations.gp5"),
    ),
    (
        "f05-note-effects.gp3",
        include_bytes!("../fixtures/tuxguitar/f05-note-effects.gp3"),
    ),
    (
        "f05-note-effects.gp4",
        include_bytes!("../fixtures/tuxguitar/f05-note-effects.gp4"),
    ),
    (
        "f05-note-effects.gp5",
        include_bytes!("../fixtures/tuxguitar/f05-note-effects.gp5"),
    ),
    (
        "f06-strokes.gp3",
        include_bytes!("../fixtures/tuxguitar/f06-strokes.gp3"),
    ),
    (
        "f06-strokes.gp4",
        include_bytes!("../fixtures/tuxguitar/f06-strokes.gp4"),
    ),
    (
        "f06-strokes.gp5",
        include_bytes!("../fixtures/tuxguitar/f06-strokes.gp5"),
    ),
    (
        "f07-tracks.gp3",
        include_bytes!("../fixtures/tuxguitar/f07-tracks.gp3"),
    ),
    (
        "f07-tracks.gp4",
        include_bytes!("../fixtures/tuxguitar/f07-tracks.gp4"),
    ),
    (
        "f07-tracks.gp5",
        include_bytes!("../fixtures/tuxguitar/f07-tracks.gp5"),
    ),
    (
        "f08-meter-key-tempo.gp3",
        include_bytes!("../fixtures/tuxguitar/f08-meter-key-tempo.gp3"),
    ),
    (
        "f08-meter-key-tempo.gp4",
        include_bytes!("../fixtures/tuxguitar/f08-meter-key-tempo.gp4"),
    ),
    (
        "f08-meter-key-tempo.gp5",
        include_bytes!("../fixtures/tuxguitar/f08-meter-key-tempo.gp5"),
    ),
    (
        "f09-repeats.gp3",
        include_bytes!("../fixtures/tuxguitar/f09-repeats.gp3"),
    ),
    (
        "f09-repeats.gp4",
        include_bytes!("../fixtures/tuxguitar/f09-repeats.gp4"),
    ),
    (
        "f09-repeats.gp5",
        include_bytes!("../fixtures/tuxguitar/f09-repeats.gp5"),
    ),
    (
        "f10-two-voices.gp3",
        include_bytes!("../fixtures/tuxguitar/f10-two-voices.gp3"),
    ),
    (
        "f10-two-voices.gp4",
        include_bytes!("../fixtures/tuxguitar/f10-two-voices.gp4"),
    ),
    (
        "f10-two-voices.gp5",
        include_bytes!("../fixtures/tuxguitar/f10-two-voices.gp5"),
    ),
    (
        "f11-text.gp3",
        include_bytes!("../fixtures/tuxguitar/f11-text.gp3"),
    ),
    (
        "f11-text.gp4",
        include_bytes!("../fixtures/tuxguitar/f11-text.gp4"),
    ),
    (
        "f11-text.gp5",
        include_bytes!("../fixtures/tuxguitar/f11-text.gp5"),
    ),
    (
        "f12-unsupported.gp3",
        include_bytes!("../fixtures/tuxguitar/f12-unsupported.gp3"),
    ),
    (
        "f12-unsupported.gp4",
        include_bytes!("../fixtures/tuxguitar/f12-unsupported.gp4"),
    ),
    (
        "f12-unsupported.gp5",
        include_bytes!("../fixtures/tuxguitar/f12-unsupported.gp5"),
    ),
    (
        "f13-seven-string.gp3",
        include_bytes!("../fixtures/tuxguitar/f13-seven-string.gp3"),
    ),
    (
        "f13-seven-string.gp4",
        include_bytes!("../fixtures/tuxguitar/f13-seven-string.gp4"),
    ),
    (
        "f13-seven-string.gp5",
        include_bytes!("../fixtures/tuxguitar/f13-seven-string.gp5"),
    ),
    (
        "f14-percussion.gp3",
        include_bytes!("../fixtures/tuxguitar/f14-percussion.gp3"),
    ),
    (
        "f14-percussion.gp4",
        include_bytes!("../fixtures/tuxguitar/f14-percussion.gp4"),
    ),
    (
        "f14-percussion.gp5",
        include_bytes!("../fixtures/tuxguitar/f14-percussion.gp5"),
    ),
    (
        "f15-chord-diagram.gp3",
        include_bytes!("../fixtures/tuxguitar/f15-chord-diagram.gp3"),
    ),
    (
        "f15-chord-diagram.gp4",
        include_bytes!("../fixtures/tuxguitar/f15-chord-diagram.gp4"),
    ),
    (
        "f15-chord-diagram.gp5",
        include_bytes!("../fixtures/tuxguitar/f15-chord-diagram.gp5"),
    ),
    (
        "f16-beat-text.gp3",
        include_bytes!("../fixtures/tuxguitar/f16-beat-text.gp3"),
    ),
    (
        "f16-beat-text.gp4",
        include_bytes!("../fixtures/tuxguitar/f16-beat-text.gp4"),
    ),
    (
        "f16-beat-text.gp5",
        include_bytes!("../fixtures/tuxguitar/f16-beat-text.gp5"),
    ),
    (
        "f17-combined-endings.gp3",
        include_bytes!("../fixtures/tuxguitar/f17-combined-endings.gp3"),
    ),
    (
        "f17-combined-endings.gp4",
        include_bytes!("../fixtures/tuxguitar/f17-combined-endings.gp4"),
    ),
    (
        "f17-combined-endings.gp5",
        include_bytes!("../fixtures/tuxguitar/f17-combined-endings.gp5"),
    ),
    (
        "f18-lyrics.gp3",
        include_bytes!("../fixtures/tuxguitar/f18-lyrics.gp3"),
    ),
    (
        "f18-lyrics.gp4",
        include_bytes!("../fixtures/tuxguitar/f18-lyrics.gp4"),
    ),
    (
        "f18-lyrics.gp5",
        include_bytes!("../fixtures/tuxguitar/f18-lyrics.gp5"),
    ),
];

pub fn fixture(name: &str) -> &'static [u8] {
    FIXTURES
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("no fixture {name}"))
        .1
}

/// Every fact the importer and exporter keep, one line per bar property, track and
/// voice. Two scores with the same text hold the same music (ids aside).
pub fn describe(score: &Score) -> String {
    render(score, None, true)
}

/// `describe` without the tempo map (tempo positions are checked on their own).
pub fn describe_without_tempo(score: &Score) -> String {
    render(score, None, false)
}

/// What `describe` of the re-import must give after an export to `v`: the score with
/// the approximations of ADR-036 section 4 applied (tempo map left out).
pub fn expected_after_export(score: &Score, v: GpVersion) -> String {
    render(score, Some(v), false)
}

/// Normal count of a tuplet in Guitar Pro's fixed table (ADR-036 section 3).
pub fn gp_tuplet_normal(actual: u8) -> Option<u8> {
    match actual {
        3 => Some(2),
        5..=7 => Some(4),
        9..=13 => Some(8),
        _ => None,
    }
}

/// The longest prefix of at most `max` bytes on a character boundary.
pub fn cut(s: &str, max: usize) -> &str {
    let mut end = s.len().min(max);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

fn render(score: &Score, export: Option<GpVersion>, tempo: bool) -> String {
    let voices = export.map_or(usize::MAX, |v| if v == GpVersion::Gp5 { 2 } else { 1 });
    let gp3 = export == Some(GpVersion::Gp3);
    let mut out = Vec::new();
    let mut last = None;
    for (i, b) in score.master_bars().iter().enumerate() {
        let mut p = Vec::new();
        let ts = (b.time_signature.numerator(), b.time_signature.denominator());
        let key = (b.key.fifths(), format!("{:?}", b.key.mode()));
        if last.as_ref() != Some(&(ts, key.clone())) {
            p.push(format!("{}/{} key {} {}", ts.0, ts.1, key.0, key.1));
        }
        last = Some((ts, key));
        if b.pickup && export.is_none() {
            p.push("pickup".into());
        }
        if b.repeat_start {
            p.push("|:".into());
        }
        if let Some(n) = b.repeat_end {
            p.push(format!(":| x{n}"));
        }
        let endings: Vec<u8> = b
            .alternate_endings
            .iter()
            .copied()
            .filter(|p| export.is_none() || *p <= 8)
            .collect();
        if !endings.is_empty() {
            p.push(format!("volta {endings:?}"));
        }
        if let Some(m) = &b.marker {
            let m = if export.is_some() { cut(m, 255) } else { m };
            p.push(format!("marker {m:?}"));
        }
        if !p.is_empty() {
            out.push(format!("bar {}: {}", i + 1, p.join(", ")));
        }
    }
    if tempo {
        for t in score.tempo_map() {
            out.push(format!(
                "tempo {} at bar {} + {}",
                t.tempo.bpm(),
                t.bar + 1,
                t.offset
            ));
        }
    }
    for (ti, t) in score.tracks().enumerate() {
        let f = t.fretted();
        let tuning: Vec<i32> = f.tuning.strings().iter().map(|p| p.semitone()).collect();
        let name = if export.is_some() {
            cut(t.name(), 40)
        } else {
            t.name()
        };
        out.push(format!(
            "track {} {:?} tuning {:?} frets {} capo {}",
            ti + 1,
            name,
            tuning,
            f.fret_count,
            f.capo.0
        ));
        for (m, bar) in t.bars().enumerate() {
            for (vi, voice) in bar.voices.iter().enumerate().take(voices) {
                if voice.beats.is_empty() {
                    continue;
                }
                let beats: Vec<String> = voice
                    .beats
                    .iter()
                    .map(|b| {
                        let v = b.value;
                        let exp = if export.is_some() {
                            v.base().exponent().min(6)
                        } else {
                            v.base().exponent()
                        };
                        let dots = if export.is_some() {
                            v.dots().min(1)
                        } else {
                            v.dots()
                        };
                        let mut s = format!("1/{}", 1u32 << exp);
                        for _ in 0..dots {
                            s.push('.');
                        }
                        let tuplet = v.tuplet().map(|t| (t.actual(), t.normal()));
                        let tuplet = match export {
                            None => tuplet,
                            Some(_) => tuplet.and_then(|(a, _)| {
                                gp_tuplet_normal(a).filter(|n| *n != a).map(|n| (a, n))
                            }),
                        };
                        if let Some((a, n)) = tuplet {
                            s.push_str(&format!(" {a}:{n}"));
                        }
                        match b.stroke.filter(|_| !gp3) {
                            Some(Stroke::Down) => s.push_str(" down"),
                            Some(Stroke::Up) => s.push_str(" up"),
                            None => {}
                        }
                        if b.notes.is_empty() {
                            s.push_str(" rest");
                        }
                        let mut sorted: Vec<_> = b.notes.iter().collect();
                        // Guitar Pro stores the notes of a beat in string order.
                        sorted.sort_by_key(|n| n.position().string);
                        let notes: Vec<String> = sorted
                            .into_iter()
                            .map(|n| {
                                let p = n.position();
                                let e = n.effects;
                                let mut x = format!("{}/{}", p.string.get(), p.fret.0);
                                let ex = export.is_some();
                                for (on, tag) in [
                                    (n.tie, "tie"),
                                    (e.dead && !(ex && n.tie), "dead"),
                                    (e.accent && !gp3, "acc"),
                                    (e.palm_mute && !gp3, "pm"),
                                    (e.let_ring, "ring"),
                                    (e.string_percussive && !ex, "perc"),
                                ] {
                                    if on {
                                        x.push(' ');
                                        x.push_str(tag);
                                    }
                                }
                                x
                            })
                            .collect();
                        if !notes.is_empty() {
                            s.push_str(&format!(" ({})", notes.join(", ")));
                        }
                        s
                    })
                    .collect();
                out.push(format!(
                    "  bar {} voice {}: {}",
                    m + 1,
                    vi + 1,
                    beats.join(" | ")
                ));
            }
        }
    }
    out.join("\n")
}
