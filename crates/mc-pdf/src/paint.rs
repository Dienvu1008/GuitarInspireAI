//! Painting: the paginated layout as drawing operations in page coordinates (points,
//! y down from the page top). Every position comes from the layout; this module only
//! chooses symbols, sizes and greys, following the Flutter painters (ADR-041 section
//! 12.1). It makes no layout decision.

use crate::PdfError;
use crate::fonts::{Face, FontId, Usage};
use mc_notation::{
    EngravedLineKind, GlyphKind, LineKind, Page, Pagination, ScoreSystem, Stave, StaveKind,
};

/// Grey of the ink (black).
pub(crate) const INK: f32 = 0.0;
/// Grey of staff and string lines, bar numbers and string names: the Flutter staff
/// colour, ink at 45 % opacity, on white paper.
pub(crate) const STAFF: f32 = 0.55;
/// Grey of the paper (white), behind fret numbers.
pub(crate) const PAPER: f32 = 1.0;
/// The Flutter TAB painter's reference string gap: its absolute sizes (stroke widths,
/// dot radii, rest shapes) are screen pixels at a string gap of 12.
pub(crate) const SCREEN_STRING_GAP: f32 = 12.0;
/// Title and page-number sizes (ADR-041 section 11: a 20 pt title line, 10 pt page
/// numbers).
pub(crate) const TITLE_SIZE: f32 = 20.0;
pub(crate) const FOOTER_SIZE: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Seg {
    M(f32, f32),
    L(f32, f32),
    C(f32, f32, f32, f32, f32, f32),
    Z,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Op {
    /// A straight stroke with butt caps.
    Line {
        gray: f32,
        width: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    /// A filled closed path (non-zero rule).
    Fill { gray: f32, path: Vec<Seg> },
    /// Glyphs of one font from (`x`, `y`) on the baseline, `size` points per em; the
    /// ids are those of the full font.
    Text {
        font: FontId,
        size: f32,
        gray: f32,
        x: f32,
        y: f32,
        gids: Vec<u16>,
    },
}

pub(crate) struct Painter<'a> {
    pub music: &'a Face,
    pub text: &'a Face,
    pub usage: Usage,
    /// Print sizes over screen sizes: string gap / 12.
    k: f32,
    /// TAB text size: the one at which a digit's advance is the layout's digit width.
    tab_size: f32,
}

impl<'a> Painter<'a> {
    pub fn new(music: &'a Face, text: &'a Face, p: &Pagination) -> Result<Self, PdfError> {
        let zero = text.glyph('0')?;
        let digit_em = text.advance(zero) / text.upem;
        Ok(Painter {
            music,
            text,
            usage: Usage::default(),
            k: p.metrics.string_spacing / SCREEN_STRING_GAP,
            tab_size: p.metrics.digit_width / digit_em,
        })
    }

    /// The TAB text size in points (the ADR-018 width check).
    #[cfg(test)]
    pub fn tab_size(&self) -> f32 {
        self.tab_size
    }

    /// Glyph ids and advance (points) of `s` in the text font at `size`.
    fn shape(&mut self, s: &str, size: f32) -> Result<(Vec<u16>, f32), PdfError> {
        let mut gids = Vec::with_capacity(s.len());
        let mut w = 0.0;
        for ch in s.chars() {
            let g = self.text.glyph(ch)?;
            self.usage.add(FontId::Text, g, ch);
            w += self.text.advance(g) / self.text.upem * size;
            gids.push(g);
        }
        Ok((gids, w))
    }

    /// Baseline of text whose line box (height = size, Flutter `height: 1.0`) has its
    /// top at `top`: the ascent and descent share the box in proportion.
    fn baseline(&self, top: f32, size: f32) -> f32 {
        top + size * self.text.ascent / (self.text.ascent - self.text.descent)
    }

    /// Text centred on (x, y), as the Flutter painter's `_centered`.
    fn centered(
        &mut self,
        ops: &mut Vec<Op>,
        s: &str,
        x: f32,
        y: f32,
        size: f32,
        gray: f32,
    ) -> Result<f32, PdfError> {
        let (gids, w) = self.shape(s, size)?;
        ops.push(Op::Text {
            font: FontId::Text,
            size,
            gray,
            x: x - w / 2.0,
            y: self.baseline(y - size / 2.0, size),
            gids,
        });
        Ok(w)
    }

    /// Every page's operations, in page order.
    pub fn pages(
        &mut self,
        p: &Pagination,
        systems: &[ScoreSystem],
        title: &str,
    ) -> Result<Vec<Vec<Op>>, PdfError> {
        let count = p.page_count();
        p.pages
            .iter()
            .map(|page| self.page(page, count, systems, title))
            .collect()
    }

    fn page(
        &mut self,
        page: &Page,
        count: usize,
        systems: &[ScoreSystem],
        title: &str,
    ) -> Result<Vec<Op>, PdfError> {
        let mut ops = Vec::new();
        if let Some(t) = page.title {
            // The title line is the upper half of the title area.
            let y = t.y + TITLE_SIZE / 2.0;
            self.centered(&mut ops, title, t.x + t.w / 2.0, y, TITLE_SIZE, INK)?;
        }
        let f = page.footer;
        let footer = format!("page {} of {count}", page.number);
        // The page-number line is the lower half of the footer area.
        let y = f.bottom() - FOOTER_SIZE / 2.0;
        self.centered(&mut ops, &footer, f.x + f.w / 2.0, y, FOOTER_SIZE, INK)?;
        for item in &page.items {
            let sys = &systems[item.system];
            let offset = item.y - sys.staves[item.staves.start].top;
            for st in &sys.staves[item.staves.clone()] {
                let (dx, dy) = (page.body.x, offset + st.top);
                match st.kind {
                    StaveKind::Notation => self.notation(&mut ops, st, dx, dy)?,
                    StaveKind::Tab => self.tab(&mut ops, st, dx, dy)?,
                }
            }
        }
        Ok(ops)
    }

    fn notation(
        &mut self,
        ops: &mut Vec<Op>,
        st: &Stave,
        dx: f32,
        dy: f32,
    ) -> Result<(), PdfError> {
        let n = &st.layout.notation;
        for l in &n.lines {
            ops.push(Op::Line {
                gray: if l.kind == EngravedLineKind::StaffLine {
                    STAFF
                } else {
                    INK
                },
                width: l.thickness,
                x1: dx + l.x1,
                y1: dy + l.y1,
                x2: dx + l.x2,
                y2: dy + l.y2,
            });
        }
        for b in &n.beams {
            let h = b.thickness / 2.0;
            ops.push(Op::Fill {
                gray: INK,
                path: vec![
                    Seg::M(dx + b.x1, dy + b.y1 - h),
                    Seg::L(dx + b.x2, dy + b.y2 - h),
                    Seg::L(dx + b.x2, dy + b.y2 + h),
                    Seg::L(dx + b.x1, dy + b.y1 + h),
                    Seg::Z,
                ],
            });
        }
        for t in &n.ties {
            // A crescent of two quadratic arcs (as in Flutter), written as cubics.
            let dir = if t.up { -1.0 } else { 1.0 };
            let outer = t.height * dir;
            let inner = (t.height - t.mid_thickness) * dir;
            let (x1, x2, y) = (dx + t.x1, dx + t.x2, dy + t.y);
            let mid = (x1 + x2) / 2.0;
            let mut path = vec![Seg::M(x1, y)];
            path.push(quad((x1, y), (mid, y + 2.0 * outer), (x2, y)));
            path.push(quad((x2, y), (mid, y + 2.0 * inner), (x1, y)));
            path.push(Seg::Z);
            ops.push(Op::Fill { gray: INK, path });
        }
        // SMuFL: one em is four staff spaces; the origin is on the baseline.
        let em = 4.0 * n.staff_space;
        for g in &n.glyphs {
            let ch = char::from_u32(g.glyph.codepoint()).expect("SMuFL codepoint");
            let gid = self.music.glyph(ch)?;
            self.usage.add(FontId::Music, gid, ch);
            ops.push(Op::Text {
                font: FontId::Music,
                size: em,
                gray: INK,
                x: dx + g.x,
                y: dy + g.y,
                gids: vec![gid],
            });
        }
        Ok(())
    }

    fn tab(&mut self, ops: &mut Vec<Op>, st: &Stave, dx: f32, dy: f32) -> Result<(), PdfError> {
        let k = self.k;
        let fs = self.tab_size;
        let lay = &st.layout;
        for l in &lay.lines {
            let (gray, width) = match l.kind {
                LineKind::String => (STAFF, 1.0),
                LineKind::ThickBarLine => (INK, 3.0),
                LineKind::Beam => (INK, 2.5),
                _ => (INK, 1.0),
            };
            ops.push(Op::Line {
                gray,
                width: width * k,
                x1: dx + l.x1,
                y1: dy + l.y1,
                x2: dx + l.x2,
                y2: dy + l.y2,
            });
        }
        for g in &lay.glyphs {
            let (x, y) = (dx + g.x, dy + g.y);
            match &g.kind {
                GlyphKind::Fret {
                    fret, tied, dead, ..
                } => {
                    let text = if *dead {
                        "x".to_string()
                    } else if *tied {
                        format!("({fret})")
                    } else {
                        fret.to_string()
                    };
                    let (_, w) = self.shape(&text, fs)?;
                    // Clear the string line behind the number.
                    let (cw, ch) = (w + 2.0 * k, fs * 0.8);
                    ops.push(rect(PAPER, x - cw / 2.0, y - ch / 2.0, cw, ch));
                    self.centered(ops, &text, x, y, fs, INK)?;
                }
                GlyphKind::Rest { exponent, .. } => self.rest(ops, x, y, *exponent),
                GlyphKind::Dot => ops.push(circle(x, y, 1.6 * k)),
                GlyphKind::Tuplet { actual } => {
                    self.centered(ops, &actual.to_string(), x, y, fs * 0.8, INK)?;
                }
                GlyphKind::TimeSignature {
                    numerator,
                    denominator,
                } => {
                    self.centered(ops, &numerator.to_string(), x, y - fs * 0.75, fs * 1.4, INK)?;
                    self.centered(
                        ops,
                        &denominator.to_string(),
                        x,
                        y + fs * 0.75,
                        fs * 1.4,
                        INK,
                    )?;
                }
                GlyphKind::RepeatDots => {
                    ops.push(circle(x, y - fs * 0.6, 1.8 * k));
                    ops.push(circle(x, y + fs * 0.6, 1.8 * k));
                }
                GlyphKind::RepeatCount { count } => {
                    self.centered(ops, &format!("x{count}"), x, y, fs * 0.85, INK)?;
                }
                GlyphKind::Volta { endings } => {
                    let text: Vec<String> = endings.iter().map(u8::to_string).collect();
                    self.centered(ops, &format!("{}.", text.join("., ")), x, y, fs * 0.85, INK)?;
                }
                GlyphKind::Marker { text } => {
                    self.centered(ops, text, x, y, fs * 0.9, INK)?;
                }
                GlyphKind::BarNumber { number } => {
                    self.centered(ops, &number.to_string(), x, y, fs * 0.75, STAFF)?;
                }
                GlyphKind::TabClef => {
                    for (i, c) in ["T", "A", "B"].iter().enumerate() {
                        let cy = y + (i as f32 - 1.0) * fs * 1.05;
                        self.centered(ops, c, x, cy, fs * 1.1, INK)?;
                    }
                }
                GlyphKind::StringName { name, .. } => {
                    self.centered(ops, name, x, y, fs * 0.75, STAFF)?;
                }
                GlyphKind::Accent => {
                    self.centered(ops, ">", x, y, fs * 0.9, INK)?;
                }
                GlyphKind::Stroke { up } => self.arrow(ops, x, y, *up),
                GlyphKind::StringPercussive => {
                    self.centered(ops, "S", x, y, fs * 0.75, INK)?;
                }
                GlyphKind::PalmMute { run } => self.palm_mute(ops, x, y, g.width, *run)?,
            }
        }
        Ok(())
    }

    /// Whole and half rests as blocks; shorter values as a slash with flag dots.
    fn rest(&self, ops: &mut Vec<Op>, x: f32, y: f32, exponent: u8) {
        let k = self.k;
        if exponent <= 1 {
            let top = if exponent == 0 { y - 3.0 * k } else { y };
            ops.push(rect(INK, x - 4.0 * k, top, 8.0 * k, 3.0 * k));
            return;
        }
        ops.push(line(x + 2.0 * k, y - 5.0 * k, x - 2.0 * k, y + 5.0 * k, k));
        for level in 0..exponent - 2 {
            let l = f32::from(level);
            ops.push(circle(
                x + (1.0 - l * 1.5) * k,
                y + (-4.0 + l * 3.0) * k,
                1.4 * k,
            ));
        }
    }

    /// A stroke arrow: a shaft and two head strokes.
    fn arrow(&self, ops: &mut Vec<Op>, x: f32, y: f32, up: bool) {
        let w = 1.2 * self.k;
        let h = self.tab_size * 0.45;
        let head = self.tab_size * 0.22;
        let tip = if up { y - h } else { y + h };
        ops.push(line(x, if up { y + h } else { y - h }, x, tip, w));
        let back = if up { head } else { -head };
        ops.push(line(x, tip, x - head, tip + back, w));
        ops.push(line(x, tip, x + head, tip + back, w));
    }

    /// "P.M." at the start of the box; a run continues as a dashed line with an end tick.
    fn palm_mute(
        &mut self,
        ops: &mut Vec<Op>,
        x: f32,
        y: f32,
        width: f32,
        run: bool,
    ) -> Result<(), PdfError> {
        let k = self.k;
        let size = self.tab_size * 0.7;
        if !run {
            self.centered(ops, "P.M.", x, y, size, INK)?;
            return Ok(());
        }
        let (start, end) = (x - width / 2.0, x + width / 2.0);
        let (_, w) = self.shape("P.M.", size)?;
        self.centered(ops, "P.M.", start + w / 2.0, y, size, INK)?;
        let (dash, gap) = (4.0 * k, 3.0 * k);
        let mut dx = start + w + 2.0 * k;
        while dx < end {
            ops.push(line(dx, y, (dx + dash).min(end), y, k));
            dx += dash + gap;
        }
        if start + w + 2.0 * k < end {
            ops.push(line(end, y - 3.0 * k, end, y + 3.0 * k, k));
        }
        Ok(())
    }
}

fn line(x1: f32, y1: f32, x2: f32, y2: f32, width: f32) -> Op {
    Op::Line {
        gray: INK,
        width,
        x1,
        y1,
        x2,
        y2,
    }
}

fn rect(gray: f32, x: f32, y: f32, w: f32, h: f32) -> Op {
    Op::Fill {
        gray,
        path: vec![
            Seg::M(x, y),
            Seg::L(x + w, y),
            Seg::L(x + w, y + h),
            Seg::L(x, y + h),
            Seg::Z,
        ],
    }
}

/// A filled circle as four cubic arcs.
fn circle(x: f32, y: f32, r: f32) -> Op {
    const KAPPA: f32 = 0.552_284_8;
    let c = r * KAPPA;
    Op::Fill {
        gray: INK,
        path: vec![
            Seg::M(x + r, y),
            Seg::C(x + r, y + c, x + c, y + r, x, y + r),
            Seg::C(x - c, y + r, x - r, y + c, x - r, y),
            Seg::C(x - r, y - c, x - c, y - r, x, y - r),
            Seg::C(x + c, y - r, x + r, y - c, x + r, y),
            Seg::Z,
        ],
    }
}

/// A quadratic arc from `p0` with control `q` to `p2`, as the equal cubic.
fn quad(p0: (f32, f32), q: (f32, f32), p2: (f32, f32)) -> Seg {
    let c1 = (
        p0.0 + 2.0 / 3.0 * (q.0 - p0.0),
        p0.1 + 2.0 / 3.0 * (q.1 - p0.1),
    );
    let c2 = (
        p2.0 + 2.0 / 3.0 * (q.0 - p2.0),
        p2.1 + 2.0 / 3.0 * (q.1 - p2.1),
    );
    Seg::C(c1.0, c1.1, c2.0, c2.1, p2.0, p2.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_notation::{PageSetup, ShownTrack};

    /// ADR-041 decision 1 and section 12: the TAB text size is the one at which the
    /// font's digit advance equals the layout's digit width (7 pt at print size), and
    /// every digit has that advance (a monospaced font).
    #[test]
    fn tab_digit_advance_equals_the_layout_digit_width() {
        let score = mc_score::fixtures::feature_rich();
        let shown: Vec<ShownTrack> = score.tracks().map(|t| ShownTrack::tab(t.id())).collect();
        let p = mc_notation::paginate(&score, &shown, &PageSetup::default()).unwrap();
        let (music, text) = (
            Face::new(FontId::Music).unwrap(),
            Face::new(FontId::Text).unwrap(),
        );
        let painter = Painter::new(&music, &text, &p).unwrap();
        assert!(
            (painter.tab_size() - 7.0).abs() < 1e-3,
            "{}",
            painter.tab_size()
        );
        for d in '0'..='9' {
            let w = text.advance(text.glyph(d).unwrap()) / text.upem * painter.tab_size();
            assert!((w - p.metrics.digit_width).abs() < 1e-3, "{d}: {w}");
        }
        assert!((painter.k - 0.6).abs() < 1e-6);
    }
}
