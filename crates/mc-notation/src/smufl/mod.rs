//! SMuFL music glyphs and their metrics (ADR-041 decision 1).
//!
//! Units: **staff spaces**. SMuFL defines 1 em = 4 staff spaces; a glyph's origin is
//! the point SMuFL places on the staff (a notehead's origin is on its staff position,
//! at the head's left edge). y grows **upward** in this module, as in SMuFL; the
//! layout converts to layout units with y downward (see `crate::engrave`).
//!
//! The metrics come from a table generated from the bundled Bravura files
//! (`bravura.rs`, never edited by hand). A test regenerates it in memory and compares
//! (`generate.rs`), so the table cannot drift from the font; the font itself is pinned
//! by SHA-256 (FF-35).

// Generated: kept exactly as the generator writes it (the consistency test compares).
#[rustfmt::skip]
mod bravura;
#[cfg(test)]
mod generate;

/// SMuFL anchors the engine uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Anchor {
    StemUpSE,
    StemDownNW,
    StemUpNW,
    StemDownSW,
}

impl Anchor {
    pub const ALL: [Anchor; 4] = [
        Anchor::StemUpSE,
        Anchor::StemDownNW,
        Anchor::StemUpNW,
        Anchor::StemDownSW,
    ];

    /// The SMuFL metadata name.
    pub const fn name(self) -> &'static str {
        match self {
            Anchor::StemUpSE => "stemUpSE",
            Anchor::StemDownNW => "stemDownNW",
            Anchor::StemUpNW => "stemUpNW",
            Anchor::StemDownSW => "stemDownSW",
        }
    }
}

/// A box in staff spaces relative to the glyph origin, y up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphBox {
    /// South-west corner (x, y).
    pub sw: (f32, f32),
    /// North-east corner (x, y).
    pub ne: (f32, f32),
}

impl GlyphBox {
    pub fn width(&self) -> f32 {
        self.ne.0 - self.sw.0
    }

    pub fn height(&self) -> f32 {
        self.ne.1 - self.sw.1
    }
}

/// Metrics of one glyph, in staff spaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphMetrics {
    /// Glyph id in `Bravura.otf` (for PDF embedding, ADR-041 decision 5).
    pub glyph_id: u16,
    /// Horizontal advance from the font's `hmtx` table.
    pub advance: f32,
    /// Bounding box from the SMuFL metadata (checked against the outline).
    pub bbox: GlyphBox,
    /// Anchors present in the metadata, indexed like [`Anchor::ALL`]. A glyph that has
    /// no such anchor has `None`; nothing is assumed.
    pub anchors: [Option<(f32, f32)>; 4],
}

impl GlyphMetrics {
    pub fn anchor(&self, a: Anchor) -> Option<(f32, f32)> {
        self.anchors[a as usize]
    }
}

/// SMuFL engraving defaults, in staff spaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EngravingDefaults {
    pub staff_line_thickness: f32,
    pub stem_thickness: f32,
    pub beam_thickness: f32,
    pub beam_spacing: f32,
    pub leger_line_thickness: f32,
    pub leger_line_extension: f32,
    pub thin_barline_thickness: f32,
    pub thick_barline_thickness: f32,
    pub barline_separation: f32,
    pub repeat_barline_dot_separation: f32,
    pub tie_endpoint_thickness: f32,
    pub tie_midpoint_thickness: f32,
}

/// The music font the metrics belong to (ADR-037).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontIdentity {
    pub name: &'static str,
    pub version: &'static str,
    pub otf_sha256: &'static str,
    pub json_sha256: &'static str,
}

/// Font identity of the generated table; part of the layout cache key (ADR-041
/// decision 3).
pub const FONT: FontIdentity = bravura::FONT;

/// Engraving defaults of the font.
pub const DEFAULTS: EngravingDefaults = bravura::DEFAULTS;

macro_rules! glyphs {
    ($( $variant:ident = $name:literal, $cp:literal, [$($req:ident),*]; )*) => {
        /// The SMuFL glyphs the notation engine uses. Adding one means regenerating
        /// the metrics table (`generate.rs`).
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum SmuflGlyph { $( $variant, )* }

        impl SmuflGlyph {
            /// Every glyph, in table order.
            pub const ALL: &'static [SmuflGlyph] = &[ $( SmuflGlyph::$variant, )* ];

            /// The SMuFL glyph name.
            pub const fn name(self) -> &'static str {
                match self { $( SmuflGlyph::$variant => $name, )* }
            }

            /// The SMuFL codepoint.
            pub const fn codepoint(self) -> u32 {
                match self { $( SmuflGlyph::$variant => $cp, )* }
            }

            /// Anchors the engine relies on; the generator fails if one is missing.
            pub const fn required_anchors(self) -> &'static [Anchor] {
                match self { $( SmuflGlyph::$variant => &[$(Anchor::$req),*], )* }
            }
        }
    };
}

