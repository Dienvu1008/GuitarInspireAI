use mc_score::TrackId;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotationError {
    UnknownTrack(TrackId),
    InvalidMetrics(String),
    /// A bar duration does not fit the rational representation.
    Overflow,
    /// `layout_score` was asked for no track (ADR-031: at least the active one).
    NoTracks,
}

impl fmt::Display for NotationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTrack(t) => write!(f, "track {} does not exist", t.raw()),
            Self::InvalidMetrics(m) => write!(f, "invalid layout metrics: {m}"),
            Self::Overflow => write!(f, "bar duration overflow"),
            Self::NoTracks => write!(f, "no track to lay out"),
        }
    }
}

impl std::error::Error for NotationError {}
