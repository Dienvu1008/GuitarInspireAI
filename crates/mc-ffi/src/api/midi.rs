//! MIDI import and export API (slice 4d, ADR-023). Thin bridge over `mc-midi`: it owns
//! no musical rules (GR-05). Export compiles the document's current score without a mute
//! set (ADR-023 item 3: every track is exported, mute is session state) and writes a
//! Standard MIDI File. Import reads a file into a new, unsaved document built through the
//! validating path (`Score::from_parts`, GR-09) and returns a report of what changed.

use crate::api::document::DocumentHandle;
use crate::api::types::ApiError;
use flutter_rust_bridge::frb;
use mc_app::playback::compile_for_export;
use mc_midi::{ExportReport, ImportReport, export_smf, import_smf};

/// What an import changed or dropped, flattened for the UI (ADR-012 report). The issue
/// list is human-readable; counts are structured for badges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportReportDto {
    pub tracks_imported: u32,
    pub notes_imported: u32,
    pub bars: u32,
    /// Onsets and ends that moved when snapped to the quantization grid.
    pub quantized_points: u32,
    /// Bars that used the triplet (1/12) grid.
    pub triplet_bars: u32,
    /// One line per issue (percussion skipped, unplayable pitch, defaulted tempo, ...).
    pub issues: Vec<String>,
}

/// The result of importing: the new document and what the import did.
#[derive(Debug)]
#[frb(opaque)]
pub struct ImportResult {
    pub document: DocumentHandle,
    pub report: ImportReportDto,
}

/// What an export wrote (ADR-012 report).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportReportDto {
    /// MIDI tracks written, including the conductor track.
    pub tracks: u32,
    pub notes: u32,
    /// Timeline ticks rounded to 960 PPQ.
    pub rounded_ticks: u32,
    /// Tempo values clamped to the SMF minimum.
    pub tempo_clamped: u32,
    pub time_signatures: u32,
}

fn import_report_dto(r: &ImportReport) -> ImportReportDto {
    ImportReportDto {
        tracks_imported: r.tracks_imported as u32,
        notes_imported: r.notes_imported as u32,
        bars: r.bars as u32,
        quantized_points: r.quantized_points as u32,
        triplet_bars: r.triplet_bars as u32,
        issues: r.issues.iter().map(|i| format!("{i:?}")).collect(),
    }
}

fn export_report_dto(r: &ExportReport) -> ExportReportDto {
    ExportReportDto {
        tracks: r.tracks as u32,
        notes: r.notes as u32,
        rounded_ticks: r.rounded_ticks as u32,
        tempo_clamped: r.tempo_clamped as u32,
        time_signatures: r.time_signatures as u32,
    }
}

/// Import a Standard MIDI File into a new, unsaved document. The bytes are the file
/// content (the UI reads the file; the core never touches the filesystem here). Parser
/// errors (too large, too many tracks, malformed) are returned as an `ApiError`.
pub fn import_midi(bytes: Vec<u8>) -> Result<ImportResult, ApiError> {
    let (score, report) =
        import_smf(&bytes).map_err(|e| ApiError::new("midi-import", e.to_string()))?;
    Ok(ImportResult {
        document: DocumentHandle::from_imported_score(score),
        report: import_report_dto(&report),
    })
}

/// Export `document` as a Standard MIDI File (type 1, 960 PPQ). Returns the file bytes
/// and a report; the UI writes the bytes to disk. All tracks are exported regardless of
/// mute (ADR-023 item 3). The tempo factor is not applied.
pub fn export_midi(document: &DocumentHandle) -> Result<ExportResult, ApiError> {
    let score = document.score_snapshot();
    let timeline = compile_for_export(&score).map_err(|e| ApiError::new("midi-export", e))?;
    let (bytes, report) =
        export_smf(&score, &timeline).map_err(|e| ApiError::new("midi-export", e.to_string()))?;
    Ok(ExportResult {
        bytes,
        report: export_report_dto(&report),
    })
}

/// The result of exporting: the file bytes and what was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportResult {
    pub bytes: Vec<u8>,
    pub report: ExportReportDto,
}
