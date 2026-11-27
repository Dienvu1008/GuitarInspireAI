//! The two bundled fonts (ADR-041 decision 5): looked up, subset and embedded as CID
//! fonts. Nothing else is ever loaded: no system font, no standard-14 font, and a
//! character a font lacks is an error, never a substitute.

use crate::PdfError;
use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Chunk, Filter, Finish, Name, Rect, Ref, Str};
use skrifa::prelude::*;
use std::collections::BTreeMap;
use subsetter::GlyphRemapper;

/// Bravura 1.482, bundled unmodified (ADR-037).
pub(crate) const BRAVURA: &[u8] =
    include_bytes!("../../../packages/ui_kit/fonts/bravura/Bravura.otf");
/// Noto Sans Mono, the TAB font (ADR-021). Variable; subset at its default instance
/// (ADR-041 section 11.1, PG6).
pub(crate) const NOTO_SANS_MONO: &[u8] =
    include_bytes!("../../../packages/ui_kit/fonts/NotoSansMono-Variable.ttf");

/// Which of the two fonts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FontId {
    /// Bravura: notation glyphs.
    Music,
    /// Noto Sans Mono: TAB text, the title and page numbers.
    Text,
}

impl FontId {
    /// The PostScript name embedded after the subset tag.
    pub fn base_name(self) -> &'static str {
        match self {
            FontId::Music => "Bravura",
            FontId::Text => "NotoSansMono",
        }
    }

    /// Resource name in page content streams.
    pub(crate) fn resource(self) -> Name<'static> {
        match self {
            FontId::Music => Name(b"F0"),
            FontId::Text => Name(b"F1"),
        }
    }
}

/// One font: parsed once, queried in font units.
pub(crate) struct Face {
    pub id: FontId,
    data: &'static [u8],
    font: FontRef<'static>,
    pub upem: f32,
    /// Ascent and descent (descent negative), font units.
    pub ascent: f32,
    pub descent: f32,
}

impl Face {
    pub fn new(id: FontId) -> Result<Self, PdfError> {
        let data = match id {
            FontId::Music => BRAVURA,
            FontId::Text => NOTO_SANS_MONO,
        };
        let font = FontRef::new(data).map_err(|e| PdfError::Font(format!("{e}")))?;
        let m = font.metrics(Size::unscaled(), LocationRef::default());
        Ok(Face {
            id,
            data,
            font,
            upem: f32::from(m.units_per_em),
            ascent: m.ascent,
            descent: m.descent,
        })
    }

    /// The glyph of `ch`, or the error that names it.
    pub fn glyph(&self, ch: char) -> Result<u16, PdfError> {
        self.font
            .charmap()
            .map(ch)
            .and_then(|g| u16::try_from(g.to_u32()).ok())
            .filter(|g| *g != 0)
            .ok_or(PdfError::MissingGlyph {
                font: self.id.base_name(),
                ch,
            })
    }

    /// Advance width of a glyph in font units.
    pub fn advance(&self, gid: u16) -> f32 {
        self.font
            .glyph_metrics(Size::unscaled(), LocationRef::default())
            .advance_width(GlyphId::new(u32::from(gid)))
            .unwrap_or(0.0)
    }
}

/// The glyphs a document uses, per font, with the character each one shows.
#[derive(Default)]
pub(crate) struct Usage {
    pub glyphs: BTreeMap<FontId, BTreeMap<u16, char>>,
}

impl Usage {
    pub fn add(&mut self, font: FontId, gid: u16, ch: char) {
        self.glyphs.entry(font).or_default().insert(gid, ch);
    }
}

/// A font after subsetting: glyph ids of the full font mapped to the subset's.
pub(crate) struct Subset {
    pub id: FontId,
    pub remapper: GlyphRemapper,
    pub bytes: Vec<u8>,
    /// Six capital letters, derived from the kept glyphs (deterministic).
    pub tag: String,
    /// Full-font glyph id to its character.
    pub chars: BTreeMap<u16, char>,
}

