//! PDF export API (ADR-041 section 12). Thin bridge over `mc-pdf`: it owns no layout
//! or painting rule (GR-05). The export contains what the view shows: the shown tracks
//! with their notation and TAB setting (section 11, choice 4).

use crate::api::document::DocumentHandle;
use crate::api::layout::ShownTrackDto;
use crate::api::types::ApiError;
use mc_app::notation::{PageSetup, PageSize, SystemSpacing};

/// Paper size (section 11, choice 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaperDto {
    A4,
    Letter,
}

/// Gap between systems (section 12, choice 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemSpacingDto {
    /// 4 staff spaces (19.2 pt).
    Spacious,
    /// The scaled screen gap (9.6 pt).
    Compact,
}

/// The file bytes and what was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfExportResult {
    pub bytes: Vec<u8>,
    pub pages: u32,
    /// Subset names of the embedded fonts, for example `ABCDEF+Bravura`.
    pub fonts: Vec<String>,
}

/// Export the `shown` staves of `document` as PDF, with `title` on page 1. Returns the
/// bytes; the UI writes them to disk. A character missing from the bundled fonts is
/// an error (`pdf-export`), never a substitute.
pub fn export_pdf(
    document: &DocumentHandle,
    shown: Vec<ShownTrackDto>,
    paper: PaperDto,
    spacing: SystemSpacingDto,
    title: String,
) -> Result<PdfExportResult, ApiError> {
    let score = document.score_snapshot();
    let shown: Vec<_> = shown.into_iter().map(ShownTrackDto::to_core).collect();
    let setup = PageSetup {
        system_spacing: match spacing {
            SystemSpacingDto::Spacious => SystemSpacing::Spacious,
            SystemSpacingDto::Compact => SystemSpacing::Compact,
        },
        ..PageSetup::new(match paper {
            PaperDto::A4 => PageSize::A4,
            PaperDto::Letter => PageSize::LETTER,
        })
    };
    let out = mc_pdf::export_pdf(&score, &shown, &setup, &title)
        .map_err(|e| ApiError::new("pdf-export", e.to_string()))?;
    Ok(PdfExportResult {
        bytes: out.bytes,
        pages: u32::try_from(out.pages).unwrap_or(u32::MAX),
        fonts: out.fonts.into_iter().map(|f| f.name).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_the_shown_staves_and_reports_missing_characters() {
        let doc = DocumentHandle::from_imported_score(mc_app::fixtures::feature_rich());
        let shown: Vec<ShownTrackDto> = doc
            .score_snapshot()
            .tracks()
            .map(|t| ShownTrackDto {
                track: t.id().raw() as i64,
                notation: true,
                tab: true,
            })
            .collect();
        let r = export_pdf(
            &doc,
            shown.clone(),
            PaperDto::Letter,
            SystemSpacingDto::Compact,
            "Song".into(),
        )
        .unwrap();
        assert!(r.bytes.starts_with(b"%PDF-1.7"));
        assert_eq!(r.pages, 1);
        assert_eq!(r.fonts.len(), 2);
        let e = export_pdf(
            &doc,
            shown,
            PaperDto::A4,
            SystemSpacingDto::Spacious,
            "曲".into(),
        )
        .unwrap_err();
        assert_eq!(e.rule, "pdf-export");
        assert!(e.message.contains("U+66F2"), "{}", e.message);
    }
}
