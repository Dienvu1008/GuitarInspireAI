//! Guitar queries for adapters: chord lookup, voicing identification, scale and note
//! positions. They read no document state, so they are plain functions. All musical
//! decisions are made by `mc-guitar` and `mc-theory`; this module only parses adapter
//! input into value objects.

use mc_theory::fretted::Capo;
use mc_theory::{NoteName, ScaleFormula, SpelledPitch, TheoryError, chord_symbol};
use std::fmt;
use std::ops::RangeInclusive;

pub use mc_guitar::{
    BassRule, CandidateTone, ChordCandidate, ConstraintViolation, GuitarError, Identification,
    IdentifyOptions, IdentifyStatus, MIN_DISTINCT_PITCH_CLASSES, RetuneMode, ScaleNote,
    SearchOptions, SoundingNote, TuningPreset, Voicing, VoicingConstraints, interval_name,
};
pub use mc_theory::fretted::Tuning;
pub use mc_theory::{Chord, ChordInterval, ChordQuality};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    Theory(TheoryError),
    Guitar(GuitarError),
    UnknownScale(String),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::Theory(e) => write!(f, "{e}"),
            QueryError::Guitar(e) => write!(f, "{e}"),
            QueryError::UnknownScale(s) => write!(
                f,
                "unknown scale '{s}' (known: {})",
                ScaleFormula::ALL.map(|s| s.name()).join(", ")
            ),
        }
    }
}

impl std::error::Error for QueryError {}

impl From<TheoryError> for QueryError {
    fn from(e: TheoryError) -> Self {
        QueryError::Theory(e)
    }
}

impl From<GuitarError> for QueryError {
    fn from(e: GuitarError) -> Self {
        QueryError::Guitar(e)
    }
}

/// A preset name (`standard`, `drop-d`, ...) or comma-separated open-string pitches,
/// string 1 first (`E4,B3,G3,D3,A2,D2`).
pub fn parse_tuning(text: &str) -> Result<Tuning, QueryError> {
    if let Ok(p) = TuningPreset::from_name(text) {
        return Ok(p.tuning());
    }
    if !text.contains(',') {
        return Err(GuitarError::UnknownPreset(text.to_string()).into());
    }
    let strings = text
        .split(',')
        .map(|p| p.trim().parse::<SpelledPitch>())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Tuning::new(strings)?)
}

/// Parses a chord symbol (aliases accepted).
pub fn parse_chord(symbol: &str) -> Result<Chord, QueryError> {
    Ok(chord_symbol::parse(symbol)?)
}

/// Canonical chord symbol.
pub fn format_chord(chord: &Chord) -> String {
    chord_symbol::format(chord)
}

/// Spelled chord tones from the root, for example `C E G Bb`.
pub fn chord_tones(chord: &Chord) -> Vec<String> {
    chord
        .spelled_tones()
        .unwrap_or_default()
        .iter()
        .map(NoteName::to_string)
        .collect()
}

/// Voicings of a chord symbol, sorted as the Guitar context orders them.
pub fn chord_voicings(
    symbol: &str,
    tuning: &Tuning,
    capo: u8,
    options: &SearchOptions,
) -> Result<(Chord, Vec<Voicing>), QueryError> {
    let chord = parse_chord(symbol)?;
    let found = mc_guitar::find_voicings(&chord, tuning, Capo(capo), options);
    Ok((chord, found))
}

/// A chord-finder result for an adapter (ADR-029 T6): the canonical symbol, the spelled
/// tones, and the voicings for one instrument. Plain data; the UI renders it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordLookup {
    pub symbol: String,
    pub tones: Vec<String>,
    pub voicings: Vec<Voicing>,
}

/// Voicings of `symbol` for a track's `instrument`, using its tuning and capo, with the
/// default constraints and at most `max` results (ADR-029 T6/T8). Advanced constraints
/// are deferred; the default `SearchOptions` are used.
pub fn track_chord_voicings(
    instrument: &mc_score::FrettedInstrument,
    symbol: &str,
    max: usize,
) -> Result<ChordLookup, QueryError> {
    let options = SearchOptions {
        max_results: max,
        ..SearchOptions::default()
    };
    let (chord, voicings) =
        chord_voicings(symbol, &instrument.tuning, instrument.capo.0, &options)?;
    Ok(ChordLookup {
        symbol: format_chord(&chord),
        tones: chord_tones(&chord),
        voicings,
    })
}

/// Ranked chord names for a shape such as `x32010`.
pub fn identify_shape(
    shape: &str,
    tuning: &Tuning,
    capo: u8,
    options: IdentifyOptions,
) -> Result<Identification, QueryError> {
    let voicing: Voicing = shape.parse()?;
    Ok(mc_guitar::identify(&voicing, tuning, Capo(capo), options)?)
}

/// Scale notes in a fret window, string 1 first.
pub fn scale_positions(
    root: &str,
    scale: &str,
    tuning: &Tuning,
    capo: u8,
    frets: RangeInclusive<u8>,
) -> Result<Vec<ScaleNote>, QueryError> {
    let root: NoteName = root.parse()?;
    let formula =
        ScaleFormula::from_name(scale).ok_or_else(|| QueryError::UnknownScale(scale.into()))?;
    Ok(mc_guitar::scale_positions(
        tuning,
        Capo(capo),
        root.pitch_class(),
        formula,
        frets,
    ))
}

/// Names of the known scales.
pub fn scale_names() -> Vec<&'static str> {
    ScaleFormula::ALL.iter().map(|s| s.name()).collect()
}

/// Note name in the canonical no-context spelling for a sounding semitone (display only).
pub fn note_name(semitone: i32) -> String {
    NoteName::default_for(mc_theory::PitchClass::from_semitone(semitone)).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_parse_adapter_input() {
        let t = parse_tuning("standard").unwrap();
        assert_eq!(parse_tuning("E4,B3,G3,D3,A2,E2").unwrap(), t);
        assert!(parse_tuning("nashville").is_err());
        let (chord, v) = chord_voicings("C", &t, 0, &SearchOptions::default()).unwrap();
        assert_eq!(format_chord(&chord), "C");
        assert_eq!(v[0].to_string(), "x32010");
        let id = identify_shape("x02010", &t, 0, IdentifyOptions::default()).unwrap();
        assert_eq!(id.preferred().unwrap().symbol, "Am7");
        assert!(matches!(
            scale_positions("A", "dorian", &t, 0, 0..=5),
            Err(QueryError::UnknownScale(_))
        ));
        assert_eq!(
            chord_tones(&parse_chord("C7").unwrap()),
            ["C", "E", "G", "Bb"]
        );
    }
}
