//! SMuFL font data for the spike: glyph codepoints (from the SMuFL specification's
//! glyph names), bounding boxes and anchors from `Bravura.json`, and engraving defaults.
//! All values are in staff spaces (SMuFL: 1 em = 4 staff spaces).

use serde_json::Value;
use std::collections::BTreeMap;

/// The glyphs the spike uses: SMuFL name and codepoint.
pub const GLYPHS: &[(&str, u32)] = &[
    ("gClef8vb", 0xE052),
    ("noteheadWhole", 0xE0A2),
    ("noteheadHalf", 0xE0A3),
    ("noteheadBlack", 0xE0A4),
    ("accidentalFlat", 0xE260),
    ("accidentalNatural", 0xE261),
    ("accidentalSharp", 0xE262),
    ("timeSig0", 0xE080),
    ("timeSig1", 0xE081),
    ("timeSig2", 0xE082),
    ("timeSig3", 0xE083),
    ("timeSig4", 0xE084),
    ("timeSig5", 0xE085),
    ("timeSig6", 0xE086),
    ("timeSig7", 0xE087),
    ("timeSig8", 0xE088),
    ("timeSig9", 0xE089),
    ("restWhole", 0xE4E3),
    ("restHalf", 0xE4E4),
    ("restQuarter", 0xE4E5),
    ("rest8th", 0xE4E6),
    ("rest16th", 0xE4E7),
    ("rest32nd", 0xE4E8),
    ("rest64th", 0xE4E9),
    ("flag8thUp", 0xE240),
    ("flag8thDown", 0xE241),
    ("flag16thUp", 0xE242),
    ("flag16thDown", 0xE243),
    ("flag32ndUp", 0xE244),
    ("flag32ndDown", 0xE245),
    ("flag64thUp", 0xE246),
    ("flag64thDown", 0xE247),
    ("augmentationDot", 0xE1E7),
    ("tuplet0", 0xE880),
    ("tuplet1", 0xE881),
    ("tuplet2", 0xE882),
    ("tuplet3", 0xE883),
    ("tuplet4", 0xE884),
    ("tuplet5", 0xE885),
    ("tuplet6", 0xE886),
    ("tuplet7", 0xE887),
    ("tuplet8", 0xE888),
    ("tuplet9", 0xE889),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BBox {
    /// South-west corner (x, y), y up, in staff spaces from the glyph origin.
    pub sw: (f32, f32),
    pub ne: (f32, f32),
}

impl BBox {
    pub fn width(&self) -> f32 {
        self.ne.0 - self.sw.0
    }
}

#[derive(Clone, Debug)]
pub struct Font {
    pub name: String,
    pub version: String,
    pub bbox: BTreeMap<&'static str, BBox>,
    /// stemUpSE and stemDownNW of the noteheads.
    pub anchors: BTreeMap<(&'static str, &'static str), (f32, f32)>,
    pub staff_line: f32,
    pub stem: f32,
    pub beam: f32,
    pub beam_spacing: f32,
    pub leger_line: f32,
    pub leger_extension: f32,
    pub thin_barline: f32,
    pub thick_barline: f32,
    /// The font file bytes (for PDF embedding and the cmap check).
    pub otf: Vec<u8>,
}

impl Font {
    pub fn codepoint(name: &str) -> u32 {
        GLYPHS
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("glyph {name} not in the spike's list"))
            .1
    }

    pub fn bbox(&self, name: &str) -> BBox {
        *self
            .bbox
            .get(name)
            .unwrap_or_else(|| panic!("no bbox for {name}"))
    }

    pub fn anchor(&self, glyph: &str, anchor: &str) -> (f32, f32) {
        *self
            .anchors
            .iter()
            .find(|((g, a), _)| *g == glyph && *a == anchor)
            .unwrap_or_else(|| panic!("no anchor {anchor} on {glyph}"))
            .1
    }

    /// Reads `Bravura.json` and `Bravura.otf` from `dir`, and checks every codepoint of
    /// [`GLYPHS`] against the font's own cmap and the metadata's bounding box.
    pub fn load(dir: &str) -> Result<Font, String> {
        let json = std::fs::read_to_string(format!("{dir}/Bravura.json"))
            .map_err(|e| format!("Bravura.json: {e}"))?;
        let otf = std::fs::read(format!("{dir}/Bravura.otf")).map_err(|e| format!("Bravura.otf: {e}"))?;
        let meta: Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let num = |v: &Value| v.as_f64().map(|x| x as f32).ok_or("not a number".to_string());
        let pair = |v: &Value| -> Result<(f32, f32), String> {
            Ok((num(&v[0])?, num(&v[1])?))
        };
        let defaults = &meta["engravingDefaults"];
        let d = |k: &str| num(&defaults[k]).map_err(|e| format!("engravingDefaults.{k}: {e}"));
        let mut bbox = BTreeMap::new();
        let mut anchors = BTreeMap::new();
        for (name, _) in GLYPHS {
            let b = &meta["glyphBBoxes"][*name];
            bbox.insert(
                *name,
                BBox {
                    sw: pair(&b["bBoxSW"]).map_err(|e| format!("{name} bBoxSW: {e}"))?,
                    ne: pair(&b["bBoxNE"]).map_err(|e| format!("{name} bBoxNE: {e}"))?,
                },
            );
            for anchor in ["stemUpSE", "stemDownNW"] {
                let a = &meta["glyphsWithAnchors"][*name][anchor];
                if !a.is_null() {
                    anchors.insert((*name, anchor), pair(a)?);
                }
            }
        }
        let font = Font {
            name: meta["fontName"].as_str().unwrap_or("").to_string(),
            version: meta["fontVersion"].to_string(),
            bbox,
            anchors,
            staff_line: d("staffLineThickness")?,
            stem: d("stemThickness")?,
            beam: d("beamThickness")?,
            beam_spacing: d("beamSpacing")?,
            leger_line: d("legerLineThickness")?,
            leger_extension: d("legerLineExtension")?,
            thin_barline: d("thinBarlineThickness")?,
            thick_barline: d("thickBarlineThickness")?,
            otf,
        };
        font.check_cmap()?;
        Ok(font)
    }

    /// Every codepoint maps to a glyph in the font, and the glyph's outline box (font
    /// units, 1 em = 4 staff spaces) agrees with the metadata box within 0.05 staff
    /// spaces. This catches a wrong codepoint in [`GLYPHS`].
    fn check_cmap(&self) -> Result<(), String> {
        let face = ttf_parser::Face::parse(&self.otf, 0).map_err(|e| e.to_string())?;
        let upem = face.units_per_em() as f32;
        for (name, cp) in GLYPHS {
            let ch = char::from_u32(*cp).ok_or("bad codepoint")?;
            let gid = face
                .glyph_index(ch)
                .ok_or_else(|| format!("{name} U+{cp:04X}: not in the font's cmap"))?;
            let r = face
                .glyph_bounding_box(gid)
                .ok_or_else(|| format!("{name}: no outline"))?;
            let sp = |v: i16| v as f32 / upem * 4.0;
            let b = self.bbox(name);
            let diffs = [
                (sp(r.x_min) - b.sw.0).abs(),
                (sp(r.y_min) - b.sw.1).abs(),
                (sp(r.x_max) - b.ne.0).abs(),
                (sp(r.y_max) - b.ne.1).abs(),
            ];
            if diffs.iter().any(|d| *d > 0.05) {
                return Err(format!(
                    "{name} U+{cp:04X}: font box differs from metadata box by {diffs:?}"
                ));
            }
        }
        Ok(())
    }
}
