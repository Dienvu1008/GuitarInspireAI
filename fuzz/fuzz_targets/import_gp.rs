//! FF-15: fuzz the Guitar Pro importer (ADR-036 G5). `import_gp` must never panic, hang
//! or allocate without bound on arbitrary bytes; it either returns a validated `Score`
//! with an import report, or a typed `GpImportError`. Every count is bounded before
//! allocating (16 MiB, 10 000 bars, 127 tracks, 7 strings, 1 000 beats per voice, 100
//! bend points, 1 MiB strings), so libfuzzer makes progress. The weekly job seeds the
//! corpus with the S10 fixtures. A crash is minimized and committed as an
//! mc-guitarpro regression test.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // A successful import went through `Score::from_parts`, so it holds every hard
    // invariant; `Invalid` would be an importer bug and is turned into a crash here.
    if let Err(mc_guitarpro::GpImportError::Invalid(v)) = mc_guitarpro::import_gp(data) {
        panic!("the importer built an invalid score: {v:?}");
    }
});
