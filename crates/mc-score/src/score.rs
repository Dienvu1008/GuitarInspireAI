use crate::model::{BarContent, InstrumentSpec, MasterBar, TempoChange, Track, TrackData};
use crate::{DomainError, TrackId, Violation, validate};
use mc_theory::{Rational, Tempo};
use std::sync::Arc;

/// The Score aggregate. Cloning is cheap: master bars, tempo map, tracks and bars are
/// shared through `Arc` until edited (copy-on-write, ADR-016).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Score {
    pub(crate) master_bars: Arc<Vec<MasterBar>>,
    pub(crate) tracks: Vec<Arc<Track>>,
    pub(crate) tempo_map: Arc<Vec<TempoChange>>,
    pub(crate) next_id: u64,
}

/// Raw content for building a validated score (loaders and importers, GR-09).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreParts {
    pub master_bars: Vec<MasterBar>,
    pub tracks: Vec<TrackData>,
    pub tempo_map: Vec<TempoChange>,
}

impl Score {
    /// A score with one track, one master bar and an initial tempo at the start.
    pub fn new(
        track_name: &str,
        instrument: InstrumentSpec,
        master_bar: MasterBar,
        tempo: Tempo,
    ) -> Result<Self, DomainError> {
        let parts = ScoreParts {
            master_bars: vec![master_bar],
            tracks: vec![TrackData {
                id: TrackId::from_raw(1),
                name: track_name.to_string(),
                instrument,
                bars: vec![BarContent::empty()],
            }],
            tempo_map: vec![TempoChange {
                bar: 0,
                offset: Rational::ZERO,
                tempo,
            }],
        };
        Score::from_parts(parts).map_err(|v| {
            let first = &v[0];
            DomainError::invariant(first.rule, first.to_string())
        })
    }

    /// Builds a score and checks every hard invariant (P-05, GR-09). Returns all
    /// violations, not only the first. The id counter continues after the largest id.
    pub fn from_parts(parts: ScoreParts) -> Result<Self, Vec<Violation>> {
        let max_id = parts
            .tracks
            .iter()
            .flat_map(|t| {
                std::iter::once(t.id.raw()).chain(t.bars.iter().flat_map(|b| {
                    b.voices.iter().flat_map(|v| {
                        v.beats.iter().flat_map(|beat| {
                            std::iter::once(beat.id.raw())
                                .chain(beat.notes.iter().map(|n| n.id.raw()))
                        })
                    })
                }))
            })
            .max()
            .unwrap_or(0);
        let score = Score {
            master_bars: Arc::new(parts.master_bars),
            tracks: parts
                .tracks
                .into_iter()
                .map(|t| {
                    Arc::new(Track {
                        id: t.id,
                        name: t.name,
                        instrument: t.instrument,
                        bars: t.bars.into_iter().map(Arc::new).collect(),
                    })
                })
                .collect(),
            tempo_map: Arc::new(parts.tempo_map),
            next_id: max_id.saturating_add(1),
        };
        let violations = score.validate();
        if violations.is_empty() {
            Ok(score)
        } else {
            Err(violations)
        }
    }

    /// Checks every hard invariant of the whole score. Commands check only what they
    /// touch; this full check runs on load, in tests and in property tests (FF-13).
    pub fn validate(&self) -> Vec<Violation> {
        validate::validate(self)
    }

    pub fn master_bars(&self) -> &[MasterBar] {
        &self.master_bars
    }

    pub fn master_bar_count(&self) -> usize {
        self.master_bars.len()
    }

    pub fn tracks(&self) -> impl ExactSizeIterator<Item = &Track> {
        self.tracks.iter().map(|t| t.as_ref())
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn track(&self, id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id).map(|t| t.as_ref())
    }

    pub fn track_at(&self, index: usize) -> Option<&Track> {
        self.tracks.get(index).map(|t| t.as_ref())
    }

    pub fn tempo_map(&self) -> &[TempoChange] {
        &self.tempo_map
    }

