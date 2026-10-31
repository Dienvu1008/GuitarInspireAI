//! Well-formedness diagnostics allowed in committed documents (never block saving).

use crate::timing::voice_duration;
use crate::{DomainError, Score, TrackId};
use mc_theory::Rational;
use std::ops::Range;

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
            let mb = &self.master_bars[m];
            let nominal = mb.time_signature.length();
            for track in &self.tracks {
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
            }
        }
        Ok(out)
    }
}
