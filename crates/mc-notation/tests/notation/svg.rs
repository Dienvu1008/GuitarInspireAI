//! Test support, not shipped: a canonical text dump of a layout (L10 goldens) and an
//! SVG painter for review by eye (L12). Painting only; every position comes from the
//! layout.

use mc_notation::{GlyphKind, LayoutMetrics, ScoreLayout, StaveKind};
use std::fmt::Write;

fn f(v: f32) -> String {
    format!("{v:.3}")
}

/// Every item of every stave, in order, with fixed float formatting.
pub fn canonical(l: &ScoreLayout) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "layout width {} height {}", f(l.width), f(l.height));
    for sys in &l.systems {
        let _ = writeln!(s, "system {} y {} h {}", sys.index, f(sys.y), f(sys.height));
        for st in &sys.staves {
            let lay = &st.layout;
            let _ = writeln!(
                s,
                " stave {:?} track {} top {} h {}",
                st.kind,
                st.track.raw(),
                f(st.top),
                f(lay.height)
            );
            for b in &lay.bars {
                let _ = writeln!(s, "  bar {} x {} w {}", b.bar, f(b.x), f(b.width));
            }
            match st.kind {
                StaveKind::Tab => {
                    for g in &lay.glyphs {
                        if let GlyphKind::Fret { fret, tied, .. } = g.kind {
                            let _ = writeln!(
                                s,
                                "  fret {fret}{} {} {}",
                                if tied { "t" } else { "" },
                                f(g.x),
                                f(g.y)
                            );
                        }
                    }
                }
                StaveKind::Notation => {
                    let n = &lay.notation;
                    for x in &n.lines {
                        let _ = writeln!(
                            s,
                            "  line {:?} {} {} {} {} {}",
                            x.kind,
                            f(x.x1),
                            f(x.y1),
                            f(x.x2),
                            f(x.y2),
                            f(x.thickness)
                        );
                    }
                    for g in &n.glyphs {
                        let _ = writeln!(s, "  glyph {} {} {}", g.glyph.name(), f(g.x), f(g.y));
                    }
                    for b in &n.beams {
                        let _ = writeln!(
                            s,
                            "  beam {} {} {} {} {}",
                            f(b.x1),
                            f(b.y1),
                            f(b.x2),
                            f(b.y2),
                            f(b.thickness)
                        );
                    }
                    for t in &n.ties {
                        let _ = writeln!(s, "  tie {} {} {} {}", f(t.x1), f(t.x2), f(t.y), t.up);
                    }
                    for x in &n.facts.notes {
                        let _ = writeln!(
                            s,
                            "  note {}:{}:{} {} pos {} acc {:?} {:?} displaced {}",
                            x.bar,
                            x.voice,
                            x.beat,
                            x.written,
                            x.position,
                            x.accidental.map(|a| a.semitones()),
                            x.tie,
                            x.displaced
                        );
                    }
                }
            }
        }
    }
    s
}

/// The default metrics scaled by `k` (review images at 2x).
pub fn scaled(k: f32) -> LayoutMetrics {
    let m = LayoutMetrics::default();
    LayoutMetrics {
        width: m.width * k,
        margin_left: m.margin_left * k,
        margin_right: m.margin_right * k,
        string_spacing: m.string_spacing * k,
        digit_width: m.digit_width * k,
        glyph_padding: m.glyph_padding * k,
        min_column_width: m.min_column_width * k,
        duration_spacing: m.duration_spacing,
        bar_padding: m.bar_padding * k,
        empty_bar_width: m.empty_bar_width * k,
        header_width: m.header_width * k,
        time_signature_width: m.time_signature_width * k,
        repeat_width: m.repeat_width * k,
        above_staff: m.above_staff * k,
        below_staff: m.below_staff * k,
        system_gap: m.system_gap * k,
        stem_length: m.stem_length * k,
    }
}

