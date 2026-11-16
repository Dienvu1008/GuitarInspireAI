//! Generation and checks of the SMuFL metrics table (ADR-041 section 8: L7, L8).
//!
//! - `table_matches_the_bundled_font`: the committed `bravura.rs` equals a fresh
//!   generation from the bundled files. On a difference it prints the new table.
//! - `print_table` (ignored): prints the table, to replace `bravura.rs` by hand after
//!   review, like the layout goldens.
//! - `ff35_*`: FF-35, the bundled files equal the sizes and SHA-256 in `PROVENANCE.md`.
//!
//! The files are read with `include_bytes!` (no file I/O at run time, FF-03).

use super::{Anchor, SmuflGlyph};
use sha2::{Digest, Sha256};
use std::fmt::Write;

const OTF: &[u8] = include_bytes!("../../../../packages/ui_kit/fonts/bravura/Bravura.otf");
const JSON: &[u8] = include_bytes!("../../../../packages/ui_kit/fonts/bravura/Bravura.json");
const OFL: &[u8] = include_bytes!("../../../../packages/ui_kit/fonts/bravura/OFL.txt");
const PROVENANCE: &str = include_str!("../../../../packages/ui_kit/fonts/bravura/PROVENANCE.md");
const COMMITTED: &str = include_str!("bravura.rs");

/// Tolerance between the metadata box and the font outline box (S11 check).
const BOX_TOLERANCE_SP: f32 = 0.05;
/// SMuFL: one em is four staff spaces.
const SP_PER_EM: f32 = 4.0;

/// One glyph the generator must find.
#[derive(Clone, Copy)]
struct Spec {
    name: &'static str,
    codepoint: u32,
    required: &'static [Anchor],
}

fn specs() -> Vec<Spec> {
    SmuflGlyph::ALL
        .iter()
        .map(|g| Spec {
            name: g.name(),
            codepoint: g.codepoint(),
            required: g.required_anchors(),
        })
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02X}");
            s
        })
}

/// f32 in its shortest round-trip form, always with a decimal point.
fn lit(v: f32) -> String {
    let s = format!("{v:?}");
    if s.contains('.') || s.contains('e') {
        s
    } else {
        format!("{s}.0")
    }
}

/// The font queries the checks need, in font units, through `skrifa`.
struct Face<'a> {
    font: skrifa::FontRef<'a>,
    upem: f32,
}

/// An outline box in font units: x_min, y_min, x_max, y_max.
struct Outline {
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
}

impl<'a> Face<'a> {
    fn parse(otf: &'a [u8]) -> Result<Self, String> {
        use skrifa::prelude::*;
        let font = FontRef::new(otf).map_err(|e| format!("{e}"))?;
        let upem = font
            .metrics(Size::unscaled(), LocationRef::default())
            .units_per_em as f32;
        Ok(Face { font, upem })
    }

    fn glyph_index(&self, ch: char) -> Option<u16> {
        use skrifa::MetadataProvider;
        let gid = self.font.charmap().map(ch)?;
        u16::try_from(gid.to_u32()).ok()
    }

    fn metrics(&self) -> skrifa::metrics::GlyphMetrics<'a> {
        use skrifa::prelude::*;
        self.font
            .glyph_metrics(Size::unscaled(), LocationRef::default())
    }

    fn advance(&self, gid: u16) -> Option<f32> {
        self.metrics()
            .advance_width(skrifa::GlyphId::new(gid as u32))
    }

    /// The outline box (for CFF fonts computed from the outline by skrifa).
    fn outline_box(&self, gid: u16) -> Option<Outline> {
        let b = self.metrics().bounds(skrifa::GlyphId::new(gid as u32))?;
        Some(Outline {
            x_min: b.x_min,
            y_min: b.y_min,
            x_max: b.x_max,
            y_max: b.y_max,
        })
    }
}

fn pair(v: &serde_json::Value) -> Option<(f32, f32)> {
    Some((v.get(0)?.as_f64()? as f32, v.get(1)?.as_f64()? as f32))
}

