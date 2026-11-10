//! Binding (ADR-034 section 3 stage 3, section 5): a resolved pattern, one voicing, its
//! chord and the instrument give a `PatternFragment` of beats and notes per lane, in
//! pattern time. Only sounded strings are ever selected (decision 13.4); every
//! failure is a typed error naming its step (PT-02).

use std::collections::BTreeMap;

use mc_guitar::Voicing;
use mc_score::{FrettedInstrument, NoteEffects, Stroke};
use mc_theory::fretted::{Fret, FretPosition, StringIndex, pitch_at};
use mc_theory::{Chord, NoteValue, Rational, TimeSignature};

use crate::error::StepRef;
use crate::ids::PatternId;
use crate::library::{ResolvedAction, ResolvedPattern};
use crate::model::{LaneId, StringSelector, Technique, TechniqueSpan};

/// One bound note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundNote {
    pub position: FretPosition,
    /// True for a held note: it continues the same string and fret of the preceding
    /// beat of its lane (H-06).
    pub tie: bool,
    pub effects: NoteEffects,
}

/// One bound beat: an attack or a hold, at its pattern onset. Rests and gaps produce
/// no beat; placement writes them as rests (PT-09).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundBeat {
    /// The index of the step in its lane.
    pub step: usize,
    pub onset: Rational,
    pub duration: NoteValue,
    /// The strike's direction; `None` on holds.
    pub stroke: Option<Stroke>,
    /// String 1 first, each string once.
    pub notes: Vec<BoundNote>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundLane {
    pub lane: LaneId,
    pub beats: Vec<BoundBeat>,
}

/// A bound pattern, in pattern time, ready to be placed (6a-3b).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternFragment {
    pub pattern: PatternId,
    pub length: Rational,
    pub meter: Option<TimeSignature>,
    pub lanes: Vec<BoundLane>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindError {
    /// The voicing does not have one entry per string of the instrument.
    VoicingStringCount { voicing: u8, instrument: u8 },
    /// A fret of the voicing is beyond the instrument's frets.
    FretOutOfRange {
        string: u8,
        fret: u8,
        fret_count: u8,
    },
    /// The voicing sounds no string.
    NoSoundedStrings,
    /// `Absolute` or `StrumRange` names a string the instrument does not have.
    StringOutOfRange {
        step: StepRef,
        string: u8,
        strings: u8,
    },
    /// `Absolute` names a string the voicing mutes.
    StringNotInVoicing { step: StepRef, string: u8 },
    /// `Alt`: no sounded string between the bass and the upper region.
    NoAlternateBass { step: StepRef },
    /// `Treble`: no sounded string above the bass.
    NoTrebleStrings { step: StepRef },
    /// The strike (or a `StrumRange` of it) selects no sounded string.
    EmptyStrike { step: StepRef },
    /// A hold of a dead, palm-muted or string-percussive note: the score would play
    /// the tie as a new attack (ADR-024, ADR-035), so it cannot be held.
    HoldOfShortNote { step: StepRef, string: u8 },
    /// The span covers no attack (only holds or rests), so its technique would reach
    /// no note.
    SpanReachesNoAttack { span: usize },
}

/// The regions of a voicing (ADR-034 section 5), as string numbers. `sounded` runs
/// from the lowest string (highest number) to string 1.
struct Regions {
    sounded: Vec<u8>,
    frets: BTreeMap<u8, Fret>,
    bass: u8,
    /// The sounded strings above the bass, lowest first.
    above: Vec<u8>,
    alt: Option<u8>,
}

impl Regions {
    fn treble(&self, n: u8) -> Vec<u8> {
        let k = self.above.len().saturating_sub(n as usize);
        self.above[k..].to_vec()
    }
}

