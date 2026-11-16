//! Staff model (ADR-038): which staves a shown track has. A view setting passed by the
//! adapter, like the list of shown tracks; never stored in the score.

use mc_score::TrackId;

/// The staves of one shown track. Notation is drawn above TAB (ADR-038 decision 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StaffConfiguration {
    /// Standard notation (written pitch from the instrument, ADR-039).
    pub notation: bool,
    pub tab: bool,
}

impl StaffConfiguration {
    /// TAB only: the layout before Phase 6c, unchanged (ADR-041 L1).
    pub const TAB: StaffConfiguration = StaffConfiguration {
        notation: false,
        tab: true,
    };
    /// Notation above TAB.
    pub const NOTATION_AND_TAB: StaffConfiguration = StaffConfiguration {
        notation: true,
        tab: true,
    };
}

/// A shown track and its staves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShownTrack {
    pub track: TrackId,
    pub staves: StaffConfiguration,
}

impl ShownTrack {
    pub const fn tab(track: TrackId) -> Self {
        ShownTrack {
            track,
            staves: StaffConfiguration::TAB,
        }
    }
}

/// The kind of a laid-out stave.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StaveKind {
    Tab,
    Notation,
}
