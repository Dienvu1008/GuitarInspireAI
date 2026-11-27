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
    /// An outline was used with a score it was not computed for (bars or tracks
    /// changed); compute a new outline.
    StaleOutline,
    /// A shown track was given neither a notation nor a TAB staff (ADR-038).
    NoStaves(TrackId),
    /// A page setup with a negative or non-finite size, or no room for systems.
    InvalidPage(String),
    /// One stave of a system is taller than every page body (ADR-041 section 11):
    /// `stave` indexes the system's staves in emission order.
    PageTooSmall {
        system: usize,
        stave: usize,
    },
}

impl fmt::Display for NotationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTrack(t) => write!(f, "track {} does not exist", t.raw()),
            Self::InvalidMetrics(m) => write!(f, "invalid layout metrics: {m}"),
            Self::Overflow => write!(f, "bar duration overflow"),
            Self::NoTracks => write!(f, "no track to lay out"),
            Self::StaleOutline => write!(f, "layout outline is out of date"),
            Self::NoStaves(t) => write!(f, "track {} is shown with no staff", t.raw()),
            Self::InvalidPage(m) => write!(f, "invalid page setup: {m}"),
            Self::PageTooSmall { system, stave } => write!(
                f,
                "stave {stave} of system {system} is taller than a page body"
            ),
        }
    }
}

impl std::error::Error for NotationError {}
