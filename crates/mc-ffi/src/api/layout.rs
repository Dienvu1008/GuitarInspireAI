//! TAB layout projection for painting (ADR-018). Synchronous: S4 measured layout plus
//! FFI at about 50 µs per system; a whole large track lays out in about 3 ms. Lines and
//! hit regions are packed into typed arrays (Float32List, Int32List, Int64List in Dart)
//! to keep the payload small.

use crate::api::document::DocumentHandle;
use crate::api::types::{ApiError, track_id};
use flutter_rust_bridge::frb;
use mc_app::notation::{GlyphKind, HitTarget, LayoutMetrics, LineKind, NotationError};
use std::collections::HashMap;

/// Font and spacing metrics measured by Dart (see `mc_notation::LayoutMetrics`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetricsDto {
    pub width: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    pub string_spacing: f32,
    pub digit_width: f32,
    pub glyph_padding: f32,
    pub min_column_width: f32,
    pub duration_spacing: f32,
    pub bar_padding: f32,
    pub empty_bar_width: f32,
    pub header_width: f32,
    pub time_signature_width: f32,
    pub repeat_width: f32,
    pub above_staff: f32,
    pub below_staff: f32,
    pub system_gap: f32,
    pub stem_length: f32,
}

impl MetricsDto {
    fn to_core(self) -> LayoutMetrics {
        LayoutMetrics {
            width: self.width,
            margin_left: self.margin_left,
            margin_right: self.margin_right,
            string_spacing: self.string_spacing,
            digit_width: self.digit_width,
            glyph_padding: self.glyph_padding,
            min_column_width: self.min_column_width,
            duration_spacing: self.duration_spacing,
            bar_padding: self.bar_padding,
            empty_bar_width: self.empty_bar_width,
            header_width: self.header_width,
            time_signature_width: self.time_signature_width,
            repeat_width: self.repeat_width,
            above_staff: self.above_staff,
            below_staff: self.below_staff,
            system_gap: self.system_gap,
            stem_length: self.stem_length,
        }
    }
}