pub fn render(l: &ScoreLayout) -> String {
    let font = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/ui_kit/fonts/bravura/Bravura.otf"
    )
    .replace('\\', "/");
    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.0}" height="{h:.0}" viewBox="0 0 {w:.1} {h:.1}">"#,
        w = l.width,
        h = l.height
    );
    let _ = writeln!(
        s,
        r#"<style>@font-face {{ font-family: Bravura; src: url("file:///{font}"); }}</style>"#
    );
    let _ = writeln!(s, r#"<rect width="100%" height="100%" fill="white"/>"#);
    for sys in &l.systems {
        for st in &sys.staves {
            let lay = &st.layout;
            let _ = writeln!(s, r#"<g transform="translate(0 {:.2})">"#, sys.y + st.top);
            match st.kind {
                StaveKind::Notation => {
                    let n = &lay.notation;
                    for x in &n.lines {
                        let _ = writeln!(
                            s,
                            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="black" stroke-width="{:.2}"/>"#,
                            x.x1, x.y1, x.x2, x.y2, x.thickness
                        );
                    }
                    for b in &n.beams {
                        let h = b.thickness / 2.0;
                        let _ = writeln!(
                            s,
                            r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}"/>"#,
                            b.x1,
                            b.y1 - h,
                            b.x2,
                            b.y2 - h,
                            b.x2,
                            b.y2 + h,
                            b.x1,
                            b.y1 + h
                        );
                    }
                    for t in &n.ties {
                        let d = if t.up { -t.height } else { t.height };
                        let m = (t.x1 + t.x2) / 2.0;
                        let _ = writeln!(
                            s,
                            r#"<path d="M {:.2} {:.2} Q {:.2} {:.2} {:.2} {:.2} Q {:.2} {:.2} {:.2} {:.2} Z"/>"#,
                            t.x1,
                            t.y,
                            m,
                            t.y + 2.0 * d,
                            t.x2,
                            t.y,
                            m,
                            t.y + 2.0 * d - d * 0.5,
                            t.x1,
                            t.y
                        );
                    }
                    for g in &n.glyphs {
                        let _ = writeln!(
                            s,
                            r#"<text x="{:.2}" y="{:.2}" font-family="Bravura" font-size="{:.2}">&#x{:X};</text>"#,
                            g.x,
                            g.y,
                            4.0 * n.staff_space,
                            g.glyph.codepoint()
                        );
                    }
                }
                StaveKind::Tab => {
                    for x in &lay.lines {
                        let w = if matches!(
                            x.kind,
                            mc_notation::LineKind::ThickBarLine | mc_notation::LineKind::Beam
                        ) {
                            3.0
                        } else {
                            1.0
                        };
                        let _ = writeln!(
                            s,
                            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="black" stroke-width="{w}"/>"#,
                            x.x1, x.y1, x.x2, x.y2
                        );
                    }
                    for g in &lay.glyphs {
                        let text = match &g.kind {
                            GlyphKind::Fret { fret, tied, .. } => {
                                if *tied {
                                    format!("({fret})")
                                } else {
                                    fret.to_string()
                                }
                            }
                            GlyphKind::TabClef => "TAB".into(),
                            GlyphKind::TimeSignature {
                                numerator,
                                denominator,
                            } => format!("{numerator}/{denominator}"),
                            GlyphKind::Rest { .. } => "r".into(),
                            GlyphKind::Tuplet { actual } => actual.to_string(),
                            _ => continue,
                        };
                        let fs = g.width.max(10.0) / text.len().max(1) as f32 * 1.4;
                        let _ = writeln!(
                            s,
                            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="white"/><text x="{:.2}" y="{:.2}" font-family="Consolas" font-size="{fs:.1}" text-anchor="middle" dominant-baseline="central">{text}</text>"#,
                            g.x - g.width / 2.0,
                            g.y - fs * 0.45,
                            g.width,
                            fs * 0.9,
                            g.x,
                            g.y
                        );
                    }
                }
            }
            let _ = writeln!(s, "</g>");
        }
    }
    let _ = writeln!(s, "</svg>");
    s
}
