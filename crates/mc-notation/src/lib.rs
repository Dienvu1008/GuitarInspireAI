//! Notation layout (`mc-notation`): a projection of the Composition context.
//!
//! Computes TAB layout as a display list in abstract units (ADR-018): string and bar
//! lines, fret numbers, rhythm stems and beams, repeat and volta signs, and hit regions
//! for cursor placement. The UI supplies `LayoutMetrics` measured from the bundled font
//! (ADR-021) and only paints the result. Standard notation comes in Phase 6c.

#[cfg(test)]
mod cache_tests;
mod display;
mod engrave;
mod error;
mod layout;
mod metrics;
mod page;
mod semantic;
pub mod smufl;
mod staff;

pub use display::{
    AnchorFact, BarLayout, Beam, BeamFact, CourtesyFact, EngravedLine, EngravedLineKind, Glyph,
    GlyphKind, HitRegion, HitTarget, Line, LineKind, MusicGlyph, NotationFacts, NotationStaff,
    NoteFact, OutlineSystem, Rect, RestFact, ScoreLayout, ScoreOutline, ScoreSystem, Stave,
    StemFact, SystemLayout, TabLayout, Tie, TupletFact,
};
pub use error::NotationError;
pub use layout::{
    LayoutCache, layout_score, layout_score_systems, layout_score_with, layout_tab, outline_score,
    outline_score_cached, outline_score_cached_with, outline_score_with,
};
pub use metrics::{LayoutMetrics, TAB_STRING_GAP_SP};
pub use page::{
    FOOTER_HEIGHT_PT, PAGE_MARGIN_PT, PRINT_STAFF_SCALE, Page, PageItem, PageSetup, PageSize,
    Pagination, SPACIOUS_SYSTEM_GAP_SP, SystemSpacing, TITLE_HEIGHT_PT, paginate,
};
pub use staff::{ShownTrack, StaffConfiguration, StaveKind};
