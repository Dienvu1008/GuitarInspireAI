//! Well-formedness diagnostics allowed in committed documents (never block saving).

use crate::model::Track;
use crate::timing::voice_duration;
use crate::{DomainError, Score, TrackId};
use mc_theory::Rational;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    /// D-01: voice shorter than the time signature.
    Underfull { actual: Rational, nominal: Rational },
    /// D-01: voice longer than the time signature.
    Overfull { actual: Rational, nominal: Rational },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub rule: &'static str,
    pub track: TrackId,
    pub bar: usize,
    pub voice: usize,
    pub kind: DiagnosticKind,
}

impl Score {
    /// D-01 for the given master bar range. Empty voices are "not written" and not
    /// reported. Pickup bars are not reported for being short.
    pub fn diagnostics(&self, bars: Range<usize>) -> Result<Vec<Diagnostic>, DomainError> {
        let end = bars.end.min(self.master_bars.len());
        let mut out = Vec::new();
        for m in bars.start..end {
            for track in &self.tracks {
                self.bar_diagnostics(track, m, &mut out)?;
            }
        }
        Ok(out)
    }

    /// D-01 for every bar, equal to `diagnostics(0..master_bar_count)`, but reusing
    /// `previous` (the full result for `before`) for bars this score still shares with
    /// `before` (copy-on-write, ADR-016). An edit then rechecks only the bars it
    /// touched instead of the whole score. Master bar changes fall back to a full scan.
    pub fn diagnostics_since(
        &self,
        before: &Score,
        previous: &[Diagnostic],
    ) -> Result<Vec<Diagnostic>, DomainError> {
        let all = 0..self.master_bars.len();
        if !Arc::ptr_eq(&self.master_bars, &before.master_bars) {
            return self.diagnostics(all);
        }
        let old_tracks: HashMap<TrackId, &Track> =
            before.tracks.iter().map(|t| (t.id, t.as_ref())).collect();
        let mut old_results: HashMap<(TrackId, usize), Vec<&Diagnostic>> = HashMap::new();
        for d in previous {
            old_results.entry((d.track, d.bar)).or_default().push(d);
        }
        let mut out = Vec::new();
        for m in all {
            for track in &self.tracks {
                let shared = old_tracks
                    .get(&track.id)
                    .and_then(|old| old.bars.get(m))
                    .is_some_and(|old| Arc::ptr_eq(old, &track.bars[m]));
                if shared {
                    if let Some(reused) = old_results.get(&(track.id, m)) {
                        out.extend(reused.iter().map(|d| (*d).clone()));
                    }
                } else {
                    self.bar_diagnostics(track, m, &mut out)?;
                }
            }
        }
        Ok(out)
    }

    /// D-01 for one track only (same rule as `diagnostics`); used by projections that
    /// show a single track, so they do not scan every track.
    pub fn track_diagnostics(
        &self,
        track: TrackId,
        bars: Range<usize>,
    ) -> Result<Vec<Diagnostic>, DomainError> {
        let t = self.track_index(track)?;
        let end = bars.end.min(self.master_bars.len());
        let mut out = Vec::new();
        for m in bars.start..end {
            self.bar_diagnostics(&self.tracks[t], m, &mut out)?;
        }
        Ok(out)
    }

    fn bar_diagnostics(
        &self,
        track: &Track,
        m: usize,
        out: &mut Vec<Diagnostic>,
    ) -> Result<(), DomainError> {
        let mb = &self.master_bars[m];
        let nominal = mb.time_signature.length();
        for (vi, voice) in track.bars[m].voices.iter().enumerate() {
            if voice.beats.is_empty() {
                continue;
            }
            let actual = voice_duration(voice)?;
            let kind = if actual > nominal {
                DiagnosticKind::Overfull { actual, nominal }
            } else if actual < nominal && !mb.pickup {
                DiagnosticKind::Underfull { actual, nominal }
            } else {
                continue;
            };
            out.push(Diagnostic {
                rule: "D-01",
                track: track.id,
                bar: m,
                voice: vi,
                kind,
            });
        }
        Ok(())
    }
}