    /// Total number of notes (used by tools and benchmarks).
    pub fn note_count(&self) -> usize {
        self.tracks
            .iter()
            .flat_map(|t| t.bars.iter())
            .flat_map(|b| b.voices.iter())
            .flat_map(|v| v.beats.iter())
            .map(|b| b.notes.len())
            .sum()
    }

    /// Number of bar allocations in `self` that are not shared with `other` (for undo
    /// memory accounting). Compares pointers only, O(tracks x bars).
    pub fn unshared_bars(&self, other: &Score) -> usize {
        let mut count = 0;
        for t in &self.tracks {
            match other.tracks.iter().find(|o| o.id == t.id) {
                Some(o) if Arc::ptr_eq(t, o) => {}
                Some(o) => {
                    count += (0..t.bars.len())
                        .filter(|&i| !t.shares_bar_with(o, i))
                        .count()
                }
                None => count += t.bars.len(),
            }
        }
        count
    }

    /// True if both scores share the same master bar allocation (unchanged between snapshots).
    pub fn shares_master_bars_with(&self, other: &Score) -> bool {
        Arc::ptr_eq(&self.master_bars, &other.master_bars)
    }

    /// True if both scores share the same tempo map allocation.
    pub fn shares_tempo_map_with(&self, other: &Score) -> bool {
        Arc::ptr_eq(&self.tempo_map, &other.tempo_map)
    }

    /// Approximate heap bytes held by `self` but not shared with `other`: copied track
    /// spines, copied bars with their voices, beats and notes, and copied master bar or
    /// tempo vectors. Used for the undo byte budget; FF-23 compares it with measured
    /// allocations.
    pub fn unshared_heap_bytes(&self, other: &Score) -> usize {
        use std::mem::size_of;
        let mut bytes = 0;
        if !self.shares_master_bars_with(other) {
            bytes += self.master_bars.len() * size_of::<MasterBar>();
        }
        if !self.shares_tempo_map_with(other) {
            bytes += self.tempo_map.len() * size_of::<TempoChange>();
        }
        for t in &self.tracks {
            let old = other.tracks.iter().find(|o| o.id == t.id);
            if old.is_some_and(|o| Arc::ptr_eq(t, o)) {
                continue;
            }
            bytes += size_of::<Track>() + t.bars.len() * size_of::<Arc<BarContent>>();
            for (i, bar) in t.bars.iter().enumerate() {
                if old.is_some_and(|o| t.shares_bar_with(o, i)) {
                    continue;
                }
                bytes += size_of::<BarContent>() + bar.voices.len() * size_of::<crate::Voice>();
                for v in &bar.voices {
                    bytes += v.beats.len() * size_of::<crate::Beat>();
                    bytes += v
                        .beats
                        .iter()
                        .map(|b| b.notes.len() * size_of::<crate::Note>())
                        .sum::<usize>();
                }
            }
        }
        bytes
    }

    /// Structural estimate of the heap bytes of the whole score (element sizes times
    /// counts; strings and allocator overhead not included). Deterministic; used by FF-23.
    pub fn heap_bytes(&self) -> usize {
        use std::mem::size_of;
        let mut bytes = self.master_bars.len() * size_of::<MasterBar>()
            + self.tempo_map.len() * size_of::<TempoChange>();
        for t in &self.tracks {
            bytes += size_of::<Track>() + t.bars.len() * size_of::<Arc<BarContent>>();
            for bar in &t.bars {
                bytes += size_of::<BarContent>() + bar.voices.len() * size_of::<crate::Voice>();
                for v in &bar.voices {
                    bytes += v.beats.len() * size_of::<crate::Beat>();
                    bytes += v
                        .beats
                        .iter()
                        .map(|b| b.notes.len() * size_of::<crate::Note>())
                        .sum::<usize>();
                }
            }
        }
        bytes
    }

    pub(crate) fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub(crate) fn track_index(&self, id: TrackId) -> Result<usize, DomainError> {
        self.tracks
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| DomainError::NotFound(format!("track {}", id.raw())))
    }
}
