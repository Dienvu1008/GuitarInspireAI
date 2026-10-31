//! Chord symbol notation: parsing (with aliases) and canonical ASCII display.
//! Aliases are only spellings of the same `ChordQuality`; they are not separate concepts.

use crate::{Chord, ChordQuality, NoteName, TheoryError};

/// Canonical ASCII suffix (Phase 2 decision): "", m, dim, aug, 7, maj7, m7, dim7, m7b5,
/// sus2, sus4, 6, m6, add9, 9.
pub fn suffix(q: ChordQuality) -> &'static str {
    use ChordQuality::*;
    match q {
        Major => "",
        Minor => "m",
        Diminished => "dim",
        Augmented => "aug",
        Dominant7 => "7",
        Major7 => "maj7",
        Minor7 => "m7",
        Diminished7 => "dim7",
        HalfDiminished7 => "m7b5",
        Sus2 => "sus2",
        Sus4 => "sus4",
        Sixth => "6",
        Minor6 => "m6",
        Add9 => "add9",
        Dominant9 => "9",
    }
}

/// Alias spellings mapped to canonical suffixes. Longest aliases first.
const ALIASES: &[(&str, &str)] = &[
    ("°7", "dim7"),
    ("ø7", "m7b5"),
    ("Δ7", "maj7"),
    ("min7", "m7"),
    ("min6", "m6"),
    ("min", "m"),
    ("°", "dim"),
    ("+", "aug"),
];

/// Canonical display, for example `Am7`, `C/E`, `F#m7b5`.
pub fn format(chord: &Chord) -> String {
    let mut s = format!("{}{}", chord.root, suffix(chord.quality));
    if let Some(b) = chord.bass {
        s.push('/');
        s.push_str(&b.to_string());
    }
    s
}

fn quality_from_suffix(s: &str) -> Option<ChordQuality> {
    let canonical = ALIASES
        .iter()
        .find(|(a, _)| *a == s)
        .map(|(_, c)| *c)
        .unwrap_or(s);
    ChordQuality::ALL
        .into_iter()
        .find(|q| suffix(*q) == canonical)
}

/// Parses `C`, `Am7`, `F#m7b5`, `Bb/D`, `CΔ7`, `Bø7`, `C°`, `C°7`, `Amin`, `C+`.
/// A slash bass equal to the root is dropped (C/C is C).
pub fn parse(text: &str) -> Result<Chord, TheoryError> {
    let bad = || TheoryError::InvalidChordSymbol(text.to_string());
    let t = text.trim();
    let (main, bass) = match t.split_once('/') {
        Some((m, b)) => (m, Some(b.parse::<NoteName>().map_err(|_| bad())?)),
        None => (t, None),
    };
    // Root: a letter plus an optional accidental (bb, b, ##, #). Longest match first.
    let mut chars = main.char_indices();
    let first = chars.next().ok_or_else(bad)?;
    if !first.1.is_ascii_alphabetic() {
        return Err(bad());
    }
    let after_letter = first.1.len_utf8();
    let rest = &main[after_letter..];
    let acc_len = ["##", "bb", "#", "b"]
        .iter()
        .find(|a| rest.starts_with(**a))
        .map_or(0, |a| a.len());
    // Greedy matching is safe: no quality suffix starts with '#' or 'b'.
    let root: NoteName = main[..after_letter + acc_len].parse().map_err(|_| bad())?;
    let quality = quality_from_suffix(&main[after_letter + acc_len..]).ok_or_else(bad)?;
    let bass = bass.filter(|b| *b != root);
    Ok(Chord {
        root,
        quality,
        bass,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canon(s: &str) -> String {
        format(&parse(s).unwrap())
    }

    #[test]
    fn canonical_and_aliases() {
        for s in [
            "C", "Am", "Bdim", "Caug", "G7", "Fmaj7", "Dm7", "Bdim7", "Bm7b5", "Dsus2", "Asus4",
            "C6", "Am6", "Cadd9", "G9", "Bb/D", "Bbm7b5", "Cb", "Ebbdim", "F##",
        ] {
            assert_eq!(canon(s), s);
        }
        assert_eq!(canon("CΔ7"), "Cmaj7");
        assert_eq!(canon("Bø7"), "Bm7b5");
        assert_eq!(canon("C°"), "Cdim");
        assert_eq!(canon("C°7"), "Cdim7");
        assert_eq!(canon("Amin"), "Am");
        assert_eq!(canon("Amin7"), "Am7");
        assert_eq!(canon("C+"), "Caug");
        assert_eq!(canon("C/C"), "C");
    }

    #[test]
    fn rejects_unknown_and_deferred_symbols() {
        for s in ["", "H", "Cmaj9", "C7b9", "C13", "Cm9", "C11", "C/H", "7"] {
            assert!(parse(s).is_err(), "{s}");
        }
    }
}
