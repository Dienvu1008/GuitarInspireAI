//! Gesture names (PT-08). Names are compared without case (Unicode lower case), may
//! not equal a DSL keyword, and use only letters and digits, starting with a letter,
//! so the text DSL (6a-4) can always print and re-read them.

use crate::error::NameProblem;

/// The reserved DSL keywords, in lower case. Provisional until the 6a-4 grammar: it
/// holds the words of the confirmed corpus sketch (selectors, lanes, strokes, the
/// palm-mute and string-percussive marks) and the declarations of ADR-034 section 9.
/// The grammar may only use words from this list, so a name accepted now is never
/// taken away by the DSL later.
pub const KEYWORDS: &[&str] = &[
    "bass", "alt", "treble", "strum", "thumb", "fingers", "d", "u", "pm", "sp", "unit", "meter",
];

/// The key two names are compared by: Unicode lower case. No normalization is done
/// (no dependency allows it), so a composed and a decomposed spelling differ; a
/// decomposed spelling is rejected anyway, because combining marks are not letters.
pub fn name_key(name: &str) -> String {
    name.to_lowercase()
}

/// Checks one name on its own; uniqueness is checked by the library.
pub fn check_name(name: &str) -> Result<(), NameProblem> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(NameProblem::Empty);
    };
    if !first.is_alphabetic() {
        return Err(NameProblem::MustStartWithLetter);
    }
    if let Some(bad) = chars.find(|c| !c.is_alphanumeric()) {
        return Err(NameProblem::InvalidCharacter(bad));
    }
    let key = name_key(name);
    if let Some(k) = KEYWORDS.iter().find(|k| **k == key) {
        return Err(NameProblem::Keyword(k));
    }
    Ok(())
}
