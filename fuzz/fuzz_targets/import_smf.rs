//! FF-15: fuzz the MIDI importer. `import_smf` must never panic, hang or read out of
//! bounds on arbitrary bytes; it either returns a validated `Score` with an
//! `ImportReport`, or a typed `ImportError` (GR-09, IM-01, ADR-023). The parser limits
//! (16 MB, 64 tracks, 1 000 000 events, 10 000 bars) bound the work, so libfuzzer makes
//! progress. A crash here is minimized and committed as an mc-midi regression fixture.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Only the outcome matters; the fuzzer catches panics, OOM and timeouts. A validated
    // score must still satisfy every hard invariant, which `Score::from_parts` enforces
    // inside the importer, so a successful import that violated an invariant cannot occur.
    let _ = mc_midi::import_smf(data);
});
