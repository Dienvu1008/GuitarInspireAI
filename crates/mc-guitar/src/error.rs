use std::fmt;

/// Errors of the Guitar context. Each variant names the rule it protects where one applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuitarError {
    /// Unparseable shape text such as "x32010".
    InvalidShape(String),
    /// G-01: two positions on the same string.
    DuplicateString(u8),
    /// A position names a string the instrument does not have.
    StringOutOfRange { string: u8, strings: u8 },
    /// The voicing and the tuning have different string counts.
    StringCountMismatch { voicing: u8, tuning: u8 },
    /// Unknown tuning preset name.
    UnknownPreset(String),
    /// Keep-pitches retuning needs the same number of strings before and after.
    RetuneStringCount { old: u8, new: u8 },
}

impl fmt::Display for GuitarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShape(s) => write!(f, "invalid shape '{s}'"),
            Self::DuplicateString(s) => {
                write!(f, "string {s} has more than one position (G-01)")
            }
            Self::StringOutOfRange { string, strings } => {
                write!(
                    f,
                    "string {string} does not exist on a {strings}-string instrument"
                )
            }
            Self::StringCountMismatch { voicing, tuning } => write!(
                f,
                "voicing has {voicing} strings but the tuning has {tuning}"
            ),
            Self::UnknownPreset(s) => write!(f, "unknown tuning preset '{s}'"),
            Self::RetuneStringCount { old, new } => write!(
                f,
                "cannot keep pitches when the string count changes ({old} -> {new})"
            ),
        }
    }
}

impl std::error::Error for GuitarError {}
