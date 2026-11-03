//! `mc-ffi`: the flutter_rust_bridge adapter and composition root of the desktop app
//! (ADR-005). Hand-written code lives in `api` and contains no `unsafe` (ADR-022);
//! `frb_generated` is produced by `flutter_rust_bridge_codegen` and checked by FF-12.

pub mod api;

// ADR-022: the only place in the workspace where unsafe code is allowed. The module is
// generated, never edited by hand (FF-12), and FF-34 rejects any other allow. Its
// formatting is owned by the generator, so rustfmt skips it (FF-12 compares it with
// fresh generator output byte for byte).
#[rustfmt::skip]
#[allow(unsafe_code)]
mod frb_generated;