/// The table source from the font files, or the first problem by glyph name.
fn generate(json: &[u8], otf: &[u8], specs: &[Spec]) -> Result<String, String> {
    let meta: serde_json::Value =
        serde_json::from_slice(json).map_err(|e| format!("Bravura.json: {e}"))?;
    let face = Face::parse(otf).map_err(|e| format!("Bravura.otf: {e}"))?;
    let upem = face.upem;
    let to_sp = |units: f32| units / upem * SP_PER_EM;

    let name = meta["fontName"].as_str().ok_or("fontName missing")?;
    let version = meta["fontVersion"].to_string();
    let defaults = &meta["engravingDefaults"];
    let d = |k: &str| -> Result<String, String> {
        defaults[k]
            .as_f64()
            .map(|v| lit(v as f32))
            .ok_or_else(|| format!("engravingDefaults.{k} missing"))
    };

    let mut s = String::new();
    let _ = writeln!(
        s,
        "//! @generated from the bundled Bravura files by `smufl::generate`"
    );
    let _ = writeln!(
        s,
        "//! (ADR-041 decision 1). Do not edit: run the ignored test `print_table`"
    );
    let _ = writeln!(
        s,
        "//! and replace this file after review. Units: staff spaces, y up."
    );
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "use super::{{EngravingDefaults, FontIdentity, GlyphBox, GlyphMetrics}};"
    );
    let _ = writeln!(s);
    let _ = writeln!(s, "pub(super) const FONT: FontIdentity = FontIdentity {{");
    let _ = writeln!(s, "    name: {name:?},");
    let _ = writeln!(s, "    version: {version:?},");
    let _ = writeln!(s, "    otf_sha256: {:?},", sha256_hex(otf));
    let _ = writeln!(s, "    json_sha256: {:?},", sha256_hex(json));
    let _ = writeln!(s, "}};");
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "pub(super) const DEFAULTS: EngravingDefaults = EngravingDefaults {{"
    );
    for (field, key) in [
        ("staff_line_thickness", "staffLineThickness"),
        ("stem_thickness", "stemThickness"),
        ("beam_thickness", "beamThickness"),
        ("beam_spacing", "beamSpacing"),
        ("leger_line_thickness", "legerLineThickness"),
        ("leger_line_extension", "legerLineExtension"),
        ("thin_barline_thickness", "thinBarlineThickness"),
        ("thick_barline_thickness", "thickBarlineThickness"),
        ("barline_separation", "barlineSeparation"),
        (
            "repeat_barline_dot_separation",
            "repeatBarlineDotSeparation",
        ),
        ("tie_endpoint_thickness", "tieEndpointThickness"),
        ("tie_midpoint_thickness", "tieMidpointThickness"),
    ] {
        let _ = writeln!(s, "    {field}: {},", d(key)?);
    }
    let _ = writeln!(s, "}};");
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "pub(super) const METRICS: [GlyphMetrics; {}] = [",
        specs.len()
    );
    for spec in specs {
        let n = spec.name;
        let ch = char::from_u32(spec.codepoint).ok_or_else(|| format!("{n}: bad codepoint"))?;
        let gid = face
            .glyph_index(ch)
            .ok_or_else(|| format!("{n} U+{:04X}: not in the font's cmap", spec.codepoint))?;
        let advance = face
            .advance(gid)
            .ok_or_else(|| format!("{n}: no advance width in hmtx"))?;
        let b = &meta["glyphBBoxes"][n];
        let sw = pair(&b["bBoxSW"]).ok_or_else(|| format!("{n}: no bBoxSW in the metadata"))?;
        let ne = pair(&b["bBoxNE"]).ok_or_else(|| format!("{n}: no bBoxNE in the metadata"))?;
        let outline = face
            .outline_box(gid)
            .ok_or_else(|| format!("{n}: no outline in the font"))?;
        let diffs = [
            (to_sp(outline.x_min) - sw.0).abs(),
            (to_sp(outline.y_min) - sw.1).abs(),
            (to_sp(outline.x_max) - ne.0).abs(),
            (to_sp(outline.y_max) - ne.1).abs(),
        ];
        if diffs.iter().any(|d| *d > BOX_TOLERANCE_SP) {
            return Err(format!(
                "{n}: metadata box differs from the outline by {diffs:?} staff spaces"
            ));
        }
        let anchors = &meta["glyphsWithAnchors"][n];
        let mut found = Vec::new();
        for a in Anchor::ALL {
            let v = pair(&anchors[a.name()]);
            if v.is_none() && spec.required.contains(&a) {
                return Err(format!("{n}: required anchor {} missing", a.name()));
            }
            found.push(match v {
                Some((x, y)) => format!("Some(({}, {}))", lit(x), lit(y)),
                None => "None".to_string(),
            });
        }
        let _ = writeln!(s, "    // {n} U+{:04X}", spec.codepoint);
        let _ = writeln!(s, "    GlyphMetrics {{");
        let _ = writeln!(s, "        glyph_id: {gid},");
        let _ = writeln!(s, "        advance: {},", lit(to_sp(advance)));
        let _ = writeln!(
            s,
            "        bbox: GlyphBox {{ sw: ({}, {}), ne: ({}, {}) }},",
            lit(sw.0),
            lit(sw.1),
            lit(ne.0),
            lit(ne.1)
        );
        let _ = writeln!(s, "        anchors: [{}],", found.join(", "));
        let _ = writeln!(s, "    }},");
    }
    let _ = writeln!(s, "];");
    Ok(s)
}

