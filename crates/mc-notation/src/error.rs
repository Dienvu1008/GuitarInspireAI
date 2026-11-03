use mc_score::TrackId;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotationError {
    UnknownTrack(TrackId),
    InvalidMetrics(String),
    /// A bar duration does not fit the rational representation.
    Overflow,
}

impl fmt::Display for NotationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTrack(t) => write!(f, "track {} does not exist", t.raw()),
            Self::InvalidMetrics(m) => write!(f, "invalid layout metrics: {m}"),
            Self::Overflow => write!(f, "bar duration overflow"),
        }
    }
}

impl std::error::Error for NotationError {}