glyphs! {
    GClef8vb = "gClef8vb", 0xE052, [];
    NoteheadWhole = "noteheadWhole", 0xE0A2, [];
    NoteheadHalf = "noteheadHalf", 0xE0A3, [StemUpSE, StemDownNW];
    NoteheadBlack = "noteheadBlack", 0xE0A4, [StemUpSE, StemDownNW];
    AccidentalFlat = "accidentalFlat", 0xE260, [];
    AccidentalNatural = "accidentalNatural", 0xE261, [];
    AccidentalSharp = "accidentalSharp", 0xE262, [];
    AccidentalDoubleSharp = "accidentalDoubleSharp", 0xE263, [];
    AccidentalDoubleFlat = "accidentalDoubleFlat", 0xE264, [];
    TimeSig0 = "timeSig0", 0xE080, [];
    TimeSig1 = "timeSig1", 0xE081, [];
    TimeSig2 = "timeSig2", 0xE082, [];
    TimeSig3 = "timeSig3", 0xE083, [];
    TimeSig4 = "timeSig4", 0xE084, [];
    TimeSig5 = "timeSig5", 0xE085, [];
    TimeSig6 = "timeSig6", 0xE086, [];
    TimeSig7 = "timeSig7", 0xE087, [];
    TimeSig8 = "timeSig8", 0xE088, [];
    TimeSig9 = "timeSig9", 0xE089, [];
    RestWhole = "restWhole", 0xE4E3, [];
    RestHalf = "restHalf", 0xE4E4, [];
    RestQuarter = "restQuarter", 0xE4E5, [];
    Rest8th = "rest8th", 0xE4E6, [];
    Rest16th = "rest16th", 0xE4E7, [];
    Rest32nd = "rest32nd", 0xE4E8, [];
    Rest64th = "rest64th", 0xE4E9, [];
    Rest128th = "rest128th", 0xE4EA, [];
    Rest256th = "rest256th", 0xE4EB, [];
    Flag8thUp = "flag8thUp", 0xE240, [StemUpNW];
    Flag8thDown = "flag8thDown", 0xE241, [StemDownSW];
    Flag16thUp = "flag16thUp", 0xE242, [StemUpNW];
    Flag16thDown = "flag16thDown", 0xE243, [StemDownSW];
    Flag32ndUp = "flag32ndUp", 0xE244, [StemUpNW];
    Flag32ndDown = "flag32ndDown", 0xE245, [StemDownSW];
    Flag64thUp = "flag64thUp", 0xE246, [StemUpNW];
    Flag64thDown = "flag64thDown", 0xE247, [StemDownSW];
    Flag128thUp = "flag128thUp", 0xE248, [StemUpNW];
    Flag128thDown = "flag128thDown", 0xE249, [StemDownSW];
    Flag256thUp = "flag256thUp", 0xE24A, [StemUpNW];
    Flag256thDown = "flag256thDown", 0xE24B, [StemDownSW];
    AugmentationDot = "augmentationDot", 0xE1E7, [];
    RepeatDot = "repeatDot", 0xE044, [];
    Tuplet0 = "tuplet0", 0xE880, [];
    Tuplet1 = "tuplet1", 0xE881, [];
    Tuplet2 = "tuplet2", 0xE882, [];
    Tuplet3 = "tuplet3", 0xE883, [];
    Tuplet4 = "tuplet4", 0xE884, [];
    Tuplet5 = "tuplet5", 0xE885, [];
    Tuplet6 = "tuplet6", 0xE886, [];
    Tuplet7 = "tuplet7", 0xE887, [];
    Tuplet8 = "tuplet8", 0xE888, [];
    Tuplet9 = "tuplet9", 0xE889, [];
}

impl SmuflGlyph {
    /// Metrics from the generated table.
    pub fn metrics(self) -> &'static GlyphMetrics {
        &bravura::METRICS[self as usize]
    }

    /// Notehead for a base value exponent (0 whole, 1 half, else black).
    pub fn notehead(exponent: u8) -> SmuflGlyph {
        match exponent {
            0 => SmuflGlyph::NoteheadWhole,
            1 => SmuflGlyph::NoteheadHalf,
            _ => SmuflGlyph::NoteheadBlack,
        }
    }

    /// Rest for a base value exponent (0 whole ... 8 = 1/256).
    pub fn rest(exponent: u8) -> SmuflGlyph {
        use SmuflGlyph::*;
        [
            RestWhole,
            RestHalf,
            RestQuarter,
            Rest8th,
            Rest16th,
            Rest32nd,
            Rest64th,
            Rest128th,
            Rest256th,
        ][exponent.min(8) as usize]
    }

    /// Flag for an exponent of 3 (eighth) or more, up or down.
    pub fn flag(exponent: u8, up: bool) -> SmuflGlyph {
        use SmuflGlyph::*;
        let i = (exponent.clamp(3, 8) - 3) as usize;
        if up {
            [
                Flag8thUp,
                Flag16thUp,
                Flag32ndUp,
                Flag64thUp,
                Flag128thUp,
                Flag256thUp,
            ][i]
        } else {
            [
                Flag8thDown,
                Flag16thDown,
                Flag32ndDown,
                Flag64thDown,
                Flag128thDown,
                Flag256thDown,
            ][i]
        }
    }

    /// Accidental for an alteration in semitones (-2..=2).
    pub fn accidental(alter: i8) -> SmuflGlyph {
        match alter {
            -2 => SmuflGlyph::AccidentalDoubleFlat,
            -1 => SmuflGlyph::AccidentalFlat,
            1 => SmuflGlyph::AccidentalSharp,
            2 => SmuflGlyph::AccidentalDoubleSharp,
            _ => SmuflGlyph::AccidentalNatural,
        }
    }

    /// Time signature digit 0..=9.
    pub fn time_digit(d: u8) -> SmuflGlyph {
        use SmuflGlyph::*;
        [
            TimeSig0, TimeSig1, TimeSig2, TimeSig3, TimeSig4, TimeSig5, TimeSig6, TimeSig7,
            TimeSig8, TimeSig9,
        ][(d % 10) as usize]
    }

    /// Tuplet number digit 0..=9.
    pub fn tuplet_digit(d: u8) -> SmuflGlyph {
        use SmuflGlyph::*;
        [
            Tuplet0, Tuplet1, Tuplet2, Tuplet3, Tuplet4, Tuplet5, Tuplet6, Tuplet7, Tuplet8,
            Tuplet9,
        ][(d % 10) as usize]
    }
}