fn fresh() -> String {
    generate(JSON, OTF, &specs()).unwrap_or_else(|e| panic!("generator failed: {e}"))
}

#[test]
fn table_matches_the_bundled_font() {
    let fresh = fresh();
    let committed = COMMITTED.replace("\r\n", "\n");
    if fresh != committed {
        println!("----- BEGIN bravura.rs -----\n{fresh}----- END bravura.rs -----");
        panic!("src/smufl/bravura.rs differs from a fresh generation (printed above)");
    }
}

#[test]
#[ignore = "prints the generated table for review"]
fn print_table() {
    print!("{}", fresh());
}

#[test]
fn table_order_matches_the_enum() {
    for (i, g) in SmuflGlyph::ALL.iter().enumerate() {
        assert_eq!(*g as usize, i, "{}", g.name());
    }
    assert_eq!(super::bravura::METRICS.len(), SmuflGlyph::ALL.len());
}

#[test]
fn l7_metrics_agree_with_the_font_glyph_by_glyph() {
    let face = Face::parse(OTF).unwrap();
    let upem = face.upem;
    let meta: serde_json::Value = serde_json::from_slice(JSON).unwrap();
    for g in SmuflGlyph::ALL {
        let m = g.metrics();
        let gid = face
            .glyph_index(char::from_u32(g.codepoint()).unwrap())
            .unwrap();
        assert_eq!(m.glyph_id, gid, "{}", g.name());
        let adv = face.advance(gid).unwrap() / upem * SP_PER_EM;
        assert!((m.advance - adv).abs() < 1e-6, "{} advance", g.name());
        let o = face.outline_box(gid).unwrap();
        let sp = |v: f32| v / upem * SP_PER_EM;
        assert!(
            (sp(o.x_min) - m.bbox.sw.0).abs() <= BOX_TOLERANCE_SP,
            "{}",
            g.name()
        );
        assert!(
            (sp(o.y_max) - m.bbox.ne.1).abs() <= BOX_TOLERANCE_SP,
            "{}",
            g.name()
        );
        assert!(
            m.bbox.width() > 0.0 && m.bbox.height() > 0.0,
            "{}",
            g.name()
        );
        // Anchors exactly where the metadata has them; nothing assumed elsewhere.
        for a in Anchor::ALL {
            let has = !meta["glyphsWithAnchors"][g.name()][a.name()].is_null();
            assert_eq!(m.anchor(a).is_some(), has, "{} {}", g.name(), a.name());
        }
        for a in g.required_anchors() {
            assert!(m.anchor(*a).is_some(), "{} needs {}", g.name(), a.name());
        }
    }
    // Glyphs differ: no shared size or anchor is assumed.
    // (Half and black heads do share a box in Bravura 1.482; the whole head does not.)
    assert_ne!(
        SmuflGlyph::NoteheadBlack.metrics().bbox,
        SmuflGlyph::NoteheadWhole.metrics().bbox
    );
    assert!(
        SmuflGlyph::NoteheadWhole
            .metrics()
            .anchor(Anchor::StemUpSE)
            .is_none()
    );
}

#[test]
fn l7_a_glyph_missing_from_the_font_fails_with_its_name() {
    let mut s = specs();
    s.push(Spec {
        name: "noteheadBlack",
        codepoint: 0x10FFFD, // plane 16 private use: not in Bravura
        required: &[],
    });
    let e = generate(JSON, OTF, &s).unwrap_err();
    assert!(e.contains("noteheadBlack") && e.contains("cmap"), "{e}");
}