/// Default metrics in abstract units, for tests and as a starting point before Dart
/// has measured the font.
#[frb(sync)]
pub fn default_metrics() -> MetricsDto {
    let m = LayoutMetrics::default();
    MetricsDto {
        width: m.width,
        margin_left: m.margin_left,
        margin_right: m.margin_right,
        string_spacing: m.string_spacing,
        digit_width: m.digit_width,
        glyph_padding: m.glyph_padding,
        min_column_width: m.min_column_width,
        duration_spacing: m.duration_spacing,
        bar_padding: m.bar_padding,
        empty_bar_width: m.empty_bar_width,
        header_width: m.header_width,
        time_signature_width: m.time_signature_width,
        repeat_width: m.repeat_width,
        above_staff: m.above_staff,
        below_staff: m.below_staff,
        system_gap: m.system_gap,
        stem_length: m.stem_length,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GlyphKindDto {
    Fret {
        fret: u8,
        tied: bool,
        dead: bool,
        let_ring: bool,
    },
    Rest {
        exponent: u8,
        dots: u8,
    },
    Dot,
    Tuplet {
        actual: u8,
    },
    TimeSignature {
        numerator: u8,
        denominator: u8,
    },
    RepeatDots,
    RepeatCount {
        count: u8,
    },
    Volta {
        endings: Vec<u8>,
    },
    Marker {
        text: String,
    },
    BarNumber {
        number: u32,
    },
    TabClef,
    StringName {
        string: u8,
        name: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlyphDto {
    pub kind: GlyphKindDto,
    pub x: f32,
    pub y: f32,
    pub width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarLayoutDto {
    pub bar: u32,
    pub x: f32,
    pub width: f32,
    pub diagnostic: bool,
}

/// Line kind codes in `SystemDto::line_kinds`.
pub const LINE_STRING: u8 = 0;
pub const LINE_BAR: u8 = 1;
pub const LINE_THICK_BAR: u8 = 2;
pub const LINE_STEM: u8 = 3;
pub const LINE_BEAM: u8 = 4;
pub const LINE_VOLTA: u8 = 5;

#[derive(Clone, Debug, PartialEq)]
pub struct SystemDto {
    pub index: u32,
    pub y: f32,
    pub height: f32,
    pub staff_top: f32,
    pub bars: Vec<BarLayoutDto>,
    /// x1, y1, x2, y2 per line, in system coordinates.
    pub lines: Vec<f32>,
    /// One code per line (0 string, 1 bar, 2 thick bar, 3 stem, 4 beam, 5 volta).
    pub line_kinds: Vec<u8>,
    pub glyphs: Vec<GlyphDto>,
    /// x, y, w, h per hit region.
    pub hit_rects: Vec<f32>,
    /// bar, string, voice, slot per hit region. `slot` indexes `beats`, or is -1 for
    /// the append slot of that voice.
    pub hit_meta: Vec<i32>,
    /// Beats addressed by hit regions in this system. A per-beat record (not a per-region
    /// id array) keeps ids as Dart `int`: `Vec<i64>` maps to a `BigInt` list.
    pub beats: Vec<HitBeatDto>,
}

/// A beat that hit regions point to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HitBeatDto {
    pub id: i64,
    pub bar: u32,
    pub voice: u32,
    /// Index of the beat in its voice.
    pub index: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TabLayoutDto {
    pub track: i64,
    pub strings: u8,
    pub width: f32,
    pub height: f32,
    pub systems: Vec<SystemDto>,
}

impl From<NotationError> for ApiError {
    fn from(e: NotationError) -> Self {
        ApiError::new("layout", e.to_string())
    }
}

impl DocumentHandle {
    /// TAB layout of a track at the current revision.
    #[frb(sync)]
    pub fn tab_layout(&self, track: i64, metrics: MetricsDto) -> Result<TabLayoutDto, ApiError> {
        let layout = self.with_session(|s| s.tab_layout(track_id(track), &metrics.to_core()))?;
        Ok(TabLayoutDto {
            track,
            strings: layout.strings,
            width: layout.width,
            height: layout.height,
            systems: layout
                .systems
                .into_iter()
                .map(|s| {
                    let mut lines = Vec::with_capacity(s.lines.len() * 4);
                    let mut line_kinds = Vec::with_capacity(s.lines.len());
                    for l in &s.lines {
                        lines.extend([l.x1, l.y1, l.x2, l.y2]);
                        line_kinds.push(match l.kind {
                            LineKind::String => LINE_STRING,
                            LineKind::BarLine => LINE_BAR,
                            LineKind::ThickBarLine => LINE_THICK_BAR,
                            LineKind::Stem => LINE_STEM,
                            LineKind::Beam => LINE_BEAM,
                            LineKind::Volta => LINE_VOLTA,
                        });
                    }
                    let mut hit_rects = Vec::with_capacity(s.hits.len() * 4);
                    let mut hit_meta = Vec::with_capacity(s.hits.len() * 4);
                    let mut beats: Vec<HitBeatDto> = Vec::new();
                    let mut slots: HashMap<(usize, usize, usize), i32> = HashMap::new();
                    for h in &s.hits {
                        hit_rects.extend([h.rect.x, h.rect.y, h.rect.w, h.rect.h]);
                        let (voice, slot) = match h.target {
                            HitTarget::Beat {
                                voice,
                                beat,
                                beat_id,
                            } => {
                                let slot =
                                    *slots.entry((h.bar, voice, beat)).or_insert_with(|| {
                                        beats.push(HitBeatDto {
                                            id: beat_id.raw() as i64,
                                            bar: h.bar as u32,
                                            voice: voice as u32,
                                            index: beat as u32,
                                        });
                                        beats.len() as i32 - 1
                                    });
                                (voice as i32, slot)
                            }
                            HitTarget::Append { voice } => (voice as i32, -1),
                        };
                        hit_meta.extend([h.bar as i32, h.string as i32, voice, slot]);
                    }
                    SystemDto {
                        index: s.index as u32,
                        y: s.y,
                        height: s.height,
                        staff_top: s.staff_top,
                        bars: s
                            .bars
                            .iter()
                            .map(|b| BarLayoutDto {
                                bar: b.bar as u32,
                                x: b.x,
                                width: b.width,
                                diagnostic: b.diagnostic,
                            })
                            .collect(),
                        lines,
                        line_kinds,
                        glyphs: s.glyphs.into_iter().map(glyph_dto).collect(),
                        hit_rects,
                        hit_meta,
                        beats,
                    }
                })
                .collect(),
        })
    }
}

fn glyph_dto(g: mc_app::notation::Glyph) -> GlyphDto {
    let kind = match g.kind {
        GlyphKind::Fret {
            fret,
            tied,
            dead,
            let_ring,
        } => GlyphKindDto::Fret {
            fret,
            tied,
            dead,
            let_ring,
        },
        GlyphKind::Rest { exponent, dots } => GlyphKindDto::Rest { exponent, dots },
        GlyphKind::Dot => GlyphKindDto::Dot,
        GlyphKind::Tuplet { actual } => GlyphKindDto::Tuplet { actual },
        GlyphKind::TimeSignature {
            numerator,
            denominator,
        } => GlyphKindDto::TimeSignature {
            numerator,
            denominator,
        },
        GlyphKind::RepeatDots => GlyphKindDto::RepeatDots,
        GlyphKind::RepeatCount { count } => GlyphKindDto::RepeatCount { count },
        GlyphKind::Volta { endings } => GlyphKindDto::Volta { endings },
        GlyphKind::Marker { text } => GlyphKindDto::Marker { text },
        GlyphKind::BarNumber { number } => GlyphKindDto::BarNumber {
            number: number as u32,
        },
        GlyphKind::TabClef => GlyphKindDto::TabClef,
        GlyphKind::StringName { string, name } => GlyphKindDto::StringName { string, name },
    };
    GlyphDto {
        kind,
        x: g.x,
        y: g.y,
        width: g.width,
    }
}