impl Subset {
    pub fn new(face: &Face, chars: &BTreeMap<u16, char>) -> Result<Self, PdfError> {
        let gids: Vec<u16> = chars.keys().copied().collect();
        let remapper = GlyphRemapper::new_from_glyphs_sorted(&gids);
        let bytes = subsetter::subset(face.data, 0, &remapper)
            .map_err(|e| PdfError::Font(format!("{}: {e}", face.id.base_name())))?;
        let mut h = crate::Fnv::new(0);
        h.write(face.id.base_name().as_bytes());
        for g in &gids {
            h.write(&g.to_be_bytes());
        }
        let mut v = h.value();
        let tag = (0..6)
            .map(|_| {
                let c = (b'A' + (v % 26) as u8) as char;
                v /= 26;
                c
            })
            .collect();
        Ok(Subset {
            id: face.id,
            remapper,
            bytes,
            tag,
            chars: chars.clone(),
        })
    }

    /// The subset glyph id (the CID) of a full-font glyph.
    pub fn cid(&self, gid: u16) -> u16 {
        self.remapper.get(gid).expect("every used glyph is kept")
    }

    pub fn name(&self) -> String {
        format!("{}+{}", self.tag, self.id.base_name())
    }

    /// Writes the Type 0 font with its CID font, descriptor, font file and ToUnicode
    /// map. `next` allocates object ids.
    pub fn write(
        &self,
        face: &Face,
        chunk: &mut Chunk,
        type0: Ref,
        next: &mut impl FnMut() -> Ref,
    ) {
        let (cid_ref, descriptor_ref, file_ref, cmap_ref) = (next(), next(), next(), next());
        let name = self.name();
        let base = Name(name.as_bytes());
        let scale = 1000.0 / face.upem;
        chunk
            .type0_font(type0)
            .base_font(base)
            .encoding_predefined(Name(b"Identity-H"))
            .descendant_font(cid_ref)
            .to_unicode(cmap_ref);
        let info = SystemInfo {
            registry: Str(b"Adobe"),
            ordering: Str(b"Identity"),
            supplement: 0,
        };
        let mut cid = chunk.cid_font(cid_ref);
        cid.subtype(match self.id {
            FontId::Music => CidFontType::Type0,
            FontId::Text => CidFontType::Type2,
        })
        .base_font(base)
        .system_info(info)
        .font_descriptor(descriptor_ref)
        .default_width(0.0);
        if self.id == FontId::Text {
            cid.cid_to_gid_map_predefined(Name(b"Identity"));
        }
        {
            let mut widths = cid.widths();
            for gid in self.chars.keys() {
                widths.consecutive(self.cid(*gid), [face.advance(*gid) * scale]);
            }
        }
        cid.finish();

        let m = face.font.metrics(Size::unscaled(), LocationRef::default());
        let b = m.bounds.unwrap_or_default();
        let mut flags = FontFlags::SYMBOLIC;
        if self.id == FontId::Text {
            flags |= FontFlags::FIXED_PITCH;
        }
        let mut d = chunk.font_descriptor(descriptor_ref);
        d.name(base)
            .flags(flags)
            .bbox(Rect::new(
                b.x_min * scale,
                b.y_min * scale,
                b.x_max * scale,
                b.y_max * scale,
            ))
            .italic_angle(0.0)
            .ascent(face.ascent * scale)
            .descent(face.descent * scale)
            .cap_height(m.cap_height.unwrap_or(face.ascent) * scale)
            .stem_v(80.0);
        match self.id {
            FontId::Music => d.font_file3(file_ref),
            FontId::Text => d.font_file2(file_ref),
        };
        d.finish();

        let packed = miniz_oxide::deflate::compress_to_vec_zlib(&self.bytes, 6);
        let mut file = chunk.stream(file_ref, &packed);
        file.filter(Filter::FlateDecode);
        if self.id == FontId::Music {
            file.pair(Name(b"Subtype"), Name(b"OpenType"));
        } else {
            file.pair(Name(b"Length1"), self.bytes.len() as i32);
        }
        file.finish();

        let mut cmap = UnicodeCmap::new(Name(b"Custom"), info);
        for (gid, ch) in &self.chars {
            cmap.pair(self.cid(*gid), *ch);
        }
        let cmap = cmap.finish();
        chunk.cmap(cmap_ref, &cmap);
    }
}
