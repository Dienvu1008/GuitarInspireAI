//! Notation queries for adapters: TAB layout of a track in the current document state
//! (ADR-018). Layout is computed by `mc-notation`; this module only routes the query.

use crate::DocumentSession;
use mc_score::TrackId;

pub use mc_notation::{
    BarLayout, Glyph, GlyphKind, HitRegion, HitTarget, LayoutMetrics, Line, LineKind,
    NotationError, Rect, SystemLayout, TabLayout,
};

impl DocumentSession {
    /// TAB layout of `track` at the current revision. Adapters cache it per
    /// (track, revision, metrics) and recompute after a document event.
    pub fn tab_layout(
        &self,
        track: TrackId,
        metrics: &LayoutMetrics,
    ) -> Result<TabLayout, NotationError> {
        mc_notation::layout_tab(self.score(), track, metrics)
    }
}
