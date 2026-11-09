//! MusicXML export API (ADR-033). Thin bridge over `mc-musicxml`: it owns no musical
//! rules (GR-05). The export reads the document's current score; every track is
//! written, whatever is muted, soloed or hidden, and the mix is not exported.

use crate::api::document::DocumentHandle;
use crate::api::types::ApiError;
use mc_musicxml::{MusicXmlReport, export_musicxml as write_musicxml};

/// What a MusicXML export wrote and approximated (ADR-012 report, ADR-033 section 4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicXmlReportDto {
    pub parts: u32,
    pub measures: u32,
    pub notes: u32,
    /// Voices shorter than their measure, padded with a rest.
    pub padded_voices: u32,
    /// Measures longer than their time signature; every note is kept.
    pub overfull_measures: u32,
    /// Tuplet beats written without a bracket (duration and ratio still exact).
    pub tuplets_without_bracket: u32,
    /// Tempo values rounded to three decimals.
    pub tempo_rounded: u32,
    /// Characters removed from names and markers (not allowed in XML).
    pub characters_removed: u32,
}

/// The result of exporting: the UTF-8 file bytes and what was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicXmlExportResult {
    pub bytes: Vec<u8>,
    pub report: MusicXmlReportDto,
}

fn report_dto(r: &MusicXmlReport) -> MusicXmlReportDto {
    let n = |v: usize| u32::try_from(v).unwrap_or(u32::MAX);
    MusicXmlReportDto {
        parts: n(r.parts),
        measures: n(r.measures),
        notes: n(r.notes),
        padded_voices: n(r.padded_voices),
        overfull_measures: n(r.overfull_measures),
        tuplets_without_bracket: n(r.tuplets_without_bracket),
        tempo_rounded: n(r.tempo_rounded),
        characters_removed: n(r.characters_removed),
    }
}

/// Export `document` as MusicXML 4.0 (uncompressed `.musicxml`). Returns the bytes and
/// a report; the UI writes the bytes to disk.
pub fn export_musicxml(document: &DocumentHandle) -> Result<MusicXmlExportResult, ApiError> {
    let score = document.score_snapshot();
    let (xml, report) =
        write_musicxml(&score).map_err(|e| ApiError::new("musicxml-export", e.to_string()))?;
    Ok(MusicXmlExportResult {
        bytes: xml.into_bytes(),
        report: report_dto(&report),
    })
}
