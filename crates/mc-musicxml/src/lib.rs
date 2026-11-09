//! MusicXML export (ADR-033). Writes the written score, not the performance: one part
//! per track with a TAB staff, one measure per master bar, repeats and endings as
//! written, voices, ties, tuplets, dead notes and let ring. Every approximation is
//! counted in a [`MusicXmlReport`]; no note is ever dropped or cut.
//!
//! The format is MusicXML 4.0 partwise, uncompressed (`.musicxml`). The adapter owns no
//! music rule (GR-05): sounding pitch comes from `mc_theory::fretted::pitch_at` and its
//! spelling from `KeySignature::spell`.

mod export;
mod xml;

pub use export::export_musicxml;

/// What the export wrote and what it had to approximate (ADR-012 `ExportReport`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MusicXmlReport {
    /// Parts written, one per track.
    pub parts: usize,
    /// Measures per part, one per master bar.
    pub measures: usize,
    /// Notes written. Always equal to the score's note count: nothing is dropped.
    pub notes: usize,
    /// Non-empty voices shorter than their measure, padded with a rest (D-01 underfull).
    pub padded_voices: usize,
    /// Measures (per part) whose content is longer than the time signature. All notes
    /// are kept (D-01 overfull); a strict reader may mark the measure as irregular.
    pub overfull_measures: usize,
    /// Tuplet beats written without a bracket because they do not form a whole group.
    /// Their duration and ratio are still exact.
    pub tuplets_without_bracket: usize,
    /// Tempo values that are not a short decimal and were rounded to 3 decimals.
    pub tempo_rounded: usize,
    /// Characters removed from names and markers because XML 1.0 forbids them.
    pub characters_removed: usize,
    /// Notes whose technique has no MusicXML 4.0 element and is written as
    /// `<other-technical>` text: palm mute and string-percussive (ADR-035). A note with
    /// both counts twice.
    pub techniques_approximated: usize,
}

/// Why a score could not be written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MusicXmlError {
    /// The rhythm needs more divisions per quarter note, or a longer duration, than a
    /// 32-bit integer holds (readers parse these as `int`).
    DurationOverflow,
    /// A note's string is not part of its track's tuning (cannot happen in a valid score).
    NoPitch { track: usize, bar: usize },
}

impl std::fmt::Display for MusicXmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MusicXmlError::DurationOverflow => {
                write!(
                    f,
                    "the rhythm is too fine or too long for MusicXML durations"
                )
            }
            MusicXmlError::NoPitch { track, bar } => {
                write!(
                    f,
                    "a note in track {} bar {} has no pitch",
                    track + 1,
                    bar + 1
                )
            }
        }
    }
}

impl std::error::Error for MusicXmlError {}