fn regions(
    voicing: &Voicing,
    chord: &Chord,
    instrument: &FrettedInstrument,
) -> Result<Regions, Vec<BindError>> {
    let strings = instrument.tuning.string_count();
    if voicing.string_count() != strings {
        return Err(vec![BindError::VoicingStringCount {
            voicing: voicing.string_count(),
            instrument: strings,
        }]);
    }
    let mut errors = Vec::new();
    let mut frets = BTreeMap::new();
    for s in 1..=strings {
        if let Some(f) = voicing.fret(StringIndex::new(s).expect("s >= 1")) {
            if f.0 > instrument.fret_count {
                errors.push(BindError::FretOutOfRange {
                    string: s,
                    fret: f.0,
                    fret_count: instrument.fret_count,
                });
            }
            frets.insert(s, f);
        }
    }
    if frets.is_empty() {
        errors.push(BindError::NoSoundedStrings);
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let sounded: Vec<u8> = frets.keys().rev().copied().collect();
    let bass = sounded[0];
    let above = sounded[1..].to_vec();
    let upper_start = above.len().saturating_sub(3);
    let candidates = &above[..upper_start];
    let pc = |s: u8| {
        pitch_at(
            &instrument.tuning,
            instrument.capo,
            FretPosition {
                string: StringIndex::new(s).expect("s >= 1"),
                fret: frets[&s],
            },
        )
        .map(|p| p.rem_euclid(12))
    };
    let root = chord.root.pitch_class().value() as i32;
    let degree = |d: u8| {
        chord
            .quality
            .intervals()
            .iter()
            .find(|i| i.degree == d)
            .map(|i| (root + i.semitones as i32).rem_euclid(12))
    };
    let pick = |d: u8| {
        degree(d).and_then(|target| candidates.iter().copied().find(|s| pc(*s) == Some(target)))
    };
    let alt = pick(5)
        .or_else(|| pick(3))
        .or_else(|| candidates.first().copied());
    Ok(Regions {
        sounded,
        frets,
        bass,
        above,
        alt,
    })
}

/// The strings one selector picks, or the error naming the step.
fn select(
    sel: StringSelector,
    r: &Regions,
    strings: u8,
    step: StepRef,
) -> Result<Vec<u8>, BindError> {
    use StringSelector::*;
    match sel {
        Bass => Ok(vec![r.bass]),
        Alt => r
            .alt
            .map(|s| vec![s])
            .ok_or(BindError::NoAlternateBass { step }),
        Treble(n) => {
            let t = r.treble(n);
            if t.is_empty() {
                Err(BindError::NoTrebleStrings { step })
            } else {
                Ok(t)
            }
        }
        Absolute(s) => {
            let s = s.get();
            if s > strings {
                Err(BindError::StringOutOfRange {
                    step,
                    string: s,
                    strings,
                })
            } else if !r.frets.contains_key(&s) {
                Err(BindError::StringNotInVoicing { step, string: s })
            } else {
                Ok(vec![s])
            }
        }
        Strum => Ok(r.sounded.clone()),
        StrumRange(a, b) => {
            let (lo, hi) = (a.get().min(b.get()), a.get().max(b.get()));
            if hi > strings {
                return Err(BindError::StringOutOfRange {
                    step,
                    string: hi,
                    strings,
                });
            }
            let t: Vec<u8> = r
                .sounded
                .iter()
                .copied()
                .filter(|s| (lo..=hi).contains(s))
                .collect();
            if t.is_empty() {
                Err(BindError::EmptyStrike { step })
            } else {
                Ok(t)
            }
        }
    }
}

/// The union of the selectors' strings, string 1 first, or every error.
fn select_all(
    selectors: &[StringSelector],
    r: &Regions,
    strings: u8,
    step: StepRef,
    errors: &mut Vec<BindError>,
) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut ok = true;
    for sel in selectors {
        match select(*sel, r, strings, step) {
            Ok(s) => out.extend(s),
            Err(e) => {
                errors.push(e);
                ok = false;
            }
        }
    }
    if !ok {
        return None;
    }
    out.sort_unstable();
    out.dedup();
    if out.is_empty() {
        errors.push(BindError::EmptyStrike { step });
        return None;
    }
    Some(out)
}

fn add(effects: &mut NoteEffects, t: Technique) {
    match t {
        Technique::Accent => effects.accent = true,
        Technique::PalmMute => effects.palm_mute = true,
        Technique::Dead => effects.dead = true,
        Technique::LetRing => effects.let_ring = true,
        Technique::StringPercussive => effects.string_percussive = true,
    }
}

fn is_short(e: &NoteEffects) -> bool {
    e.dead || e.palm_mute || e.string_percussive
}

