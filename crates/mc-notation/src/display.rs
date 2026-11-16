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
    /// A fret number on a string. Tied notes are shown in parentheses; `dead` notes are
    /// drawn as `x`, which covers dead and string-percussive notes (ADR-035). Painting
    /// decides the exact symbol.
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
    /// Accent mark (`>`) above a beat with an accented note (ADR-035).
    Accent,
    /// Stroke arrow above a beat: down (`up` false) or up (ADR-035).
    Stroke {
        up: bool,
    },
    /// "S" above a beat with a string-percussive note (ADR-035).
    StringPercussive,
    /// "P.M." over palm-muted beats (ADR-035). `run` is true for a run of beats in
    /// which every sounding note is palm-muted: the label starts at `x - width / 2`
    /// and a dashed line extends over `width`. `run` is false for a beat where only
    /// some notes are palm-muted: the label only, on that beat.
    PalmMute {
        run: bool,
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
    /// Notation content (ADR-038); empty on a TAB stave, whose content is above.
    pub notation: NotationStaff,
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
    /// TAB or notation (ADR-038). A track with both has two staves, notation first.
    pub kind: crate::StaveKind,
    /// String count of the track's instrument (also on its notation stave).
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
    /// Width added to the header for the notation clef and key signature (0 with no
    /// notation staff).
    pub header_extra: f32,
}

/// Line breaking of the shown tracks for the whole score: enough to size a scroll
/// view and to lay out any system later with `layout_score_systems`.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreOutline {
    /// The shown tracks in stave order (score order).
    pub tracks: Vec<TrackId>,
    /// The staves of each shown track, in the order of `tracks`.
    pub staves: Vec<crate::StaffConfiguration>,
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

// ------------------------------------------------------------------ notation staff

/// Kinds of engraved lines on a notation staff.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EngravedLineKind {
    StaffLine,
    LedgerLine,
    Stem,
    BarLine,
    ThickBarLine,
}

/// A straight line with its thickness (layout units), centred on (x1, y1)-(x2, y2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EngravedLine {
    pub kind: EngravedLineKind,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub thickness: f32,
}

/// A SMuFL glyph with its origin at (`x`, `y`) in stave coordinates (y down). Painted
/// with the music font at a size of 4 staff spaces per em (SMuFL).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicGlyph {
    pub glyph: crate::smufl::SmuflGlyph,
    pub x: f32,
    pub y: f32,
}

/// A beam: a band of `thickness` (vertical) whose centre line runs from (x1, y1) to
/// (x2, y2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beam {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub thickness: f32,
}

/// A tie from `x1` to `x2` whose ends sit at `y`; it bulges up (`up`) or down by
/// `height`, with the end and middle thicknesses of the engraving defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tie {
    pub x1: f32,
    pub x2: f32,
    pub y: f32,
    pub up: bool,
    pub height: f32,
    pub end_thickness: f32,
    pub mid_thickness: f32,
}

/// Facts about one laid-out note (checks, later hit testing). Not painted.
#[derive(Clone, Debug, PartialEq)]
pub struct NoteFact {
    pub bar: usize,
    pub voice: usize,
    pub beat: usize,
    pub string: u8,
    pub written: mc_theory::SpelledPitch,
    /// Half staff spaces above the bottom line (E4 written = 0, top line = 8).
    pub position: i32,
    pub accidental: Option<mc_theory::Alter>,
    pub tie: mc_theory::notation::TieRole,
    pub head: Rect,
    /// The head sits on the other side of the stem (a second in a chord).
    pub displaced: bool,
    pub accidental_box: Option<Rect>,
    pub ledgers: Vec<EngravedLine>,
    /// Dot centres.
    pub dots: Vec<(f32, f32)>,
}

/// Facts about one stem.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StemFact {
    pub bar: usize,
    pub voice: usize,
    pub beat: usize,
    pub up: bool,
    pub x: f32,
    /// From the outermost head (away from the stem end) to the end, in staff spaces.
    pub length_sp: f32,
    pub beamed: bool,
}

/// A rest and its box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RestFact {
    pub bar: usize,
    pub voice: usize,
    pub beat: usize,
    pub rect: Rect,
}

/// A beam group: its beats and the box of its beams.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamFact {
    pub bar: usize,
    pub voice: usize,
    pub up: bool,
    pub beats: Vec<usize>,
    pub band: Rect,
}

/// A tuplet number.
#[derive(Clone, Debug, PartialEq)]
pub struct TupletFact {
    pub bar: usize,
    pub voice: usize,
    pub beats: Vec<usize>,
    pub number: u8,
    pub rect: Rect,
}

/// The x where a beat sits (notation and TAB share it, ADR-038 decision 4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchorFact {
    pub bar: usize,
    pub voice: usize,
    pub beat: usize,
    pub x: f32,
}

/// What the checks and hit testing need of a notation staff.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotationFacts {
    pub notes: Vec<NoteFact>,
    pub stems: Vec<StemFact>,
    pub rests: Vec<RestFact>,
    /// Whole-bar rests of bars without beats.
    pub bar_rests: Vec<RestFact>,
    pub beams: Vec<BeamFact>,
    pub tuplets: Vec<TupletFact>,
    pub anchors: Vec<AnchorFact>,
    /// x of every bar line drawn (in drawing order).
    pub barlines: Vec<f32>,
    /// Key-signature signs drawn (header and key changes).
    pub key_signs: usize,
}

/// The content of a notation stave, in stave coordinates (y down). Empty on TAB staves.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotationStaff {
    /// Layout units per staff space (`LayoutMetrics::staff_space`).
    pub staff_space: f32,
    /// y of the top staff line.
    pub lines_top: f32,
    pub lines: Vec<EngravedLine>,
    pub glyphs: Vec<MusicGlyph>,
    pub beams: Vec<Beam>,
    pub ties: Vec<Tie>,
    pub facts: NotationFacts,
}
