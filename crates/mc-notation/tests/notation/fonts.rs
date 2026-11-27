//! PG6 font probe (ADR-041 section 11): can the two bundled fonts be subset for PDF
//! embedding, with every kept glyph drawing the same outline as in the full font?
//! A probe for the 6c-5b decision, not the export. `subsetter` runs without its
//! `variable-fonts` feature, so the variable Noto Sans Mono is subset at its default
//! instance (the outlines stored in `glyf`).

use crate::{layout, samples};
use mc_notation::{LayoutMetrics, StaveKind};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::*;
use subsetter::GlyphRemapper;

const BRAVURA: &[u8] = include_bytes!("../../../../packages/ui_kit/fonts/bravura/Bravura.otf");
const NOTO_MONO: &[u8] =
    include_bytes!("../../../../packages/ui_kit/fonts/NotoSansMono-Variable.ttf");

/// Records every path command, coordinates rounded to 1/1000 font unit.
#[derive(Default, PartialEq, Debug)]
struct Recorder(Vec<(char, Vec<i64>)>);

impl Recorder {
    fn push(&mut self, c: char, v: &[f32]) {
        self.0
            .push((c, v.iter().map(|x| (x * 1000.0).round() as i64).collect()));
    }
}

impl OutlinePen for Recorder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.push('M', &[x, y]);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.push('L', &[x, y]);
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.push('Q', &[cx0, cy0, x, y]);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.push('C', &[cx0, cy0, cx1, cy1, x, y]);
    }
    fn close(&mut self) {
        self.push('Z', &[]);
    }
}

fn outline(font: &FontRef, gid: u16) -> Recorder {
    let glyph = font
        .outline_glyphs()
        .get(GlyphId::new(gid as u32))
        .unwrap_or_else(|| panic!("glyph {gid} has no outline"));
    let mut pen = Recorder::default();
    glyph
        .draw(
            DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
            &mut pen,
        )
        .expect("draws");
    pen
}

fn advance(font: &FontRef, gid: u16) -> Option<f32> {
    font.glyph_metrics(Size::unscaled(), LocationRef::default())
        .advance_width(GlyphId::new(gid as u32))
}

struct Probe {
    glyphs: usize,
    full: usize,
    subset: usize,
    commands: usize,
}

/// Subsets `data` to the glyphs of `chars` and compares each kept glyph's outline and
/// advance with the full font's.
fn probe(name: &str, data: &[u8], chars: &[char]) -> Probe {
    let full = FontRef::new(data).expect("parses");
    let gids: Vec<u16> = chars
        .iter()
        .map(|c| {
            let g = full
                .charmap()
                .map(*c)
                .unwrap_or_else(|| panic!("{name}: no glyph for {c:?} (U+{:04X})", *c as u32));
            u16::try_from(g.to_u32()).expect("16-bit glyph id")
        })
        .collect();
    let mut remapper = GlyphRemapper::new();
    for g in &gids {
        remapper.remap(*g);
    }
    let bytes = subsetter::subset(data, 0, &remapper).unwrap_or_else(|e| panic!("{name}: {e}"));
    let sub = FontRef::new(&bytes).unwrap_or_else(|e| panic!("{name}: subset parses: {e}"));
    let mut commands = 0;
    for (c, g) in chars.iter().zip(&gids) {
        let new = remapper.get(*g).expect("remapped");
        let (a, b) = (outline(&full, *g), outline(&sub, new));
        assert!(
            !a.0.is_empty() || c.is_whitespace(),
            "{name}: {c:?} has an empty outline"
        );
        assert_eq!(a, b, "{name}: {c:?} outline differs in the subset");
        assert_eq!(
            advance(&full, *g),
            advance(&sub, new),
            "{name}: {c:?} advance"
        );
        commands += a.0.len();
    }
    Probe {
        glyphs: remapper.num_gids() as usize,
        full: data.len(),
        subset: bytes.len(),
        commands,
    }
}

/// The Bravura characters the `feature_rich` notation draws.
fn feature_rich_music_chars() -> Vec<char> {
    let l = layout(&samples::all()[6].1, &LayoutMetrics::default());
    let mut cps: Vec<u32> = l
        .systems
        .iter()
        .flat_map(|s| &s.staves)
        .filter(|s| s.kind == StaveKind::Notation)
        .flat_map(|s| &s.layout.notation.glyphs)
        .map(|g| g.glyph.codepoint())
        .collect();
    cps.sort_unstable();
    cps.dedup();
    cps.into_iter()
        .map(|c| char::from_u32(c).expect("codepoint"))
        .collect()
}

#[test]
fn pg6_bravura_and_noto_sans_mono_subset_with_identical_outlines() {
    assert_eq!(samples::all()[6].0, "feature_rich");
    let music = feature_rich_music_chars();
    assert!(music.len() >= 10, "{} music glyphs", music.len());
    let b = probe("Bravura", BRAVURA, &music);
    // TAB text: digits, parentheses for tied frets, "TAB", and the footer's
    // "page n of N" letters.
    let text: Vec<char> = "0123456789()TABpageof".chars().collect();
    let n = probe("Noto Sans Mono", NOTO_MONO, &text);
    // The variable font: its axes, and the default instance is what was kept.
    let full = FontRef::new(NOTO_MONO).unwrap();
    let axes: Vec<String> = full
        .axes()
        .iter()
        .map(|a| {
            format!(
                "{} {}..{} default {}",
                a.tag(),
                a.min_value(),
                a.max_value(),
                a.default_value()
            )
        })
        .collect();
    assert!(!axes.is_empty(), "Noto Sans Mono is a variable font");
    let sub_axes = {
        let mut remapper = GlyphRemapper::new();
        remapper.remap(full.charmap().map('0').unwrap().to_u32() as u16);
        let bytes = subsetter::subset(NOTO_MONO, 0, &remapper).unwrap();
        FontRef::new(&bytes).unwrap().axes().len()
    };
    println!(
        "PG6 Bravura: {} glyphs kept (with .notdef), {} commands, {} -> {} bytes",
        b.glyphs, b.commands, b.full, b.subset
    );
    println!(
        "PG6 Noto Sans Mono: {} glyphs kept, {} commands, {} -> {} bytes; axes {axes:?}; axes in the subset: {sub_axes}",
        n.glyphs, n.commands, n.full, n.subset
    );
    assert!(b.subset < b.full / 10 && n.subset < n.full / 10);
}
