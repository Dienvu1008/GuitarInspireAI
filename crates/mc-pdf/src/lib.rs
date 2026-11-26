//! PDF export (`mc-pdf`, ADR-041 decision 5 and section 12): the paginated notation
//! layout written as vector PDF with the two bundled fonts embedded.
//!
//! - Pagination and layout come from `mc-notation` (`paginate`, `layout_score_systems`);
//!   this crate only paints (GR-05).
//! - Fonts: Bravura and Noto Sans Mono only, subset and embedded as CID fonts with a
//!   ToUnicode map. A character missing from a font is an error ([`PdfError::MissingGlyph`]).
//! - Deterministic: no date, and the document ID is a hash of the content, so equal
//!   input gives equal bytes in any process or build.
//! - Returns bytes; writing the file is the caller's (CLI or app).

mod fonts;
mod paint;

pub use fonts::FontId;
use fonts::{Face, Subset};
use mc_notation::{NotationError, PageSetup, ShownTrack};
use mc_score::Score;
use paint::{Op, Painter, Seg};
use pdf_writer::{Content, Filter, Finish, Pdf, Rect, Ref, Str, TextStr};
use std::fmt;

/// The producer recorded in the document information.
const PRODUCER: &str = "GuitarInspireAI";

#[derive(Debug, Clone, PartialEq)]
pub enum PdfError {
    Layout(NotationError),
    /// A character to be shown is not in the font. Nothing is substituted.
    MissingGlyph {
        font: &'static str,
        ch: char,
    },
    /// A bundled font could not be read or subset.
    Font(String),
}

impl fmt::Display for PdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout(e) => write!(f, "{e}"),
            Self::MissingGlyph { font, ch } => write!(
                f,
                "the character '{ch}' (U+{:04X}) is not in the font {font}",
                *ch as u32
            ),
            Self::Font(m) => write!(f, "font error: {m}"),
        }
    }
}

impl std::error::Error for PdfError {}

impl From<NotationError> for PdfError {
    fn from(e: NotationError) -> Self {
        PdfError::Layout(e)
    }
}

/// An embedded font subset (for the report and the user's Edge check).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedFont {
    pub font: FontId,
    /// Subset name, for example `ABCDEF+Bravura`.
    pub name: String,
    /// Glyphs kept, without `.notdef`.
    pub glyphs: usize,
    /// Size of the subset font file before compression.
    pub bytes: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PdfExport {
    pub bytes: Vec<u8>,
    pub pages: usize,
    pub fonts: Vec<EmbeddedFont>,
}

/// Exports the shown tracks of `score` (what the view shows) on pages of `setup`,
/// with `title` on page 1.
pub fn export_pdf(
    score: &Score,
    shown: &[ShownTrack],
    setup: &PageSetup,
    title: &str,
) -> Result<PdfExport, PdfError> {
    let p = mc_notation::paginate(score, shown, setup)?;
    let systems = mc_notation::layout_score_systems(
        score,
        &p.outline,
        0..p.outline.systems.len(),
        &p.metrics,
    )?;
    let music = Face::new(FontId::Music)?;
    let text = Face::new(FontId::Text)?;
    let mut painter = Painter::new(&music, &text, &p)?;
    let pages = painter.pages(&p, &systems, title)?;
    let subsets: Vec<Subset> = painter
        .usage
        .glyphs
        .iter()
        .map(|(id, chars)| Subset::new(face_of(*id, &music, &text), chars))
        .collect::<Result<_, _>>()?;

    let contents: Vec<Vec<u8>> = pages
        .iter()
        .map(|ops| content(ops, setup, &subsets))
        .collect();

    let mut id = 0;
    let mut next = || {
        id += 1;
        Ref::new(id)
    };
    let (catalog, tree, info) = (next(), next(), next());
    let page_refs: Vec<(Ref, Ref)> = contents.iter().map(|_| (next(), next())).collect();
    let font_refs: Vec<Ref> = subsets.iter().map(|_| next()).collect();

    let mut pdf = Pdf::new();
    pdf.catalog(catalog).pages(tree);
    pdf.pages(tree)
        .kids(page_refs.iter().map(|(p, _)| *p))
        .count(page_refs.len() as i32);
    pdf.document_info(info)
        .title(TextStr(title))
        .producer(TextStr(PRODUCER));
    let media = Rect::new(0.0, 0.0, setup.size.width, setup.size.height);
    let mut hash = Fnv::new(0);
    hash.write(title.as_bytes());
    for ((page, stream), data) in page_refs.iter().zip(&contents) {
        hash.write(data);
        let mut pg = pdf.page(*page);
        pg.parent(tree).media_box(media).contents(*stream);
        let mut resources = pg.resources();
        let mut fonts = resources.fonts();
        for (s, r) in subsets.iter().zip(&font_refs) {
            fonts.pair(s.id.resource(), *r);
        }
        fonts.finish();
        resources.finish();
        pg.finish();
        let packed = miniz_oxide::deflate::compress_to_vec_zlib(data, 6);
        pdf.stream(*stream, &packed).filter(Filter::FlateDecode);
    }
    let mut report = Vec::new();
    for (s, r) in subsets.iter().zip(&font_refs) {
        hash.write(&s.bytes);
        s.write(face_of(s.id, &music, &text), &mut pdf, *r, &mut next);
        report.push(EmbeddedFont {
            font: s.id,
            name: s.name(),
            glyphs: s.chars.len(),
            bytes: s.bytes.len(),
        });
    }
    let mut file_id = hash.value().to_be_bytes().to_vec();
    file_id.extend(Fnv::from(&hash, 1).to_be_bytes());
    pdf.set_file_id((file_id.clone(), file_id));
    Ok(PdfExport {
        bytes: pdf.finish(),
        pages: contents.len(),
        fonts: report,
    })
}

