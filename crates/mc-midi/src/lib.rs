//! MIDI interchange adapter (ADR-012, ADR-023).
//!
//! Export writes a compiled `PerformanceTimeline` as a Standard MIDI File (type 1,
//! 960 PPQ). Import reads a file into a validated `Score` through `Score::from_parts`
//! (GR-09, IM-01) with an [`ImportReport`] that lists everything quantized, skipped or
//! defaulted. Musical rules stay in the owning crates: pitch to string is
//! `mc_guitar::assign_group`, the MIDI value objects and rendering conventions are in
//! `mc-playback`. This crate only translates between the file format and those types.

mod export;
mod import;

pub use export::{ExportError, ExportReport, PPQ, export_smf};
pub use import::{
    ImportError, ImportIssue, ImportReport, MAX_BARS, MAX_EVENTS, MAX_FILE_BYTES, MAX_TRACKS,
    import_smf,
};
