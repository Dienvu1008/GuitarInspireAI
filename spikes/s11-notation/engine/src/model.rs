//! The spike's immutable layout model (README section 4.3): systems of staves, each a
//! `Staff` with its `StaffPosition`, built from one `StaffConfiguration` per track.
//! Units: layout units; `Config::staff_space` units per staff space. Staff-local y
//! grows downwards from the staff's top.

use crate::pitch::Transposition;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StaffKind {
    Notation,
    Tab,
}

/// Which staves a track shows, and how (a view setting, not stored in the score).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StaffConfiguration {
    pub notation: bool,
    pub tab: bool,
    pub transposition: Transposition,
}

impl StaffConfiguration {
    pub const GUITAR: StaffConfiguration = StaffConfiguration {
        notation: true,
        tab: true,
        transposition: Transposition::GUITAR,
    };

    pub fn kinds(&self) -> Vec<StaffKind> {
        let mut k = Vec::new();
        if self.notation {
            k.push(StaffKind::Notation);
        }
        if self.tab {
            k.push(StaffKind::Tab);
        }
        k
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Layout units per staff space.
    pub staff_space: f32,
    pub width: f32,
    pub staves: StaffConfiguration,
}

/// Where a staff sits in its system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StaffPosition {
    pub track: usize,
    /// Index of the staff within its track (notation 0, TAB 1 when both are shown).
    pub staff: usize,
    /// Top of the staff's box in system coordinates.
    pub top: f32,
    pub height: f32,
    /// Top of the first staff line (or string) inside the box.
    pub lines_top: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineKind {
    StaffLine,
    TabString,
    Ledger,
    Stem,
    Barline,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub kind: LineKind,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub thickness: f32,
}

/// A SMuFL glyph with its origin at (x, y); drawn at a font size of 4 staff spaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub name: &'static str,
    pub x: f32,
    pub y: f32,
}

/// Text centred at (x, y) (TAB fret numbers, the TAB clef).
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    /// A white box painted under the text to clear the string line (decided by the
    /// layout, so every renderer paints the same).
    pub knockout: Option<Rect>,
}

/// A beam: a band of vertical thickness `thickness` from (x1, y1) to (x2, y2), its
/// centre line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beam {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub thickness: f32,
}

/// A tie arc from (x1, y) to (x2, y), bulging up or down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tie {
    pub x1: f32,
    pub x2: f32,
    pub y: f32,
    pub up: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
}

/// Facts about one laid-out note, for the checks (N1, N3). Not painted.
#[derive(Clone, Debug, PartialEq)]
pub struct NoteFact {
    pub bar: usize,
    pub voice: usize,
    pub beat: usize,
    pub string: u8,
    /// Written pitch: letter index (C = 0), alteration, octave.
    pub written: (i32, i8, i8),
    pub position: i32,
    pub accidental: Option<i8>,
    pub tied: bool,
    pub head: Rect,
    /// The head sits on the other side of the stem (a second in a chord).
    pub displaced: bool,
    pub accidental_box: Option<Rect>,
    pub stem_up: Option<bool>,
    pub ledgers: Vec<Line>,
    pub dots: Vec<(f32, f32)>,
}

