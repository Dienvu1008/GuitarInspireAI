//! N6: two PDF routes from the same layout model.
//! - Vector: the layout model straight to PDF with `pdf-writer`; Bravura glyphs as text
//!   of an embedded font subset (`subsetter`, OFL FAQ 1.12 allows a subset in a
//!   document); lines, beams and ties as paths; TAB digits in Helvetica (a standard
//!   PDF font, not embedded).
//! - SVG-to-PDF: our SVG converted by `svg2pdf` (text embedded as text).
//! No creation date is written, so both are deterministic (N8).

use crate::model::*;
use crate::smufl::Font;
use pdf_writer::{Content, Finish, Name, Pdf, Rect as PRect, Ref, Str};
use std::collections::BTreeMap;

/// Points per layout unit: 1 000 layout units to 600 pt.
pub const K: f32 = 0.6;

thread_local! {
    static COMPRESS: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// Whether the vector route compresses its content stream (off for the read-back).
pub fn set_compress(on: bool) {
    COMPRESS.with(|c| c.set(on));
}

fn compress() -> bool {
    COMPRESS.with(|c| c.get())
}

pub fn vector(layout: &Layout, font: &Font) -> Vec<u8> {
    let face = ttf_parser::Face::parse(&font.otf, 0).expect("font");
    let upem = face.units_per_em() as f32;
    // Glyphs used, in a fixed order (N8): old glyph id per codepoint.
    let mut remapper = subsetter::GlyphRemapper::new();
    let mut new_gid: BTreeMap<&str, u16> = BTreeMap::new();
    for sys in &layout.systems {
        for st in &sys.staves {
            for g in &st.glyphs {
                new_gid.entry(g.name).or_insert_with(|| {
                    let ch = char::from_u32(Font::codepoint(g.name)).unwrap();
                    let old = face.glyph_index(ch).expect("glyph").0;
                    remapper.remap(old)
                });
            }
        }
    }
    let subset = subsetter::subset(&font.otf, 0, &remapper).expect("subset");
    let widths: Vec<f32> = remapper
        .remapped_gids()
        .map(|old| face.glyph_hor_advance(ttf_parser::GlyphId(old)).unwrap_or(0) as f32 * 1000.0 / upem)
        .collect();

    let (w, h) = (layout.width * K, layout.height * K);
    let catalog = Ref::new(1);
    let pages = Ref::new(2);
    let page = Ref::new(3);
    let content_id = Ref::new(4);
    let music = Ref::new(5);
    let cid = Ref::new(6);
    let descriptor = Ref::new(7);
    let file = Ref::new(8);
    let helvetica = Ref::new(9);
    let mut pdf = Pdf::new();
    pdf.catalog(catalog).pages(pages);
    pdf.pages(pages).kids([page]).count(1);
    {
        let mut p = pdf.page(page);
        p.media_box(PRect::new(0.0, 0.0, w, h));
        p.parent(pages);
        p.contents(content_id);
        let mut res = p.resources();
        res.fonts().pair(Name(b"M"), music).pair(Name(b"H"), helvetica);
        res.finish();
        p.finish();
    }
    let mut c = Content::new();
    let fy = |y: f32| h - y * K;
    let sp = layout.config.staff_space;
    for sys in &layout.systems {
        for st in &sys.staves {
            let oy = sys.y + st.position.top;
            for l in &st.lines {
                c.set_line_width(l.thickness * K);
                c.move_to(l.x1 * K, fy(oy + l.y1));
                c.line_to(l.x2 * K, fy(oy + l.y2));
                c.stroke();
            }
            for b in &st.beams {
                let t = b.thickness / 2.0;
                c.move_to(b.x1 * K, fy(oy + b.y1 - t));
                c.line_to(b.x2 * K, fy(oy + b.y2 - t));
                c.line_to(b.x2 * K, fy(oy + b.y2 + t));
                c.line_to(b.x1 * K, fy(oy + b.y1 + t));
                c.close_path();
                c.fill_nonzero();
            }
            for t in &st.ties {
                // The SVG's two quadratic curves, as cubics.
                let bulge = if t.up { -0.7 * sp } else { 0.7 * sp };
                let mid = (t.x1 + t.x2) / 2.0;
                let q = |p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), c: &mut Content| {
                    let c1 = (p0.0 + 2.0 / 3.0 * (p1.0 - p0.0), p0.1 + 2.0 / 3.0 * (p1.1 - p0.1));
                    let c2 = (p2.0 + 2.0 / 3.0 * (p1.0 - p2.0), p2.1 + 2.0 / 3.0 * (p1.1 - p2.1));
                    c.cubic_to(c1.0 * K, fy(oy + c1.1), c2.0 * K, fy(oy + c2.1), p2.0 * K, fy(oy + p2.1));
                };
                c.move_to(t.x1 * K, fy(oy + t.y));
                q((t.x1, t.y), (mid, t.y + bulge), (t.x2, t.y), &mut c);
                q((t.x2, t.y), (mid, t.y + bulge * 0.75), (t.x1, t.y), &mut c);
                c.close_path();
                c.fill_nonzero();
            }
            // Knockouts from the layout, white, before any text.
            if st.texts.iter().any(|t| t.knockout.is_some()) {
                c.save_state();
                c.set_fill_gray(1.0);
                for k in st.texts.iter().filter_map(|t| t.knockout) {
                    c.rect(k.x * K, fy(oy + k.y + k.h), k.w * K, k.h * K);
                }
                c.fill_nonzero();
                c.restore_state();
            }
            c.begin_text();
            let size = 4.0 * sp * K;
            c.set_font(Name(b"M"), 1.0);
            for g in &st.glyphs {
                c.set_text_matrix([size, 0.0, 0.0, size, g.x * K, fy(oy + g.y)]);
                let gid = new_gid[g.name];
                c.show(Str(&gid.to_be_bytes()));
            }
            c.set_font(Name(b"H"), 1.0);
            for t in &st.texts {
                let size = t.size * K;
                // Helvetica digits and capitals: 556 and about 640 units per 1 000.
                let width: f32 = t.text.chars().map(|ch| if ch.is_ascii_digit() { 0.556 } else if ch == '(' || ch == ')' { 0.333 } else { 0.64 }).sum::<f32>() * size;
                c.set_text_matrix([size, 0.0, 0.0, size, t.x * K - width / 2.0, fy(oy + t.y) - size * 0.35]);
                c.show(Str(t.text.as_bytes()));
            }
            c.end_text();
        }
    }
    let raw = c.finish();
    if compress() {
        let packed = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
        pdf.stream(content_id, &packed).filter(pdf_writer::Filter::FlateDecode);
    } else {
        pdf.stream(content_id, &raw);
    }
    let base = Name(b"SPIKES+Bravura");
    pdf.type0_font(music)
        .base_font(base)
        .encoding_predefined(Name(b"Identity-H"))
        .descendant_font(cid);
    {
        let mut f = pdf.cid_font(cid);
        f.subtype(pdf_writer::types::CidFontType::Type0);
        f.base_font(base);
        f.system_info(pdf_writer::types::SystemInfo {
            registry: Str(b"Adobe"),
            ordering: Str(b"Identity"),
            supplement: 0,
        });
        f.font_descriptor(descriptor);
        f.default_width(0.0);
        f.widths().consecutive(0, widths.iter().copied());
        f.finish();
    }
    let bb = face.global_bounding_box();
    let s = 1000.0 / upem;
    pdf.font_descriptor(descriptor)
        .name(base)
        .flags(pdf_writer::types::FontFlags::SYMBOLIC)
        .bbox(PRect::new(bb.x_min as f32 * s, bb.y_min as f32 * s, bb.x_max as f32 * s, bb.y_max as f32 * s))
        .italic_angle(0.0)
        .ascent(face.ascender() as f32 * s)
        .descent(face.descender() as f32 * s)
        .cap_height(face.capital_height().unwrap_or(0) as f32 * s)
        .stem_v(80.0)
        .font_file3(file);
    pdf.stream(file, &subset).pair(Name(b"Subtype"), Name(b"OpenType"));
    pdf.type1_font(helvetica).base_font(Name(b"Helvetica"));
    pdf.finish()
}

/// The SVG-to-PDF route. `compress` false keeps the content readable for the
/// fidelity check.
pub fn from_svg(svg: &str, font: &Font, compress: bool) -> Vec<u8> {
    use svg2pdf::usvg;
    let mut opts = usvg::Options::default();
    opts.fontdb_mut().load_font_data(font.otf.clone());
    opts.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_str(svg, &opts).expect("svg");
    let conv = svg2pdf::ConversionOptions {
        compress,
        ..svg2pdf::ConversionOptions::default()
    };
    // 72 dpi: one SVG unit is one point; scale the page like the vector route.
    let page = svg2pdf::PageOptions { dpi: 72.0 / K };
    svg2pdf::to_pdf(&tree, conv, page).expect("svg2pdf")
}
