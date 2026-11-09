//! Notation queries for adapters: TAB layout in the current document state (ADR-018,
//! ADR-031). Layout is computed by `mc-notation`; this module routes the query and
//! keeps the multi-track outline of the current revision.

use crate::{DocumentSession, Revision};
use mc_score::TrackId;
use std::ops::Range;

pub use mc_notation::{
    BarLayout, Glyph, GlyphKind, HitRegion, HitTarget, LayoutCache, LayoutMetrics, Line, LineKind,
    NotationError, OutlineSystem, Rect, ScoreLayout, ScoreOutline, ScoreSystem, Stave,
    SystemLayout, TabLayout,
};

/// The outline of one revision, tracks and metrics (a view cache, not document state).
#[derive(Debug)]
pub(crate) struct OutlineCache {
    revision: Revision,
    tracks: Vec<TrackId>,
    metrics: LayoutMetrics,
    outline: ScoreOutline,
}

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

    /// Layout of the shown `tracks` together, one stave per track per system, with bar
    /// lines and simultaneous beats lined up (ADR-031). Which tracks are shown is UI
    /// state the adapter passes in; it is not stored in the document. Lays out every
    /// system; views of large scores use [`Self::score_outline`] and
    /// [`Self::score_systems`] instead.
    pub fn score_layout(
        &self,
        tracks: &[TrackId],
        metrics: &LayoutMetrics,
    ) -> Result<ScoreLayout, NotationError> {
        mc_notation::layout_score(self.score(), tracks, metrics)
    }

    /// Line breaking of the shown `tracks` at the current revision: every system's
    /// position, height and bars, without content (ADR-031 fallback 2). Computed once
    /// per (revision, tracks, metrics) and kept, so [`Self::score_systems`] for the
    /// systems in view does not break lines again and matches this outline.
    pub fn score_outline(
        &mut self,
        tracks: &[TrackId],
        metrics: &LayoutMetrics,
    ) -> Result<&ScoreOutline, NotationError> {
        let fresh = self.outline_cache.as_ref().is_some_and(|c| {
            c.revision == self.revision() && c.tracks == tracks && c.metrics == *metrics
        });
        if !fresh {
            let (score, widths) = self.score_and_width_cache();
            let outline = mc_notation::outline_score_cached(score, tracks, metrics, widths)?;
            self.outline_cache = Some(OutlineCache {
                revision: self.revision(),
                tracks: tracks.to_vec(),
                metrics: *metrics,
                outline,
            });
        }
        Ok(&self.outline_cache.as_ref().expect("set above").outline)
    }

    /// The systems `range` of the current outline (computed if needed), laid out
    /// exactly as [`Self::score_layout`] would lay them out. Only their bars are
    /// measured; the rest of the score is not laid out.
    pub fn score_systems(
        &mut self,
        tracks: &[TrackId],
        metrics: &LayoutMetrics,
        range: Range<usize>,
    ) -> Result<Vec<ScoreSystem>, NotationError> {
        self.score_outline(tracks, metrics)?;
        let outline = &self.outline_cache.as_ref().expect("set above").outline;
        mc_notation::layout_score_systems(self.score(), outline, range, metrics)
    }
}
