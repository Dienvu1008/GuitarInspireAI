//! Layout model to SVG (the rendering stand-in for N5b, the review images for N4, and
//! the input of the SVG-to-PDF route). Painting only: every position comes from the
//! layout. Glyphs are text in the Bravura font at 4 staff spaces per em.

use crate::model::*;
use crate::smufl::Font;
use std::fmt::Write;

/// `font_url`: how the SVG refers to the font for viewing (a file URL in the review
/// HTML); `None` names the family only (the SVG-to-PDF route supplies the font).
pub fn render(layout: &Layout, font_url: Option<&str>) -> String {
    let sp = layout.config.staff_space;
    let mut s = String::with_capacity(64 * 1024);
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.2}" height="{h:.2}" viewBox="0 0 {w:.2} {h:.2}">"#,
        w = layout.width,
        h = layout.height
    );
    if let Some(url) = font_url {
        let _ = writeln!(s, r#"<style>@font-face {{ font-family: Bravura; src: url("{url}"); }}</style>"#);
    }
    let _ = writeln!(s, r#"<rect width="100%" height="100%" fill="white"/>"#);
    for sys in &layout.systems {
        for st in &sys.staves {
            let oy = sys.y + st.position.top;
            let _ = writeln!(s, r#"<g transform="translate(0 {oy:.3})">"#);
            for l in &st.lines {
                let _ = writeln!(
                    s,
                    r#"<line x1="{:.3}" y1="{:.3}" x2="{:.3}" y2="{:.3}" stroke="black" stroke-width="{:.3}"/>"#,
                    l.x1, l.y1, l.x2, l.y2, l.thickness
                );
            }
            for b in &st.beams {
                let h = b.thickness / 2.0;
                let _ = writeln!(
                    s,
                    r#"<polygon points="{:.3},{:.3} {:.3},{:.3} {:.3},{:.3} {:.3},{:.3}" fill="black"/>"#,
                    b.x1, b.y1 - h, b.x2, b.y2 - h, b.x2, b.y2 + h, b.x1, b.y1 + h
                );
            }
            for t in &st.ties {
                let bulge = if t.up { -0.7 * sp } else { 0.7 * sp };
                let mid = (t.x1 + t.x2) / 2.0;
                let _ = writeln!(
                    s,
                    r#"<path d="M {:.3} {:.3} Q {:.3} {:.3} {:.3} {:.3} Q {:.3} {:.3} {:.3} {:.3} Z" fill="black"/>"#,
                    t.x1, t.y, mid, t.y + bulge, t.x2, t.y, mid, t.y + bulge * 0.75, t.x1, t.y
                );
            }
            for g in &st.glyphs {
                let ch = char::from_u32(Font::codepoint(g.name)).unwrap();
                let _ = writeln!(
                    s,
                    r#"<text x="{:.3}" y="{:.3}" font-family="Bravura" font-size="{:.3}">&#x{:X};</text>"#,
                    g.x,
                    g.y,
                    4.0 * sp,
                    ch as u32
                );
            }
            for t in &st.texts {
                if let Some(k) = t.knockout {
                    let _ = writeln!(
                        s,
                        r#"<rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="white"/>"#,
                        k.x, k.y, k.w, k.h
                    );
                }
                let _ = writeln!(
                    s,
                    r#"<text x="{:.3}" y="{:.3}" font-family="Consolas, monospace" font-size="{:.3}" text-anchor="middle" dominant-baseline="central">{}</text>"#,
                    t.x, t.y, t.size, t.text
                );
            }
            let _ = writeln!(s, "</g>");
        }
    }
    let _ = writeln!(s, "</svg>");
    s
}
