//! Guitar Pro import and export API (ADR-036). Thin bridge over `mc-guitarpro`: it owns
//! no format or music rule (GR-05). Import reads the file bytes into a new, unsaved
//! document built through the validating path (`Score::from_parts`, GR-09); export
//! writes the document's current score. The UI reads and writes the files.

use crate::api::document::DocumentHandle;
use crate::api::types::ApiError;
use flutter_rust_bridge::frb;
use mc_guitarpro::{GpExportReport, GpImportReport, GpVersion, export_gp, import_gp};

fn n(v: usize) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

/// What a Guitar Pro import kept and dropped (ADR-012 report, ADR-036 section 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuitarProImportReportDto {
    /// The file's version string, for example "FICHIER GUITAR PRO v5.00".
    pub version: String,
    pub tracks_imported: u32,
    pub bars: u32,
    pub notes_in_file: u32,
    pub notes_imported: u32,
    /// Notes in the file that are not in the score; each drop is also an issue.
    pub notes_dropped: u32,
    /// One human-readable line per kind of change, with its count and first bar.
    pub issues: Vec<String>,
}

/// The result of importing: the new document and what the import did.
#[derive(Debug)]
#[frb(opaque)]
pub struct GuitarProImportResult {
    pub document: DocumentHandle,
    pub report: GuitarProImportReportDto,
}

/// What a Guitar Pro export wrote and approximated (ADR-036 section 4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuitarProExportReportDto {
    /// "gp5", "gp4" or "gp3".
    pub version: String,
    pub tracks: u32,
    pub bars: u32,
    pub notes_written: u32,
    pub notes_dropped: u32,
    pub issues: Vec<String>,
}

/// The result of exporting: the file bytes and what was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuitarProExportResult {
    pub bytes: Vec<u8>,
    pub report: GuitarProExportReportDto,
}

fn import_dto(r: &GpImportReport) -> GuitarProImportReportDto {
    GuitarProImportReportDto {
        version: r.version.clone(),
        tracks_imported: n(r.tracks_imported),
        bars: n(r.bars),
        notes_in_file: n(r.notes_in_file),
        notes_imported: n(r.notes_imported),
        notes_dropped: n(r.notes_dropped),
        issues: r.issues.iter().map(|i| i.to_string()).collect(),
    }
}

fn export_dto(r: &GpExportReport) -> GuitarProExportReportDto {
    GuitarProExportReportDto {
        version: r.version.extension().to_string(),
        tracks: n(r.tracks),
        bars: n(r.bars),
        notes_written: n(r.notes_written),
        notes_dropped: n(r.notes_dropped),
        issues: r.issues.iter().map(|i| i.to_string()).collect(),
    }
}

/// Import a GP3, GP4 or GP5 (5.00) file into a new, unsaved document. The version is
/// recognised from the content, not the file name. Unsupported or unreadable files
/// are an `ApiError` naming the cause.
pub fn import_guitar_pro(bytes: Vec<u8>) -> Result<GuitarProImportResult, ApiError> {
    let (score, report) =
        import_gp(&bytes).map_err(|e| ApiError::new("guitar-pro-import", e.to_string()))?;
    Ok(GuitarProImportResult {
        document: DocumentHandle::from_imported_score(score),
        report: import_dto(&report),
    })
}

/// Whether `extension` ("gp5", ".gp4", "GP3" ...) names a version the exporter writes.
/// The UI uses it to decide the version from the chosen file name (Dart decides
/// nothing about the format, ADR-036 section 5).
#[frb(sync)]
pub fn guitar_pro_export_extension_supported(extension: String) -> bool {
    GpVersion::from_extension(&extension).is_some()
}

/// Export `document` as Guitar Pro in the version named by `extension` ("gp5", "gp4"
/// or "gp3", with or without the dot). Every track is written, whatever is muted.
pub fn export_guitar_pro(
    document: &DocumentHandle,
    extension: String,
) -> Result<GuitarProExportResult, ApiError> {
    let version = GpVersion::from_extension(&extension).ok_or_else(|| {
        ApiError::new(
            "guitar-pro-export",
            format!("unknown Guitar Pro extension '{extension}'; use gp5, gp4 or gp3"),
        )
    })?;
    let score = document.score_snapshot();
    let (bytes, report) = export_gp(&score, version)
        .map_err(|e| ApiError::new("guitar-pro-export", e.to_string()))?;
    Ok(GuitarProExportResult {
        bytes,
        report: export_dto(&report),
    })
}