#[test]
fn l7_a_missing_metadata_box_or_anchor_fails_with_its_name() {
    let mut meta: serde_json::Value = serde_json::from_slice(JSON).unwrap();
    meta["glyphsWithAnchors"]["noteheadHalf"]
        .as_object_mut()
        .unwrap()
        .remove("stemUpSE");
    let e = generate(meta.to_string().as_bytes(), OTF, &specs()).unwrap_err();
    assert!(e.contains("noteheadHalf") && e.contains("stemUpSE"), "{e}");

    let mut meta: serde_json::Value = serde_json::from_slice(JSON).unwrap();
    meta["glyphBBoxes"]
        .as_object_mut()
        .unwrap()
        .remove("restQuarter");
    let e = generate(meta.to_string().as_bytes(), OTF, &specs()).unwrap_err();
    assert!(e.contains("restQuarter"), "{e}");
}

#[test]
fn l7_a_box_that_disagrees_with_the_outline_fails() {
    let mut meta: serde_json::Value = serde_json::from_slice(JSON).unwrap();
    meta["glyphBBoxes"]["gClef8vb"]["bBoxNE"][1] = serde_json::json!(99.0);
    let e = generate(meta.to_string().as_bytes(), OTF, &specs()).unwrap_err();
    assert!(e.contains("gClef8vb") && e.contains("outline"), "{e}");
}

// ----------------------------------------------------------------------- FF-35

/// (file, size, SHA-256) rows of the PROVENANCE.md file table.
fn provenance_rows(text: &str) -> Vec<(String, usize, String)> {
    text.lines()
        .filter_map(|l| {
            let cells: Vec<&str> = l.split('|').map(str::trim).collect();
            // | `file` | size | `hash` |
            if cells.len() < 4 {
                return None;
            }
            let file = cells[1].strip_prefix('`')?.strip_suffix('`')?;
            let size = cells[2].parse().ok()?;
            let hash = cells[3].strip_prefix('`')?.strip_suffix('`')?;
            (hash.len() == 64).then(|| (file.to_string(), size, hash.to_string()))
        })
        .collect()
}

fn bundled(file: &str) -> &'static [u8] {
    match file {
        "Bravura.otf" => OTF,
        "Bravura.json" => JSON,
        "OFL.txt" => OFL,
        other => panic!("PROVENANCE.md names an unknown file {other}"),
    }
}

/// Every row matches the bytes, or the list of mismatches.
fn verify(rows: &[(String, usize, String)], read: impl Fn(&str) -> Vec<u8>) -> Vec<String> {
    let mut bad = Vec::new();
    for (file, size, hash) in rows {
        let bytes = read(file);
        if bytes.len() != *size {
            bad.push(format!(
                "{file}: {} bytes, PROVENANCE.md says {size}",
                bytes.len()
            ));
        }
        if sha256_hex(&bytes) != *hash {
            bad.push(format!("{file}: SHA-256 differs from PROVENANCE.md"));
        }
    }
    bad
}

#[test]
fn ff35_bundled_bravura_matches_its_provenance() {
    let rows = provenance_rows(PROVENANCE);
    let files: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
    assert_eq!(files, ["Bravura.otf", "Bravura.json", "OFL.txt"]);
    let bad = verify(&rows, |f| bundled(f).to_vec());
    assert!(bad.is_empty(), "{bad:?}");
    // The table was generated from these very files.
    assert_eq!(super::FONT.otf_sha256, rows[0].2);
    assert_eq!(super::FONT.json_sha256, rows[1].2);
    assert_eq!(
        (super::FONT.name, super::FONT.version),
        ("Bravura", "1.482")
    );
}

#[test]
fn ff35_a_changed_byte_is_detected() {
    let rows = provenance_rows(PROVENANCE);
    for target in ["Bravura.otf", "Bravura.json"] {
        let bad = verify(&rows, |f| {
            let mut b = bundled(f).to_vec();
            if f == target {
                let i = b.len() / 2;
                b[i] ^= 0x01;
            }
            b
        });
        assert_eq!(bad.len(), 1, "{target}: {bad:?}");
        assert!(bad[0].starts_with(target));
    }
}
