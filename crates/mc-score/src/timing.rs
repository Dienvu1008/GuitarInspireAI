//! Effective master bar duration and bar start times (domain-invariants section 4a).

use crate::model::Voice;
use crate::repeats::{self, PlaybackOrder};
use crate::{DomainError, Score};
use mc_theory::Rational;

/// Sum of beat lengths of a voice.
pub fn voice_duration(voice: &Voice) -> Result<Rational, DomainError> {
    Rational::checked_sum(voice.beats.iter().map(|b| b.value.length())).ok_or(DomainError::Overflow)
}

impl Score {
    /// `nominal(m)`: time signature length, or 0 for a pickup bar.
    pub fn nominal_bar_duration(&self, bar: usize) -> Option<Rational> {
        self.master_bars.get(bar).map(|mb| {
            if mb.pickup {
                Rational::ZERO
            } else {
                mb.time_signature.length()
            }
        })
    }

    /// `E(m) = max(nominal(m), longest voice across every track and voice)`. Muted
    /// state is not part of the score, so mute can never change timing (PB-04).
    pub fn effective_bar_duration(&self, bar: usize) -> Result<Rational, DomainError> {
        let mut e = self
            .nominal_bar_duration(bar)
            .ok_or_else(|| DomainError::NotFound(format!("master bar {bar}")))?;
        for track in &self.tracks {
            for voice in &track.bars[bar].voices {
                e = e.max(voice_duration(voice)?);
            }
        }
        Ok(e)
    }

    /// Master bars in playback order after repeat expansion.
    pub fn playback_order(&self) -> PlaybackOrder {
        repeats::playback_order(&self.master_bars)
    }

    /// `(bar, start)` for every played bar with a non-zero effective duration, in
    /// playback order. Start times are shared by all tracks, so later bars stay
    /// synchronized after an overfull bar. Bars with `E(m) = 0` (empty pickup) are
    /// skipped (PB-06).
    pub fn bar_start_times(&self) -> Result<Vec<(usize, Rational)>, DomainError> {
        let durations: Vec<Rational> = (0..self.master_bars.len())
            .map(|m| self.effective_bar_duration(m))
            .collect::<Result<_, _>>()?;
        let mut t = Rational::ZERO;
        let mut out = Vec::new();
        for m in self.playback_order().bars {
            let e = durations[m];
            if e.is_zero() {
                continue;
            }
            out.push((m, t));
            t = t.checked_add(e).ok_or(DomainError::Overflow)?;
        }
        Ok(out)
    }
}
