//! @generated from the bundled Bravura files by `smufl::generate`
//! (ADR-041 decision 1). Do not edit: run the ignored test `print_table`
//! and replace this file after review. Units: staff spaces, y up.

use super::{EngravingDefaults, FontIdentity, GlyphBox, GlyphMetrics};

pub(super) const FONT: FontIdentity = FontIdentity {
    name: "Bravura",
    version: "1.482",
    otf_sha256: "CDF0F893EE1FDB64B7F6713D71EE0DCFC349C0AC01429A8E451B01A9E79F5F3B",
    json_sha256: "489FB4D734FC0B72A3CC601B50EBDF0BF8C57802AB9893E52CE030B86BF2CE11",
};

pub(super) const DEFAULTS: EngravingDefaults = EngravingDefaults {
    staff_line_thickness: 0.13,
    stem_thickness: 0.12,
    beam_thickness: 0.5,
    beam_spacing: 0.25,
    leger_line_thickness: 0.16,
    leger_line_extension: 0.4,
    thin_barline_thickness: 0.16,
    thick_barline_thickness: 0.5,
    barline_separation: 0.4,
    repeat_barline_dot_separation: 0.16,
    tie_endpoint_thickness: 0.1,
    tie_midpoint_thickness: 0.22,
};

pub(super) const METRICS: [GlyphMetrics; 52] = [
    // gClef8vb U+E052
    GlyphMetrics {
        glyph_id: 76,
        advance: 2.656,
        bbox: GlyphBox { sw: (0.0, -3.512), ne: (2.684, 4.392) },
        anchors: [None, None, None, None],
    },
    // noteheadWhole U+E0A2
    GlyphMetrics {
        glyph_id: 156,
        advance: 1.688,
        bbox: GlyphBox { sw: (0.0, -0.5), ne: (1.688, 0.5) },
        anchors: [None, None, None, None],
    },
    // noteheadHalf U+E0A3
    GlyphMetrics {
        glyph_id: 157,
        advance: 1.18,
        bbox: GlyphBox { sw: (0.0, -0.5), ne: (1.18, 0.5) },
        anchors: [Some((1.18, 0.168)), Some((0.0, -0.168)), None, None],
    },
    // noteheadBlack U+E0A4
    GlyphMetrics {
        glyph_id: 158,
        advance: 1.18,
        bbox: GlyphBox { sw: (0.0, -0.5), ne: (1.18, 0.5) },
        anchors: [Some((1.18, 0.168)), Some((0.0, -0.168)), None, None],
    },
    // accidentalFlat U+E260
    GlyphMetrics {
        glyph_id: 535,
        advance: 0.904,
        bbox: GlyphBox { sw: (0.0, -0.7), ne: (0.904, 1.756) },
        anchors: [None, None, None, None],
    },
    // accidentalNatural U+E261
    GlyphMetrics {
        glyph_id: 536,
        advance: 0.672,
        bbox: GlyphBox { sw: (0.0, -1.34), ne: (0.672, 1.364) },
        anchors: [None, None, None, None],
    },
    // accidentalSharp U+E262
    GlyphMetrics {
        glyph_id: 537,
        advance: 0.996,
        bbox: GlyphBox { sw: (0.0, -1.392), ne: (0.996, 1.4) },
        anchors: [None, None, None, None],
    },
    // accidentalDoubleSharp U+E263
    GlyphMetrics {
        glyph_id: 538,
        advance: 1.0,
        bbox: GlyphBox { sw: (0.0, -0.5), ne: (0.988, 0.508) },
        anchors: [None, None, None, None],
    },
    // accidentalDoubleFlat U+E264
    GlyphMetrics {
        glyph_id: 539,
        advance: 1.652,
        bbox: GlyphBox { sw: (0.0, -0.7), ne: (1.644, 1.748) },
        anchors: [None, None, None, None],
    },
    // timeSig0 U+E080
    GlyphMetrics {
        glyph_id: 122,
        advance: 1.88,
        bbox: GlyphBox { sw: (0.08, -1.0), ne: (1.8, 1.004) },
        anchors: [None, None, None, None],
    },
    // timeSig1 U+E081
    GlyphMetrics {
        glyph_id: 123,
        advance: 1.336,
        bbox: GlyphBox { sw: (0.08, -1.0), ne: (1.256, 1.004) },
        anchors: [None, None, None, None],
    },
    // timeSig2 U+E082
    GlyphMetrics {
        glyph_id: 124,
        advance: 1.784,
        bbox: GlyphBox { sw: (0.08, -1.028), ne: (1.704, 1.016) },
        anchors: [None, None, None, None],
    },
    // timeSig3 U+E083
    GlyphMetrics {
        glyph_id: 125,
        advance: 1.684,
        bbox: GlyphBox { sw: (0.08, -1.004), ne: (1.604, 0.996) },
        anchors: [None, None, None, None],
    },
    // timeSig4 U+E084
    GlyphMetrics {
        glyph_id: 126,
        advance: 1.88,
        bbox: GlyphBox { sw: (0.08, -1.0), ne: (1.8, 1.004) },
        anchors: [None, None, None, None],
    },
    // timeSig5 U+E085
    GlyphMetrics {
        glyph_id: 127,
        advance: 1.612,
        bbox: GlyphBox { sw: (0.08, -1.004), ne: (1.532, 0.984) },
        anchors: [None, None, None, None],
    },
    // timeSig6 U+E086
    GlyphMetrics {
        glyph_id: 128,
        advance: 1.736,
        bbox: GlyphBox { sw: (0.08, -0.996), ne: (1.656, 1.004) },
        anchors: [None, None, None, None],
    },
    // timeSig7 U+E087
    GlyphMetrics {
        glyph_id: 129,
        advance: 1.764,
        bbox: GlyphBox { sw: (0.08, -1.0), ne: (1.684, 0.996) },
        anchors: [None, None, None, None],
    },
    // timeSig8 U+E088
    GlyphMetrics {
        glyph_id: 130,
        advance: 1.744,
        bbox: GlyphBox { sw: (0.08, -1.036), ne: (1.664, 1.036) },
        anchors: [None, None, None, None],
    },
    // timeSig9 U+E089
    GlyphMetrics {
        glyph_id: 131,
        advance: 1.736,
        bbox: GlyphBox { sw: (0.08, -0.996), ne: (1.656, 1.004) },
        anchors: [None, None, None, None],
    },
    // restWhole U+E4E3
    GlyphMetrics {
        glyph_id: 994,
        advance: 1.132,
        bbox: GlyphBox { sw: (0.0, -0.54), ne: (1.128, 0.036) },
        anchors: [None, None, None, None],
    },
    // restHalf U+E4E4
    GlyphMetrics {
        glyph_id: 995,
        advance: 1.132,
        bbox: GlyphBox { sw: (0.0, -0.008), ne: (1.128, 0.568) },
        anchors: [None, None, None, None],
    },
    // restQuarter U+E4E5
    GlyphMetrics {
        glyph_id: 996,
        advance: 1.08,
        bbox: GlyphBox { sw: (0.004, -1.5), ne: (1.08, 1.492) },
        anchors: [None, None, None, None],
    },
    // rest8th U+E4E6
    GlyphMetrics {
        glyph_id: 997,
        advance: 1.0,
        bbox: GlyphBox { sw: (0.0, -1.004), ne: (0.988, 0.696) },
        anchors: [None, None, None, None],
    },
    // rest16th U+E4E7
    GlyphMetrics {
        glyph_id: 998,
        advance: 1.28,
        bbox: GlyphBox { sw: (0.0, -2.0), ne: (1.28, 0.716) },
        anchors: [None, None, None, None],
    },
    // rest32nd U+E4E8
    GlyphMetrics {
        glyph_id: 999,
        advance: 1.452,
        bbox: GlyphBox { sw: (0.0, -2.0), ne: (1.452, 1.704) },
        anchors: [None, None, None, None],
    },
    // rest64th U+E4E9
    GlyphMetrics {
        glyph_id: 1000,
        advance: 1.696,
        bbox: GlyphBox { sw: (0.0, -3.012), ne: (1.692, 1.72) },
        anchors: [None, None, None, None],
    },
    // rest128th U+E4EA
    GlyphMetrics {
        glyph_id: 1001,
        advance: 1.94,
        bbox: GlyphBox { sw: (0.0, -3.0), ne: (1.94, 2.756) },
        anchors: [None, None, None, None],
    },
    // rest256th U+E4EB
    GlyphMetrics {
        glyph_id: 1002,
        advance: 2.164,
        bbox: GlyphBox { sw: (0.0, -4.0), ne: (2.164, 2.784) },
        anchors: [None, None, None, None],
    },
    // flag8thUp U+E240
    GlyphMetrics {
        glyph_id: 517,
        advance: 1.056,
        bbox: GlyphBox { sw: (0.0, -3.24), ne: (1.056, 0.036) },
        anchors: [None, None, Some((0.0, -0.04)), None],
    },
    // flag8thDown U+E241
    GlyphMetrics {
        glyph_id: 518,
        advance: 1.224,
        bbox: GlyphBox { sw: (0.0, -0.056), ne: (1.224, 3.232) },
        anchors: [None, None, None, Some((0.0, 0.132))],
    },
    // flag16thUp U+E242
    GlyphMetrics {
        glyph_id: 519,
        advance: 1.116,
        bbox: GlyphBox { sw: (0.0, -3.252), ne: (1.116, 0.008) },
        anchors: [None, None, Some((0.0, -0.088)), None],
    },
    // flag16thDown U+E243
    GlyphMetrics {
        glyph_id: 520,
        advance: 1.168,
        bbox: GlyphBox { sw: (0.0, -0.036), ne: (1.164, 3.248026) },
        anchors: [None, None, None, Some((0.0, 0.128))],
    },
    // flag32ndUp U+E244
    GlyphMetrics {
        glyph_id: 521,
        advance: 1.048,
        bbox: GlyphBox { sw: (0.0, -3.248), ne: (1.044, 0.596) },
        anchors: [None, None, Some((0.0, 0.376)), None],
    },
    // flag32ndDown U+E245
    GlyphMetrics {
        glyph_id: 522,
        advance: 1.096,
        bbox: GlyphBox { sw: (0.0, -0.688), ne: (1.092, 3.248) },
        anchors: [None, None, None, Some((0.0, -0.448))],
    },
    // flag64thUp U+E246
    GlyphMetrics {
        glyph_id: 523,
        advance: 1.048,
        bbox: GlyphBox { sw: (0.0, -3.248), ne: (1.044, 1.388) },
        anchors: [None, None, Some((0.0, 1.172)), None],
    },
    // flag64thDown U+E247
    GlyphMetrics {
        glyph_id: 524,
        advance: 1.1,
        bbox: GlyphBox { sw: (0.0, -1.504), ne: (1.092, 3.248) },
        anchors: [None, None, None, Some((0.0, -1.244))],
    },
    // flag128thUp U+E248
    GlyphMetrics {
        glyph_id: 525,
        advance: 1.036,
        bbox: GlyphBox { sw: (0.0, -3.248), ne: (1.044, 2.132) },
        anchors: [None, None, Some((0.0, 1.9)), None],
    },
    // flag128thDown U+E249
    GlyphMetrics {
        glyph_id: 526,
        advance: 1.1,
        bbox: GlyphBox { sw: (0.0, -2.32), ne: (1.092, 3.248) },
        anchors: [None, None, None, Some((0.0, -2.076))],
    },
    // flag256thUp U+E24A
    GlyphMetrics {
        glyph_id: 527,
        advance: 1.072,
        bbox: GlyphBox { sw: (0.0, -3.248), ne: (1.056, 2.816) },
        anchors: [None, None, Some((0.0, 2.592)), None],
    },
    // flag256thDown U+E24B
    GlyphMetrics {
        glyph_id: 528,
        advance: 1.204,
        bbox: GlyphBox { sw: (0.0, -3.004), ne: (1.196, 3.252) },
        anchors: [None, None, None, Some((0.0, -2.812))],
    },
    // augmentationDot U+E1E7
    GlyphMetrics {
        glyph_id: 459,
        advance: 0.4,
        bbox: GlyphBox { sw: (0.0, -0.2), ne: (0.4, 0.2) },
        anchors: [None, None, None, None],
    },
    // repeatDot U+E044
    GlyphMetrics {
        glyph_id: 64,
        advance: 0.4,
        bbox: GlyphBox { sw: (0.0, -0.2), ne: (0.4, 0.2) },
        anchors: [None, None, None, None],
    },
    // tuplet0 U+E880
    GlyphMetrics {
        glyph_id: 1715,
        advance: 1.276,
        bbox: GlyphBox { sw: (0.0, -0.032), ne: (1.272, 1.5) },
        anchors: [None, None, None, None],
    },
    // tuplet1 U+E881
    GlyphMetrics {
        glyph_id: 1716,
        advance: 0.984,
        bbox: GlyphBox { sw: (0.04, 0.0), ne: (1.024, 1.488) },
        anchors: [None, None, None, None],
    },
    // tuplet2 U+E882
    GlyphMetrics {
        glyph_id: 1717,
        advance: 1.276,
        bbox: GlyphBox { sw: (0.04, -0.024), ne: (1.316, 1.5) },
        anchors: [None, None, None, None],
    },
    // tuplet3 U+E883
    GlyphMetrics {
        glyph_id: 1718,
        advance: 1.184,
        bbox: GlyphBox { sw: (0.04, -0.032), ne: (1.224, 1.5) },
        anchors: [None, None, None, None],
    },
    // tuplet4 U+E884
    GlyphMetrics {
        glyph_id: 1719,
        advance: 1.212,
        bbox: GlyphBox { sw: (0.04, 0.0), ne: (1.252, 1.488) },
        anchors: [None, None, None, None],
    },
    // tuplet5 U+E885
    GlyphMetrics {
        glyph_id: 1720,
        advance: 1.268,
        bbox: GlyphBox { sw: (0.04, -0.032), ne: (1.308, 1.492) },
        anchors: [None, None, None, None],
    },
    // tuplet6 U+E886
    GlyphMetrics {
        glyph_id: 1721,
        advance: 1.216,
        bbox: GlyphBox { sw: (0.04, -0.032), ne: (1.256, 1.5) },
        anchors: [None, None, None, None],
    },
    // tuplet7 U+E887
    GlyphMetrics {
        glyph_id: 1722,
        advance: 1.212,
        bbox: GlyphBox { sw: (0.12, -0.016), ne: (1.332, 1.488) },
        anchors: [None, None, None, None],
    },
    // tuplet8 U+E888
    GlyphMetrics {
        glyph_id: 1723,
        advance: 1.252,
        bbox: GlyphBox { sw: (0.04, -0.032), ne: (1.292, 1.5) },
        anchors: [None, None, None, None],
    },
    // tuplet9 U+E889
    GlyphMetrics {
        glyph_id: 1724,
        advance: 1.216,
        bbox: GlyphBox { sw: (0.04, -0.032), ne: (1.256, 1.5) },
        anchors: [None, None, None, None],
    },
];
