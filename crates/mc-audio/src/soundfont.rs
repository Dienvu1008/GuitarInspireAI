//! SoundFont loading and synthesizer construction. Runs on the control thread, never in
//! the callback. A new synthesizer is warmed up before it is handed to the callback
//! (ADR-007 amendment 4: S2 saw late callbacks of up to 17 ms right after a large
//! SoundFont load).

use crate::error::AudioError;
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::sync::Arc;

/// Parse SF2 bytes.
pub fn parse_soundfont(bytes: &[u8]) -> Result<Arc<SoundFont>, AudioError> {
    let mut reader = bytes;
    SoundFont::new(&mut reader)
        .map(Arc::new)
        .map_err(|e| AudioError::SoundFont(e.to_string()))
}

/// A warmed-up synthesizer for `sample_rate`, ready to be installed in the callback.
pub fn build_synth(
    font: &Arc<SoundFont>,
    sample_rate: u32,
) -> Result<Box<Synthesizer>, AudioError> {
    let settings = SynthesizerSettings::new(sample_rate as i32);
    let mut synth =
        Synthesizer::new(font, &settings).map_err(|e| AudioError::SoundFont(e.to_string()))?;
    warm_up(&mut synth, sample_rate);
    Ok(Box::new(synth))
}

/// Touch the sample data and the render path: a few quiet notes over the guitar range,
/// rendered for about 100 ms, then released. Output is discarded.
fn warm_up(synth: &mut Synthesizer, sample_rate: u32) {
    let frames = (sample_rate as usize / 10).max(480);
    let mut l = vec![0.0f32; frames];
    let mut r = vec![0.0f32; frames];
    for key in (40..=88).step_by(6) {
        synth.note_on(0, key, 1);
    }
    synth.render(&mut l, &mut r);
    synth.note_off_all(true);
    synth.render(&mut l, &mut r);
}
