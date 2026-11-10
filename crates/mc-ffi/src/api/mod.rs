//! Public bridge API. Every public item here is exposed to Dart by the code generator.
//! No `unsafe` in this module tree (ADR-022, FF-34).

pub mod document;
pub mod engine;
pub mod info;
pub mod layout;
pub mod midi;
pub mod musicxml;
pub mod pattern;
pub mod tuner;
pub mod types;