fn face_of<'a>(id: FontId, music: &'a Face, text: &'a Face) -> &'a Face {
    match id {
        FontId::Music => music,
        FontId::Text => text,
    }
}

/// One page's content stream. The page is flipped once (y down, as the layout); text
/// is flipped back by its text matrix.
fn content(ops: &[Op], setup: &PageSetup, subsets: &[Subset]) -> Vec<u8> {
    let mut c = Content::new();
    c.transform([1.0, 0.0, 0.0, -1.0, 0.0, setup.size.height]);
    let (mut stroke, mut fill, mut width) = (None, None, None);
    for op in ops {
        match op {
            Op::Line {
                gray,
                width: w,
                x1,
                y1,
                x2,
                y2,
            } => {
                if stroke != Some(*gray) {
                    c.set_stroke_gray(*gray);
                    stroke = Some(*gray);
                }
                if width != Some(*w) {
                    c.set_line_width(*w);
                    width = Some(*w);
                }
                c.move_to(*x1, *y1).line_to(*x2, *y2).stroke();
            }
            Op::Fill { gray, path } => {
                if fill != Some(*gray) {
                    c.set_fill_gray(*gray);
                    fill = Some(*gray);
                }
                for s in path {
                    match *s {
                        Seg::M(x, y) => c.move_to(x, y),
                        Seg::L(x, y) => c.line_to(x, y),
                        Seg::C(a, b, d, e, x, y) => c.cubic_to(a, b, d, e, x, y),
                        Seg::Z => c.close_path(),
                    };
                }
                c.fill_nonzero();
            }
            Op::Text {
                font,
                size,
                gray,
                x,
                y,
                gids,
            } => {
                if fill != Some(*gray) {
                    c.set_fill_gray(*gray);
                    fill = Some(*gray);
                }
                let s = subsets.iter().find(|s| s.id == *font).expect("used font");
                let cids: Vec<u8> = gids.iter().flat_map(|g| s.cid(*g).to_be_bytes()).collect();
                c.begin_text();
                c.set_font(font.resource(), 1.0);
                c.set_text_matrix([*size, 0.0, 0.0, -*size, *x, *y]);
                c.show(Str(&cids));
                c.end_text();
            }
        }
    }
    c.finish().to_vec()
}

/// FNV-1a, 64 bits: a fixed, portable hash for the subset tags and the document ID.
pub(crate) struct Fnv(u64);

impl Fnv {
    pub fn new(seed: u64) -> Self {
        Fnv(0xcbf2_9ce4_8422_2325 ^ seed)
    }

    fn from(other: &Fnv, seed: u64) -> u64 {
        let mut h = Fnv(other.0 ^ seed);
        h.write(b"second");
        h.value()
    }

    pub fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}