/// Facts about one beat's stem (N3).
#[derive(Clone, Debug, PartialEq)]
pub struct StemFact {
    pub bar: usize,
    pub voice: usize,
    pub beat: usize,
    pub up: bool,
    pub x: f32,
    /// From the outermost notehead centre (away from the stem's end) to the stem's end.
    pub length_from_outer: f32,
    pub beamed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RestFact {
    pub bar: usize,
    pub voice: usize,
    pub beat: usize,
    pub rect: Rect,
}

/// A tuplet number (N1: one per tuplet; N3: it clears the notes).
#[derive(Clone, Debug, PartialEq)]
pub struct TupletFact {
    pub bar: usize,
    pub voice: usize,
    pub beats: Vec<usize>,
    pub number: u8,
    pub rect: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BeamFact {
    pub bar: usize,
    pub voice: usize,
    pub up: bool,
    pub beats: Vec<usize>,
    /// The outer beam's band, as a rectangle (beams are flat in the spike).
    pub band: Rect,
}

/// The x where a beat's notes sit on a staff (N2): head or rest centre on the notation
/// staff, fret-number centre on the TAB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    pub bar: usize,
    pub voice: usize,
    pub beat: usize,
    pub x: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Staff {
    pub kind: StaffKind,
    pub position: StaffPosition,
    pub lines: Vec<Line>,
    pub glyphs: Vec<Glyph>,
    pub texts: Vec<Text>,
    pub beams: Vec<Beam>,
    pub ties: Vec<Tie>,
    pub barlines: Vec<f32>,
    pub anchors: Vec<Anchor>,
    pub notes: Vec<NoteFact>,
    pub stems: Vec<StemFact>,
    pub rests: Vec<RestFact>,
    pub beam_facts: Vec<BeamFact>,
    pub tuplets: Vec<TupletFact>,
    /// Bars shown with a whole-bar rest (no beat in any voice).
    pub bar_rests: Vec<RestFact>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct System {
    pub index: usize,
    pub y: f32,
    pub height: f32,
    pub first_bar: usize,
    pub end_bar: usize,
    pub staves: Vec<Staff>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub font: String,
    pub config: Config,
    pub width: f32,
    pub height: f32,
    pub systems: Vec<System>,
}

impl Layout {
    /// Canonical serialization (N8): every painted item and fact, in order, with fixed
    /// float formatting. Two layouts are equal exactly when these strings are.
    pub fn canonical(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        let f = |v: f32| format!("{v:.4}");
        let _ = writeln!(
            s,
            "font {} space {} width {} config {:?}",
            self.font,
            f(self.config.staff_space),
            f(self.config.width),
            self.config.staves
        );
        for sys in &self.systems {
            let _ = writeln!(
                s,
                "system {} y {} h {} bars {}..{}",
                sys.index,
                f(sys.y),
                f(sys.height),
                sys.first_bar,
                sys.end_bar
            );
            for st in &sys.staves {
                let p = st.position;
                let _ = writeln!(
                    s,
                    " staff {:?} track {} staff {} top {} h {}",
                    st.kind,
                    p.track,
                    p.staff,
                    f(p.top),
                    f(p.height)
                );
                for l in &st.lines {
                    let _ = writeln!(
                        s,
                        "  line {:?} {} {} {} {} {}",
                        l.kind,
                        f(l.x1),
                        f(l.y1),
                        f(l.x2),
                        f(l.y2),
                        f(l.thickness)
                    );
                }
                for g in &st.glyphs {
                    let _ = writeln!(s, "  glyph {} {} {}", g.name, f(g.x), f(g.y));
                }
                for t in &st.texts {
                    let k = t.knockout.map(|r| format!("{} {} {} {}", f(r.x), f(r.y), f(r.w), f(r.h))).unwrap_or_default();
                    let _ = writeln!(s, "  text {:?} {} {} {} knockout {k}", t.text, f(t.x), f(t.y), f(t.size));
                }
                for b in &st.beams {
                    let _ = writeln!(s, "  beam {} {} {} {} {}", f(b.x1), f(b.y1), f(b.x2), f(b.y2), f(b.thickness));
                }
                for t in &st.ties {
                    let _ = writeln!(s, "  tie {} {} {} {}", f(t.x1), f(t.x2), f(t.y), t.up);
                }
                for n in &st.notes {
                    let _ = writeln!(
                        s,
                        "  note {}:{}:{} {:?} pos {} acc {:?} tied {} displaced {}",
                        n.bar, n.voice, n.beat, n.written, n.position, n.accidental, n.tied, n.displaced
                    );
                }
            }
        }
        s
    }
}
