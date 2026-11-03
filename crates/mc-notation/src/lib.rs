//! Notation layout (`mc-notation`): a projection of the Composition context.
//!
//! Computes TAB layout as a display list in abstract units (ADR-018): string and bar
//! lines, fret numbers, rhythm stems and beams, repeat and volta signs, and hit regions
//! for cursor placement. The UI supplies `LayoutMetrics` measured from the bundled font
//! (ADR-021) and only paints the result. Standard notation comes in Phase 6c.

mod display;
mod error;
mod layout;
mod metrics;

pub use display::{
    BarLayout, Glyph, GlyphKind, HitRegion, HitTarget, Line, LineKind, Rect, SystemLayout,
    TabLayout,
};
pub use error::NotationError;
pub use layout::layout_tab;
pub use metrics::LayoutMetrics;
