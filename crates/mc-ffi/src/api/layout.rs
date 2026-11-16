//! TAB layout projection for painting (ADR-018). Synchronous: S4 measured layout plus
//! FFI at about 50 µs per system; a whole large track lays out in about 3 ms. Lines and
//! hit regions are packed into typed arrays (Float32List, Int32List, Int64List in Dart)
//! to keep the payload small.

use crate::api::document::DocumentHandle;
use crate::api::types::{ApiError, track_id};
use flutter_rust_bridge::frb;
use mc_app::notation::{GlyphKind, HitTarget, LayoutMetrics, LineKind, NotationError, StaveKind};
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
    /// Accent mark above the staff (ADR-035).
    Accent,
    /// Stroke arrow above the staff: down or up (ADR-035).
    Stroke {
        up: bool,
    },
    /// "S" mark of a string-percussive beat (ADR-035).
    StringPercussive,
    /// "P.M." label; `run` means a dashed line over the glyph width follows the label
    /// (ADR-035).
    PalmMute {
        run: bool,
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

/// One track's stave in a multi-track system (ADR-031). `system` is in stave
/// coordinates, like a single-track system, so Dart paints and hit-tests it the same
/// way; `top` places it in the system. Bar x-positions are equal in every stave.
#[derive(Clone, Debug, PartialEq)]
pub struct StaveDto {
    pub track: i64,
    /// `STAVE_TAB` or `STAVE_NOTATION` (ADR-038). A track with both has two staves,
    /// notation first.
    pub kind: u8,
    pub strings: u8,
    pub top: f32,
    /// TAB content (empty lists on a notation stave, except the bars).
    pub system: SystemDto,
    /// Notation content (empty on a TAB stave).
    pub notation: NotationStaffDto,
}

/// Stave kind codes in `StaveDto::kind`.
pub const STAVE_TAB: u8 = 0;
pub const STAVE_NOTATION: u8 = 1;

/// Line kind codes in `NotationStaffDto::line_kinds`.
pub const NOTATION_LINE_STAFF: u8 = 0;
pub const NOTATION_LINE_LEDGER: u8 = 1;
pub const NOTATION_LINE_STEM: u8 = 2;
pub const NOTATION_LINE_BAR: u8 = 3;
pub const NOTATION_LINE_THICK_BAR: u8 = 4;

/// A notation stave as packed arrays, in stave coordinates (ADR-041 section 9). Dart
/// paints it as given: glyphs with the bundled Bravura at 4 staff spaces per em, each
/// on its alphabetic baseline at its SMuFL origin.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotationStaffDto {
    /// Layout units per staff space.
    pub staff_space: f32,
    /// y of the top staff line.
    pub lines_top: f32,
    /// x1, y1, x2, y2, thickness per line.
    pub lines: Vec<f32>,
    /// One `NOTATION_LINE_*` code per line.
    pub line_kinds: Vec<u8>,
    /// SMuFL codepoint per glyph.
    pub glyph_codepoints: Vec<u32>,
    /// x, y of each glyph's origin.
    pub glyph_positions: Vec<f32>,
    /// x1, y1, x2, y2, thickness per beam (the band's centre line).
    pub beams: Vec<f32>,
    /// x1, x2, y, height, end thickness, middle thickness per tie.
    pub ties: Vec<f32>,
    /// 1 when the tie bulges up, per tie.
    pub tie_up: Vec<u8>,
}

/// A shown track and its staves (ADR-038). A view setting the adapter passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownTrackDto {
    pub track: i64,
    pub notation: bool,
    pub tab: bool,
}