/// Binds a resolved pattern (gestures expanded) to one voicing.
///
/// - An attack's notes get the strike's techniques plus those of every span covering
///   the step; the beat gets the strike's direction.
/// - A hold's notes are tied, on the same strings and frets, and keep only the let
///   ring of the note they continue (an accent or span is not repeated on a tie).
pub fn bind(
    pattern: &ResolvedPattern,
    voicing: &Voicing,
    chord: &Chord,
    instrument: &FrettedInstrument,
) -> Result<PatternFragment, Vec<BindError>> {
    let r = regions(voicing, chord, instrument)?;
    let strings = instrument.tuning.string_count();
    let mut errors = Vec::new();
    let covering = |lane: LaneId, step: usize| {
        pattern
            .spans
            .iter()
            .filter(move |s: &&TechniqueSpan| s.lane == lane && s.first <= step && step <= s.last)
    };
    // A span must reach at least one attack.
    for (i, span) in pattern.spans.iter().enumerate() {
        let attacks = pattern
            .lanes
            .iter()
            .filter(|(l, _)| *l == span.lane)
            .flat_map(|(_, steps)| steps.iter().enumerate())
            .filter(|(k, s)| {
                span.first <= *k && *k <= span.last && matches!(s.action, ResolvedAction::Strike(_))
            })
            .count();
        if attacks == 0 {
            errors.push(BindError::SpanReachesNoAttack { span: i });
        }
    }
    let mut lanes = Vec::new();
    for (lane, steps) in &pattern.lanes {
        let mut beats = Vec::new();
        let mut held: Option<Vec<BoundNote>> = None;
        for (index, step) in steps.iter().enumerate() {
            let at = StepRef {
                lane: *lane,
                index,
                onset: step.onset,
            };
            match &step.action {
                ResolvedAction::Strike(k) => {
                    let mut effects = k.techniques;
                    for span in covering(*lane, index) {
                        add(&mut effects, span.technique);
                    }
                    held = None;
                    if let Some(chosen) = select_all(&k.targets, &r, strings, at, &mut errors) {
                        let notes: Vec<BoundNote> = chosen
                            .iter()
                            .map(|s| BoundNote {
                                position: FretPosition {
                                    string: StringIndex::new(*s).expect("s >= 1"),
                                    fret: r.frets[s],
                                },
                                tie: false,
                                effects,
                            })
                            .collect();
                        held = Some(notes.clone());
                        beats.push(BoundBeat {
                            step: index,
                            onset: step.onset,
                            duration: step.duration,
                            stroke: k.direction,
                            notes,
                        });
                    }
                }
                ResolvedAction::Hold(selectors) => {
                    // Validation guarantees a directly preceding strike or hold (PT-05);
                    // `held` is None only when that strike failed to bind (reported).
                    let Some(source) = held.take() else { continue };
                    let chosen: Option<Vec<u8>> = match selectors {
                        None => Some(source.iter().map(|n| n.position.string.get()).collect()),
                        Some(sel) => select_all(sel, &r, strings, at, &mut errors),
                    };
                    let Some(chosen) = chosen else { continue };
                    let mut notes = Vec::new();
                    for s in chosen {
                        // PT-05 (checked for every voicing) puts every held string in
                        // the source.
                        let Some(src) = source.iter().find(|n| n.position.string.get() == s) else {
                            continue;
                        };
                        if is_short(&src.effects) {
                            errors.push(BindError::HoldOfShortNote {
                                step: at,
                                string: s,
                            });
                            continue;
                        }
                        notes.push(BoundNote {
                            position: src.position,
                            tie: true,
                            effects: NoteEffects {
                                let_ring: src.effects.let_ring,
                                ..NoteEffects::default()
                            },
                        });
                    }
                    held = Some(notes.clone());
                    beats.push(BoundBeat {
                        step: index,
                        onset: step.onset,
                        duration: step.duration,
                        stroke: None,
                        notes,
                    });
                }
                ResolvedAction::Rest => held = None,
            }
        }
        lanes.push(BoundLane { lane: *lane, beats });
    }
    if errors.is_empty() {
        Ok(PatternFragment {
            pattern: pattern.id,
            length: pattern.length,
            meter: pattern.meter,
            lanes,
        })
    } else {
        Err(errors)
    }
}
