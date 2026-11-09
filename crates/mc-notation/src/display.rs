//! Display list types. Coordinates are abstract units; each system has its own
//! coordinate space with (0, 0) at its top-left corner, and `SystemLayout::y` places it
//! in the document. Flutter paints these items as given and makes no layout decisions.

use mc_score::{BeatId, TrackId};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineKind {
    /// A string of the TAB staff.
    String,
    BarLine,
    /// Thick line of a repeat sign or the final bar line.
    ThickBarLine,
    Stem,
    Beam,
    /// Bracket of an alternate ending (volta).
    Volta,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub kind: LineKind,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GlyphKind {
    /// A fret number on a string. Tied notes are shown in parentheses and dead notes
    /// as `x` (painting decides the exact symbol).
    Fret {
        fret: u8,
        tied: bool,
        dead: bool,
        let_ring: bool,
    },
    /// A rest of the given base value exponent (0 whole ... 8) and dots.
    Rest {
        exponent: u8,
        dots: u8,
    },
    /// Rhythm dot next to a stem.
    Dot,
    /// Tuplet number (for example 3) under or over a group.
    Tuplet {
        actual: u8,
    },
    TimeSignature {
        numerator: u8,
        denominator: u8,
    },
    /// The two dots of a repeat sign.
    RepeatDots,
    /// Play count of a repeated section, shown as "x3".
    RepeatCount {
        count: u8,
    },
    /// Numbers of an alternate ending, shown as "1., 2.".
    Volta {
        endings: Vec<u8>,
    },
    Marker {
        text: String,
    },
    /// 1-based bar number at the start of a system.
    BarNumber {
        number: usize,
    },
    TabClef,
    /// Open-string name in the system header (first system only).
    StringName {
        string: u8,
        name: String,
    },
}

/// A glyph centred at (`x`, `y`). `width` is the horizontal extent used for layout
/// (painting may clear the string line behind fret numbers over this width).
#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    pub kind: GlyphKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
}

/// What a click or the cursor addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HitTarget {
    /// An existing beat of a voice (on one string).
    Beat {
        voice: usize,
        beat: usize,
        beat_id: BeatId,
    },
    /// The insertion slot after the last beat of a voice (the whole bar when empty).
    Append { voice: usize },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitRegion {
    pub rect: Rect,
    pub bar: usize,
    /// 1-based string number (string 1 is the highest, Q-07).
    pub string: u8,
    pub target: HitTarget,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarLayout {
    pub bar: usize,
    /// Left edge of the bar in system coordinates.
    pub x: f32,
    pub width: f32,
    /// A voice of this track is underfull or overfull (D-01); painted as a warning.
    pub diagnostic: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SystemLayout {
    pub index: usize,
    /// Top of the system in document coordinates.
    pub y: f32,
    pub height: f32,
    /// Y of string 1 (top string line) in system coordinates.
    pub staff_top: f32,
    pub bars: Vec<BarLayout>,
    pub lines: Vec<Line>,
    pub glyphs: Vec<Glyph>,
    pub hits: Vec<HitRegion>,
}

impl SystemLayout {
    pub fn first_bar(&self) -> usize {
        self.bars.first().map_or(0, |b| b.bar)
    }

    /// One past the last bar in this system.
    pub fn end_bar(&self) -> usize {
        self.bars.last().map_or(0, |b| b.bar + 1)
    }

    /// The hit region at a point in system coordinates, preferring `voice`.
    pub fn hit_test(&self, x: f32, y: f32, voice: usize) -> Option<&HitRegion> {
        let voice_of = |h: &HitRegion| match h.target {
            HitTarget::Beat { voice, .. } | HitTarget::Append { voice } => voice,
        };
        let mut found = self.hits.iter().filter(|h| h.rect.contains(x, y));
        let first = found.clone().next();
        found.find(|h| voice_of(h) == voice).or(first)
    }
}

/// One track's stave inside a [`ScoreSystem`] (ADR-031). `layout` is in stave
/// coordinates: its lines, glyphs and hit regions start at the stave top, and its `y`
/// is the stave top in document coordinates. Bar x-positions and widths are identical
/// in every stave of the system.
#[derive(Clone, Debug, PartialEq)]
pub struct Stave {
    pub track: TrackId,
    pub strings: u8,
    /// Top of the stave in system coordinates.
    pub top: f32,
    pub layout: SystemLayout,
}

/// One line of music with a stave per shown track, stacked in score order.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreSystem {
    pub index: usize,
    /// Top of the system in document coordinates.
    pub y: f32,
    /// Sum of the stave heights.
    pub height: f32,
    pub staves: Vec<Stave>,
}

/// Where a system sits and which bars it holds, without its content (ADR-031).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutlineSystem {
    pub index: usize,
    /// Top of the system in document coordinates.
    pub y: f32,
    /// Sum of the stave heights.
    pub height: f32,
    pub first_bar: usize,
    /// One past the last bar.
    pub end_bar: usize,
    /// Justification factor of the system's flexible width (shared by its staves).
    pub scale: f32,
}

/// Line breaking of the shown tracks for the whole score: enough to size a scroll
/// view and to lay out any system later with `layout_score_systems`.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreOutline {
    /// The shown tracks in stave order (score order).
    pub tracks: Vec<TrackId>,
    pub width: f32,
    pub height: f32,
    /// Master bar count the outline was computed for (stale-outline check).
    pub bar_count: usize,
    pub systems: Vec<OutlineSystem>,
}

/// Layout of several tracks shown together: systems from top to bottom.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreLayout {
    /// The shown tracks in stave order (score order).
    pub tracks: Vec<TrackId>,
    pub width: f32,
    /// Total height of all systems including gaps.
    pub height: f32,
    pub systems: Vec<ScoreSystem>,
}

/// Layout of one track: systems from top to bottom.
#[derive(Clone, Debug, PartialEq)]
pub struct TabLayout {
    pub track: TrackId,
    pub strings: u8,
    pub width: f32,
    /// Total height of all systems including gaps.
    pub height: f32,
    pub systems: Vec<SystemLayout>,
}

impl TabLayout {
    /// Index of the system that contains `bar`.
    pub fn system_of_bar(&self, bar: usize) -> Option<usize> {
        self.systems
            .iter()
            .position(|s| (s.first_bar()..s.end_bar()).contains(&bar))
    }

    /// Systems that intersect the vertical document range `top..bottom`.
    pub fn visible(&self, top: f32, bottom: f32) -> impl Iterator<Item = &SystemLayout> {
        self.systems
            .iter()
            .filter(move |s| s.y < bottom && s.y + s.height > top)
    }

    /// Cursor rectangle (system index, rect in system coordinates) for a target.
    pub fn cursor_rect(&self, bar: usize, string: u8, target: HitTarget) -> Option<(usize, Rect)> {
        let s = self.system_of_bar(bar)?;
        self.systems[s]
            .hits
            .iter()
            .find(|h| h.bar == bar && h.string == string && h.target == target)
            .map(|h| (s, h.rect))
    }
}
