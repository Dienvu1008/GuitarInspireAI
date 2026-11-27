//! Semantic layer of the notation staff (ADR-040 decision 2): per note, its written
//! spelled pitch, the accidental shown and its tie role. No font, no geometry.
//!
//! Every musical decision is a call into `mc-theory`:
//! - sounding pitch: `fretted::pitch_at` (capo included once, ADR-039);
//! - spelling: `KeySignature::spell`, and `notation::tie_continuation` for ties;
//! - written pitch: the instrument's `written_transposition`;
//! - accidentals: `notation::bar_accidentals` over all voices of the bar.
//!
//! This module only gathers inputs from the score and records the results. A source
//! scan test keeps spelling and accidental rules out of this crate (ADR-041 L6).

use crate::NotationError;
use mc_score::{MasterBar, Score, Track};
use mc_theory::fretted::pitch_at;
use mc_theory::notation::{BarNote, TieRole, bar_accidentals, tie_continuation};
use mc_theory::{Alter, NoteValue, Rational, SpelledPitch};

/// One note of a notated beat.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NotatedNote {
    pub string: u8,
    /// Sounding spelled pitch (what a tie continuing this note keeps).
    pub sounding: SpelledPitch,
    pub written: SpelledPitch,
    pub accidental: Option<Alter>,
    pub tie: TieRole,
    /// A tie leaves this note into the next bar (the voice's last beat only, EP5).
    pub ties_out: bool,
}

/// One beat of one voice.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NotatedBeat {
    pub voice: usize,
    pub beat: usize,
    pub onset: Rational,
    pub value: NoteValue,
    pub notes: Vec<NotatedNote>,
}

/// The notated content of one track in one bar, in voice order then beat order.
pub(crate) fn notate_bar(
    score: &Score,
    track: &Track,
    bar: usize,
) -> Result<Vec<NotatedBeat>, NotationError> {
    let master = &score.master_bars()[bar];
    let content = track.bar(bar).expect("H-02: bars aligned");
    let inst = track.fretted();
    let transposition = track.instrument().written_transposition();
    let leaving = leaving_ties(track, bar);
    let mut beats = Vec::new();
    for (v, voice) in content.voices.iter().enumerate() {
        let mut onset = Rational::ZERO;
        for (k, beat) in voice.beats.iter().enumerate() {
            let mut notes = Vec::with_capacity(beat.notes.len());
            for n in &beat.notes {
                let pos = n.position();
                let semitone =
                    pitch_at(&inst.tuning, inst.capo, pos).ok_or(NotationError::Overflow)?;
                let tie = match (n.tie, k) {
                    (false, _) => TieRole::NotTied,
                    (true, 0) => TieRole::TiedAcrossBarLine,
                    (true, _) => TieRole::TiedInBar,
                };
                let sounding = match tie {
                    TieRole::NotTied => master.key.spell(semitone),
                    _ => match tie_source(score, track, bar, v, k, pos.string.get()) {
                        Some(source) => tie_continuation(master.key, source, semitone),
                        None => master.key.spell(semitone),
                    },
                };
                let written = transposition
                    .written(sounding)
                    .map_err(|_| NotationError::Overflow)?;
                let ties_out =
                    k + 1 == voice.beats.len() && leaving.contains(&(v, pos.string.get()));
                notes.push(NotatedNote {
                    string: pos.string.get(),
                    sounding,
                    written,
                    accidental: None,
                    tie,
                    ties_out,
                });
            }
            beats.push(NotatedBeat {
                voice: v,
                beat: k,
                onset,
                value: beat.value,
                notes,
            });
            onset = onset
                .checked_add(beat.value.length())
                .ok_or(NotationError::Overflow)?;
        }
    }
    // Accidentals: one call for every note of the bar on this staff.
    let input: Vec<BarNote> = beats
        .iter()
        .flat_map(|b| {
            b.notes.iter().map(move |n| BarNote {
                onset: b.onset,
                written: n.written,
                tie: n.tie,
            })
        })
        .collect();
    let shown = bar_accidentals(master.key, &input);
    let mut i = 0;
    for b in &mut beats {
        for n in &mut b.notes {
            n.accidental = shown[i];
            i += 1;
        }
    }
    Ok(beats)
}

/// The sounding spelling of the note a tie on (`bar`, `voice`, `beat`, `string`)
/// continues: the same string in the previous beat of the voice (H-06), following the
/// chain back through earlier bars.
fn tie_source(
    score: &Score,
    track: &Track,
    bar: usize,
    voice: usize,
    beat: usize,
    string: u8,
) -> Option<SpelledPitch> {
    let (pb, pk) = if beat > 0 {
        (bar, beat - 1)
    } else {
        let pb = bar.checked_sub(1)?;
        let len = track.bar(pb)?.voices.get(voice)?.beats.len();
        (pb, len.checked_sub(1)?)
    };
    let prev = track.bar(pb)?.voices.get(voice)?.beats.get(pk)?;
    let note = prev
        .notes
        .iter()
        .find(|n| n.position().string.get() == string)?;
    let inst = track.fretted();
    let semitone = pitch_at(&inst.tuning, inst.capo, note.position())?;
    let key = score.master_bars()[pb].key;
    if note.tie
        && let Some(source) = tie_source(score, track, pb, voice, pk, string)
    {
        return Some(tie_continuation(key, source, semitone));
    }
    Some(key.spell(semitone))
}

/// Sounding spellings of the ties that enter `bar` from the previous bar, per voice
/// and string (ADR-041 decision 3: part of the bar-plan cache key). Empty when no tie
/// crosses the bar line.
pub(crate) fn entering_ties(
    score: &Score,
    track: &Track,
    bar: usize,
) -> Vec<(usize, u8, SpelledPitch)> {
    let Some(content) = track.bar(bar) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (v, voice) in content.voices.iter().enumerate() {
        let Some(first) = voice.beats.first() else {
            continue;
        };
        for n in first.notes.iter().filter(|n| n.tie) {
            let s = n.position().string.get();
            if let Some(p) = tie_source(score, track, bar, v, 0, s) {
                out.push((v, s, p));
            }
        }
    }
    out
}

/// The ties that leave `bar` into the next bar, per voice and string: the next bar's
/// first beat of the voice has a tied note on that string (H-06). Part of the
/// bar-plan cache key, because a leaving tie adds reach to the bar (EP5).
pub(crate) fn leaving_ties(track: &Track, bar: usize) -> Vec<(usize, u8)> {
    let Some(next) = track.bar(bar + 1) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (v, voice) in next.voices.iter().enumerate() {
        if let Some(first) = voice.beats.first() {
            for n in first.notes.iter().filter(|n| n.tie) {
                out.push((v, n.position().string.get()));
            }
        }
    }
    out
}

/// Whether the key signature changes at `bar` (shown at the bar, or by the header when
/// the bar starts a system).
pub(crate) fn key_changes(master: &MasterBar, previous: Option<&MasterBar>) -> bool {
    previous.is_some_and(|p| p.key != master.key)
}