impl ShownTrackDto {
    fn to_core(self) -> mc_app::notation::ShownTrack {
        mc_app::notation::ShownTrack {
            track: track_id(self.track),
            staves: mc_app::notation::StaffConfiguration {
                notation: self.notation,
                tab: self.tab,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScoreSystemDto {
    pub index: u32,
    pub y: f32,
    pub height: f32,
    pub staves: Vec<StaveDto>,
}

/// Where a system sits and which bars it holds, without content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutlineSystemDto {
    pub index: u32,
    pub y: f32,
    pub height: f32,
    pub first_bar: u32,
    /// One past the last bar.
    pub end_bar: u32,
}

/// Line breaking of the shown tracks at `revision` (ADR-031): sizes the scroll view.
/// Fetch the content of the systems in view with `score_systems`.
/// No track list here: a returned `Vec<i64>` becomes `BigInt`s in Dart; each stave of
/// `score_systems` carries its track id instead (score order).
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreOutlineDto {
    pub revision: i64,
    pub width: f32,
    pub height: f32,
    pub systems: Vec<OutlineSystemDto>,
}

impl From<NotationError> for ApiError {
    fn from(e: NotationError) -> Self {
        ApiError::new("layout", e.to_string())
    }
}

impl DocumentHandle {
    /// Line breaking of the shown `tracks` at the current revision (ADR-031). Kept by
    /// the core per (revision, tracks, metrics), so the systems fetched next match it.
    #[frb(sync)]
    pub fn score_outline(
        &self,
        tracks: Vec<i64>,
        metrics: MetricsDto,
    ) -> Result<ScoreOutlineDto, ApiError> {
        let shown: Vec<ShownTrackDto> = tracks
            .iter()
            .map(|&track| ShownTrackDto {
                track,
                notation: false,
                tab: true,
            })
            .collect();
        self.score_outline_with(shown, metrics)
    }

    /// [`Self::score_outline`] with the staves of each shown track (ADR-038, ADR-041
    /// section 9): notation above TAB where asked.
    #[frb(sync)]
    pub fn score_outline_with(
        &self,
        shown: Vec<ShownTrackDto>,
        metrics: MetricsDto,
    ) -> Result<ScoreOutlineDto, ApiError> {
        let shown: Vec<_> = shown.into_iter().map(ShownTrackDto::to_core).collect();
        let mut s = self.lock_for_layout();
        let revision = s.revision().0 as i64;
        let o = s.score_outline_with(&shown, &metrics.to_core())?;
        Ok(ScoreOutlineDto {
            revision,
            width: o.width,
            height: o.height,
            systems: o
                .systems
                .iter()
                .map(|x| OutlineSystemDto {
                    index: x.index as u32,
                    y: x.y,
                    height: x.height,
                    first_bar: x.first_bar as u32,
                    end_bar: x.end_bar as u32,
                })
                .collect(),
        })
    }

    /// The content of `count` systems starting at `first`, for the current outline of
    /// the same tracks and metrics. Only these systems are laid out.
    #[frb(sync)]
    pub fn score_systems(
        &self,
        tracks: Vec<i64>,
        metrics: MetricsDto,
        first: u32,
        count: u32,
    ) -> Result<Vec<ScoreSystemDto>, ApiError> {
        let shown: Vec<ShownTrackDto> = tracks
            .iter()
            .map(|&track| ShownTrackDto {
                track,
                notation: false,
                tab: true,
            })
            .collect();
        self.score_systems_with(shown, metrics, first, count)
    }

    /// [`Self::score_systems`] with the staves of each shown track (ADR-038), for the
    /// current outline of the same configuration and metrics.
    #[frb(sync)]
    pub fn score_systems_with(
        &self,
        shown: Vec<ShownTrackDto>,
        metrics: MetricsDto,
        first: u32,
        count: u32,
    ) -> Result<Vec<ScoreSystemDto>, ApiError> {
        let shown: Vec<_> = shown.into_iter().map(ShownTrackDto::to_core).collect();
        let systems = self.lock_for_layout().score_systems_with(
            &shown,
            &metrics.to_core(),
            first as usize..(first as usize + count as usize),
        )?;
        Ok(systems.into_iter().map(score_system_dto).collect())
    }

    /// TAB layout of a track at the current revision.
    #[frb(sync)]
    pub fn tab_layout(&self, track: i64, metrics: MetricsDto) -> Result<TabLayoutDto, ApiError> {
        let layout = self.with_session(|s| s.tab_layout(track_id(track), &metrics.to_core()))?;
        Ok(TabLayoutDto {
            track,
            strings: layout.strings,
            width: layout.width,
            height: layout.height,
            systems: layout.systems.into_iter().map(system_dto).collect(),
        })
    }
}

fn score_system_dto(sys: mc_app::notation::ScoreSystem) -> ScoreSystemDto {
    ScoreSystemDto {
        index: sys.index as u32,
        y: sys.y,
        height: sys.height,
        staves: sys
            .staves
            .into_iter()
            .map(|st| {
                let notation = notation_dto(&st.layout.notation);
                StaveDto {
                    track: st.track.raw() as i64,
                    kind: match st.kind {
                        StaveKind::Tab => STAVE_TAB,
                        StaveKind::Notation => STAVE_NOTATION,
                    },
                    strings: st.strings,
                    top: st.top,
                    system: system_dto(st.layout),
                    notation,
                }
            })
            .collect(),
    }
}

/// A notation stave as packed arrays (empty for a TAB stave).
fn notation_dto(n: &mc_app::notation::NotationStaff) -> NotationStaffDto {
    use mc_app::notation::EngravedLineKind as K;
    let mut d = NotationStaffDto {
        staff_space: n.staff_space,
        lines_top: n.lines_top,
        ..NotationStaffDto::default()
    };
    for l in &n.lines {
        d.lines.extend([l.x1, l.y1, l.x2, l.y2, l.thickness]);
        d.line_kinds.push(match l.kind {
            K::StaffLine => NOTATION_LINE_STAFF,
            K::LedgerLine => NOTATION_LINE_LEDGER,
            K::Stem => NOTATION_LINE_STEM,
            K::BarLine => NOTATION_LINE_BAR,
            K::ThickBarLine => NOTATION_LINE_THICK_BAR,
        });
    }
    for g in &n.glyphs {
        d.glyph_codepoints.push(g.glyph.codepoint());
        d.glyph_positions.extend([g.x, g.y]);
    }
    for b in &n.beams {
        d.beams.extend([b.x1, b.y1, b.x2, b.y2, b.thickness]);
    }
    for t in &n.ties {
        d.ties
            .extend([t.x1, t.x2, t.y, t.height, t.end_thickness, t.mid_thickness]);
        d.tie_up.push(u8::from(t.up));
    }
    d
}

/// One stave or single-track system as packed arrays (lines, hit regions).
fn system_dto(s: mc_app::notation::SystemLayout) -> SystemDto {
    {
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
                    let slot = *slots.entry((h.bar, voice, beat)).or_insert_with(|| {
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
        GlyphKind::Accent => GlyphKindDto::Accent,
        GlyphKind::Stroke { up } => GlyphKindDto::Stroke { up },
        GlyphKind::StringPercussive => GlyphKindDto::StringPercussive,
        GlyphKind::PalmMute { run } => GlyphKindDto::PalmMute { run },
    };
    GlyphDto {
        kind,
        x: g.x,
        y: g.y,
        width: g.width,
    }
}

#[cfg(test)]
mod tests {
    //! ADR-041 section 9: F1 (TAB unchanged through the new functions) and F2 (the
    //! notation DTO unpacks to the core layout exactly).
    use super::*;
    use mc_app::fixtures::{SMALL, feature_rich, reference, techniques};
    use mc_app::notation::{ShownTrack, StaffConfiguration};

    fn handles() -> Vec<(&'static str, DocumentHandle)> {
        vec![
            (
                "feature_rich",
                DocumentHandle::from_imported_score(feature_rich()),
            ),
            (
                "techniques",
                DocumentHandle::from_imported_score(techniques()),
            ),
            (
                "small",
                DocumentHandle::from_imported_score(reference(SMALL)),
            ),
        ]
    }

    fn ids(d: &DocumentHandle) -> Vec<i64> {
        d.with_session(|s| s.score().tracks().map(|t| t.id().raw() as i64).collect())
    }

    fn shown(ids: &[i64], notation: bool) -> Vec<ShownTrackDto> {
        ids.iter()
            .map(|&track| ShownTrackDto {
                track,
                notation,
                tab: true,
            })
            .collect()
    }

    #[test]
    fn f1_tab_only_through_the_new_functions_equals_the_old_ones() {
        let m = default_metrics();
        for (name, d) in handles() {
            let ids = ids(&d);
            let old_o = d.score_outline(ids.clone(), m).unwrap();
            let new_o = d.score_outline_with(shown(&ids, false), m).unwrap();
            assert_eq!(old_o, new_o, "{name}");
            let n = old_o.systems.len() as u32;
            let old = d.score_systems(ids.clone(), m, 0, n).unwrap();
            let new = d.score_systems_with(shown(&ids, false), m, 0, n).unwrap();
            assert_eq!(old, new, "{name}");
            for st in old.iter().flat_map(|s| &s.staves) {
                assert_eq!(st.kind, STAVE_TAB);
                assert_eq!(st.notation, NotationStaffDto::default());
            }
        }
    }

    #[test]
    fn f2_notation_dto_unpacks_to_the_core_layout_exactly() {
        let m = default_metrics();
        let mut counted = (0, 0, 0, 0);
        for (name, d) in handles() {
            let ids = ids(&d);
            let o = d.score_outline_with(shown(&ids, true), m).unwrap();
            let n = o.systems.len() as u32;
            let dtos = d.score_systems_with(shown(&ids, true), m, 0, n).unwrap();
            let core: Vec<_> = {
                let shown: Vec<ShownTrack> = ids
                    .iter()
                    .map(|&t| ShownTrack {
                        track: track_id(t),
                        staves: StaffConfiguration::NOTATION_AND_TAB,
                    })
                    .collect();
                d.lock_for_layout()
                    .score_systems_with(&shown, &m.to_core(), 0..n as usize)
                    .unwrap()
            };
            assert_eq!(dtos.len(), core.len(), "{name}");
            for (dto, sys) in dtos.iter().zip(&core) {
                // Notation then TAB per track, in score order.
                let order: Vec<(i64, u8)> = dto.staves.iter().map(|s| (s.track, s.kind)).collect();
                let expected: Vec<(i64, u8)> = ids
                    .iter()
                    .flat_map(|t| [(*t, STAVE_NOTATION), (*t, STAVE_TAB)])
                    .collect();
                assert_eq!(order, expected, "{name}");
                for (st, cs) in dto.staves.iter().zip(&sys.staves) {
                    let n = &st.notation;
                    let c = &cs.layout.notation;
                    assert_eq!((n.staff_space, n.lines_top), (c.staff_space, c.lines_top));
                    assert_eq!(n.lines.len(), c.lines.len() * 5);
                    for (i, l) in c.lines.iter().enumerate() {
                        assert_eq!(
                            &n.lines[i * 5..i * 5 + 5],
                            &[l.x1, l.y1, l.x2, l.y2, l.thickness]
                        );
                    }
                    assert_eq!(n.line_kinds.len(), c.lines.len());
                    assert_eq!(n.glyph_codepoints.len(), c.glyphs.len());
                    for (i, g) in c.glyphs.iter().enumerate() {
                        assert_eq!(n.glyph_codepoints[i], g.glyph.codepoint());
                        assert_eq!(&n.glyph_positions[i * 2..i * 2 + 2], &[g.x, g.y]);
                    }
                    for (i, b) in c.beams.iter().enumerate() {
                        assert_eq!(
                            &n.beams[i * 5..i * 5 + 5],
                            &[b.x1, b.y1, b.x2, b.y2, b.thickness]
                        );
                    }
                    for (i, t) in c.ties.iter().enumerate() {
                        assert_eq!(
                            &n.ties[i * 6..i * 6 + 6],
                            &[t.x1, t.x2, t.y, t.height, t.end_thickness, t.mid_thickness]
                        );
                        assert_eq!(n.tie_up[i], u8::from(t.up));
                    }
                    counted.0 += c.lines.len();
                    counted.1 += c.glyphs.len();
                    counted.2 += c.beams.len();
                    counted.3 += c.ties.len();
                }
            }
        }
        // The fixtures carry every kind of item (no vacuous check).
        assert!(
            counted.0 > 0 && counted.1 > 0 && counted.2 > 0 && counted.3 > 0,
            "{counted:?}"
        );
    }
}
