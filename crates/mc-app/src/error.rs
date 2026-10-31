use crate::{Revision, ports::StoreError};
use mc_guitar::GuitarError;
use mc_project::LoadError;
use mc_score::DomainError;
use mc_theory::TheoryError;
use std::fmt;

/// Why a command was rejected. The document is unchanged in every case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// E-01: the caller's view of the document is out of date.
    StaleRevision {
        expected: Revision,
        actual: Revision,
    },
    /// A value could not be constructed (for example an invalid duration).
    Value(TheoryError),
    /// The domain rejected the operation (hard invariant or editing constraint).
    Domain(DomainError),
    /// A configurable constraint (T-06, T-07, C-01, C-02) rejected user input.
    Constraint {
        rule: &'static str,
        message: String,
    },
    /// A Guitar context service rejected the input (for example a retune plan).
    Guitar(GuitarError),
    NothingToUndo,
    NothingToRedo,
}

impl CommandError {
    pub fn rule(&self) -> &'static str {
        match self {
            CommandError::StaleRevision { .. } => "E-01",
            CommandError::Value(_) => "T",
            CommandError::Domain(d) => d.rule(),
            CommandError::Constraint { rule, .. } => rule,
            CommandError::Guitar(_) => "G",
            CommandError::NothingToUndo | CommandError::NothingToRedo => "E-05",
        }
    }
}

impl From<DomainError> for CommandError {
    fn from(e: DomainError) -> Self {
        CommandError::Domain(e)
    }
}

impl From<GuitarError> for CommandError {
    fn from(e: GuitarError) -> Self {
        CommandError::Guitar(e)
    }
}

impl From<TheoryError> for CommandError {
    fn from(e: TheoryError) -> Self {
        CommandError::Value(e)
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::StaleRevision { expected, actual } => {
                write!(
                    f,
                    "E-01: document changed (expected revision {}, current {})",
                    expected.0, actual.0
                )
            }
            CommandError::Value(e) => write!(f, "{e}"),
            CommandError::Domain(e) => write!(f, "{e}"),
            CommandError::Constraint { rule, message } => write!(f, "{rule}: {message}"),
            CommandError::Guitar(e) => write!(f, "{e}"),
            CommandError::NothingToUndo => write!(f, "nothing to undo"),
            CommandError::NothingToRedo => write!(f, "nothing to redo"),
        }
    }
}

impl std::error::Error for CommandError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    Store(StoreError),
    Load(LoadError),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Store(e) => write!(f, "{e}"),
            OpenError::Load(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for OpenError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// U2: the document contains content this build cannot write back; overwriting
    /// the original would lose it. Save As to another location is allowed.
    WouldDropUnknownContent {
        paths: Vec<String>,
    },
    /// A new document has no location yet; use Save As.
    NoLocation,
    Store(StoreError),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::WouldDropUnknownContent { paths } => write!(
                f,
                "this project contains {} item(s) written by a newer version that cannot be saved back; use Save As to keep the original",
                paths.len()
            ),
            SaveError::NoLocation => {
                write!(f, "the document has not been saved yet; choose a location")
            }
            SaveError::Store(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SaveError {}
