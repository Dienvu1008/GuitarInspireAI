//! Full validation of every hard invariant of the score model.

use crate::model::{Beat, FrettedInstrument, InstrumentSpec, Note, NoteKind, Track};
use crate::{Score, Violation, repeats};
use mc_theory::Rational;
use std::collections::HashSet;

pub(crate) fn validate(score: &Score) -> Vec<Violation> {
    let mut v = Vec::new();
    let mut push = |rule: &'static str, location: String, message: String| {
        v.push(Violation {
            rule,
            location,
            message,
        })
    };

    // H-09: at least one master bar and one track; each bar has at least one voice.
    if score.master_bars.is_empty() {
        push("H-09", "score".into(), "score has no master bars".into());
    }
    if score.tracks.is_empty() {
        push("H-09", "score".into(), "score has no tracks".into());
    }

    let mut ids = HashSet::new();
    for track in &score.tracks {
        let tid = track.id.raw();
        if !ids.insert(tid) {
            push(
                "H-01",
                format!("track {tid}"),
                format!("duplicate id {tid}"),
            );
        }
        // H-02: bars aligned with master bars (v1 model limit).
        if track.bars.len() != score.master_bars.len() {
            push(
                "H-02",
                format!("track {tid}"),
                format!(
                    "{} bars but {} master bars",
                    track.bars.len(),
                    score.master_bars.len()
                ),
            );
        }
        let InstrumentSpec::Fretted(inst) = &track.instrument;
        if let Err(m) = check_instrument(inst) {
            push("H-10", format!("track {tid}"), m);
        }
        for (b, bar) in track.bars.iter().enumerate() {
            if bar.voices.is_empty() {
                push(
                    "H-09",
                    format!("track {tid}, bar {b}"),
                    "bar has no voices".into(),
                );
            }
            for (vi, voice) in bar.voices.iter().enumerate() {
                for (k, beat) in voice.beats.iter().enumerate() {
                    let loc =
                        || format!("track {tid}, bar {b}, voice {vi}, beat {}", beat.id.raw());
                    if !ids.insert(beat.id.raw()) {
                        push("H-01", loc(), format!("duplicate id {}", beat.id.raw()));
                    }
                    let mut strings = HashSet::new();
                    for note in &beat.notes {
                        let nloc = || format!("{}, note {}", loc(), note.id.raw());
                        if !ids.insert(note.id.raw()) {
                            push("H-01", nloc(), format!("duplicate id {}", note.id.raw()));
                        }
                        if let Err((rule, m)) = check_note(inst, note) {
                            push(rule, nloc(), m);
                        }
                        let NoteKind::Fretted(p) = note.kind;
                        if !strings.insert(p.string) {
                            push(
                                "H-05",
                                nloc(),
                                format!("second note on string {} in one beat", p.string.get()),
                            );
                        }
                        if note.tie && !tie_is_valid(track, b, vi, k, note) {
                            push(
                                "H-06",
                                nloc(),
                                "tie without an equal preceding note on the same string".into(),
                            );
                        }
                    }
                }
            }
        }
    }

    // H-11: tempo map ordered, unique, starting at the beginning, offsets within the bar.
    let tm = &score.tempo_map;
    match tm.first() {
        None => push("H-11", "tempo map".into(), "tempo map is empty".into()),
        Some(first) if first.bar != 0 || !first.offset.is_zero() => push(
            "H-11",
            "tempo map".into(),
            "first tempo change is not at the start of the score".into(),
        ),
        _ => {}
    }
    for (i, t) in tm.iter().enumerate() {
        let loc = format!("tempo change {i}");
        if let Some(prev) = i.checked_sub(1).map(|p| &tm[p])
            && (prev.bar, prev.offset) >= (t.bar, t.offset)
        {
            push(
                "H-11",
                loc.clone(),
                "tempo changes are not strictly ordered by position".into(),
            );
        }
        match score.master_bars.get(t.bar) {
            None => push("H-11", loc, format!("bar {} does not exist", t.bar)),
            Some(mb) => {
                if !tempo_offset_valid(mb, t.offset) {
                    push(
                        "H-11",
                        loc,
                        format!("offset {} outside bar {}", t.offset, t.bar),
                    );
                }
            }
        }
    }

    // H-12: repeat structure.
    v.extend(repeats::check(&score.master_bars));
    v
}

/// Tempo offsets are within the nominal bar length (0 for pickup bars), so content
/// edits can never invalidate the tempo map.
pub(crate) fn tempo_offset_valid(mb: &crate::MasterBar, offset: Rational) -> bool {
    if offset.is_negative() {
        return false;
    }
    if mb.pickup {
        offset.is_zero()
    } else {
        offset < mb.time_signature.length()
    }
}

pub(crate) fn check_instrument(inst: &FrettedInstrument) -> Result<(), String> {
    if inst.fret_count == 0 {
        return Err("fret count must be at least 1".into());
    }
    if inst.capo.0 > inst.fret_count {
        return Err(format!(
            "capo {} exceeds fret count {}",
            inst.capo.0, inst.fret_count
        ));
    }
    Ok(())
}

/// H-03, H-04, H-10 (and H-08, which cannot fail while only fretted kinds exist).
pub(crate) fn check_note(
    inst: &FrettedInstrument,
    note: &Note,
) -> Result<(), (&'static str, String)> {
    let NoteKind::Fretted(p) = note.kind;
    if p.string.get() > inst.tuning.string_count() {
        return Err((
            "H-03",
            format!(
                "string {} but the track has {} strings",
                p.string.get(),
                inst.tuning.string_count()
            ),
        ));
    }
    if p.fret.0 > inst.fret_count {
        return Err((
            "H-04",
            format!("fret {} exceeds fret count {}", p.fret.0, inst.fret_count),
        ));
    }
    if p.fret.0 as u16 + inst.capo.0 as u16 > inst.fret_count as u16 {
        return Err((
            "H-10",
            format!(
                "capo {} + fret {} exceeds fret count {}",
                inst.capo.0, p.fret.0, inst.fret_count
            ),
        ));
    }
    Ok(())
}

/// Preceding beat in written order: previous beat of the same voice, or the last beat
/// of the same voice in the previous bar.
pub(crate) fn previous_beat(track: &Track, bar: usize, voice: usize, beat: usize) -> Option<&Beat> {
    if beat > 0 {
        return track.bars[bar]
            .voices
            .get(voice)
            .and_then(|v| v.beats.get(beat - 1));
    }
    bar.checked_sub(1)
        .and_then(|pb| track.bars.get(pb))
        .and_then(|b| b.voices.get(voice))
        .and_then(|v| v.beats.last())
}

/// H-06: the preceding beat has a note on the same string with the same fret.
pub(crate) fn tie_is_valid(
    track: &Track,
    bar: usize,
    voice: usize,
    beat: usize,
    note: &Note,
) -> bool {
    let p = note.position();
    previous_beat(track, bar, voice, beat)
        .is_some_and(|prev| prev.notes.iter().any(|n| n.position() == p))
}
