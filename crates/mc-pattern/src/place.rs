//! Placing (ADR-034 section 3 stage 4, section 7): a bound fragment laid over a run of
//! whole master bars. Never stretched or squeezed (Q-03): whole repetitions, or an
//! explicit fit. Steps crossing a bar line are split into tied values that sum exactly;
//! gaps become rests (PT-09). The result is the content of voices 1 and 2 per bar.

use mc_score::{FragmentBeat, FragmentNote, MasterBar, NoteEffects, Stroke};
use mc_theory::{BaseValue, NoteValue, Rational, TimeSignature, Tuplet};

use crate::bind::{BoundNote, PatternFragment};
use crate::model::LaneId;

/// What to do when the range is not a whole multiple of the pattern (ADR-034 section
/// 7). There is no default: the caller must choose (decision 13.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    /// Full repetitions, then the last one cut at the range end.
    RepeatAndCut,
    /// Whole repetitions only; the rest of the range is left as rests.
    WholeRepetitionsOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceError {
    EmptyRange,
    /// A pickup bar is in the range; its intended length is not stored.
    PickupInRange {
        bar: usize,
    },
    /// The pattern declares a meter and this bar has another time signature.
    MeterMismatch {
        bar: usize,
        meter: TimeSignature,
        found: TimeSignature,
    },
    /// The range is not a whole multiple of the pattern and no fit was given.
    FitRequired {
        range: Rational,
        pattern: Rational,
    },
    /// A tuplet step would cross a bar line; a tuplet cannot be split.
    TupletCrossesBar {
        lane: LaneId,
        repetition: usize,
        step: usize,
        bar: usize,
    },
    /// A dead, palm-muted or string-percussive note would cross a bar line. The tied
    /// continuation would be played as a new attack (ADR-024, ADR-035).
    ShortNoteCrossesBar {
        lane: LaneId,
        repetition: usize,
        step: usize,
        bar: usize,
    },
    /// A computed length (a gap, a split or a cut) cannot be written as note values.
    UnwritableLength {
        lane: LaneId,
        at: Rational,
        length: Rational,
    },
    /// Arithmetic out of range.
    Overflow,
}

/// One step that `RepeatAndCut` dropped or shortened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepChange {
    pub lane: LaneId,
    pub repetition: usize,
    pub step: usize,
    /// The step's onset in the pattern.
    pub onset: Rational,
    pub written: NoteValue,
    /// The values it is now written with (tied), or empty when dropped.
    pub now: Vec<NoteValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PlaceReport {
    /// Repetitions started in the range, the cut one included.
    pub repetitions: usize,
    pub dropped: Vec<StepChange>,
    pub shortened: Vec<StepChange>,
    /// Attacked notes written (tied continuations not counted).
    pub notes: usize,
    /// With `WholeRepetitionsOnly`: where the trailing rests begin, from the range start.
    pub rests_from: Option<Rational>,
}

/// The content of one bar of the range: one beat list per lane, in voice order
/// (Strum or Thumb = voice 1, Fingers = voice 2). Each list fills the bar exactly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedBar {
    pub voices: Vec<(LaneId, Vec<FragmentBeat>)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub bars: Vec<PlacedBar>,
    pub report: PlaceReport,
}

/// One sounding event in range time, before bar splitting.
struct Event {
    start: Rational,
    length: Rational,
    /// The step's written value when it is not cut (keeps tuplets and dots).
    written: Option<NoteValue>,
    notes: Vec<BoundNote>,
    stroke: Option<Stroke>,
    repetition: usize,
    step: usize,
}

fn add(a: Rational, b: Rational) -> Result<Rational, PlaceError> {
    a.checked_add(b).ok_or(PlaceError::Overflow)
}
fn sub(a: Rational, b: Rational) -> Result<Rational, PlaceError> {
    a.checked_sub(b).ok_or(PlaceError::Overflow)
}

/// Writes a length exactly as note values, longest first:
/// - the dyadic part as whole notes, then at most one plain value per binary digit,
///   down to 1/256;
/// - a length with one factor 3 in its reduced denominator also gets one triplet
///   value for the remainder.
///
/// `None` if that is impossible (another prime factor, 9 in the denominator, or a
/// value below 1/256).
pub fn values_for(length: Rational) -> Option<Vec<NoteValue>> {
    if !length.is_positive() {
        return None;
    }
    let (p, mut q) = (length.numerator(), length.denominator());
    let triplet = q % 3 == 0;
    if triplet {
        q /= 3;
    }
    if q & (q - 1) != 0 {
        return None; // not a power of two: another prime factor, or 9
    }
    let a = q.trailing_zeros() as i64; // q = 2^a
    // p / (3 * 2^a) = s / 2^a + t / (3 * 2^a), t in {0, 1, 2}.
    let (mut rest, tail) = if triplet { (p / 3, p % 3) } else { (p, 0) };
    let mut out = Vec::new();
    let whole = 1i64 << a;
    while rest >= whole {
        out.push(NoteValue::plain(BaseValue::WHOLE));
        rest -= whole;
    }
    for e in 1..=a.min(8) {
        let unit = 1i64 << (a - e);
        if rest >= unit {
            out.push(NoteValue::plain(BaseValue::from_exponent(e as u8).ok()?));
            rest -= unit;
        }
    }
    if rest != 0 {
        return None;
    }
    if tail > 0 {
        // A triplet of base 2^-b lasts 2 / (3 * 2^b): b = a for t = 2, b = a + 1 for t = 1.
        let b = a + 2 - tail;
        if b > 8 {
            return None;
        }
        out.push(
            NoteValue::new(
                BaseValue::from_exponent(b as u8).ok()?,
                0,
                Some(Tuplet::TRIPLET),
            )
            .ok()?,
        );
    }
    Some(out)
}
fn short(n: &BoundNote) -> bool {
    n.effects.dead || n.effects.palm_mute || n.effects.string_percussive
}

/// Places a bound fragment over `bars` (the master bars of the range, in order).
pub fn place(
    fragment: &PatternFragment,
    bars: &[MasterBar],
    fit: Option<Fit>,
) -> Result<Placement, PlaceError> {
    if bars.is_empty() {
        return Err(PlaceError::EmptyRange);
    }
    if let Some(k) = bars.iter().position(|b| b.pickup) {
        return Err(PlaceError::PickupInRange { bar: k });
    }
    if let Some(meter) = fragment.meter
        && let Some(k) = bars.iter().position(|b| b.time_signature != meter)
    {
        return Err(PlaceError::MeterMismatch {
            bar: k,
            meter,
            found: bars[k].time_signature,
        });
    }
    // Bar starts in range time, nominal durations only.
    let mut starts = vec![Rational::ZERO];
    for b in bars {
        let last = *starts.last().expect("non-empty");
        starts.push(add(last, b.time_signature.length())?);
    }
    let range = *starts.last().expect("non-empty");
    let pattern = fragment.length;
    let ratio = range.checked_div(pattern).ok_or(PlaceError::Overflow)?;
    let whole = ratio.numerator() / ratio.denominator(); // floor, both positive
    let exact = ratio.denominator() == 1;
    let (repetitions, cut_at, rests_from) = match (exact, fit) {
        (true, _) => (whole as usize, range, None),
        (false, None) => return Err(PlaceError::FitRequired { range, pattern }),
        (false, Some(Fit::RepeatAndCut)) => (whole as usize + 1, range, None),
        (false, Some(Fit::WholeRepetitionsOnly)) => {
            let end = pattern
                .checked_mul(Rational::from_int(whole))
                .ok_or(PlaceError::Overflow)?;
            (whole as usize, end, Some(end))
        }
    };
    let mut report = PlaceReport {
        repetitions,
        rests_from,
        ..PlaceReport::default()
    };
    let mut voices: Vec<(LaneId, Vec<Vec<FragmentBeat>>)> = Vec::new();
    for lane in &fragment.lanes {
        // Events of all repetitions, in time order.
        let mut events = Vec::new();
        for rep in 0..repetitions {
            let offset = pattern
                .checked_mul(Rational::from_int(rep as i64))
                .ok_or(PlaceError::Overflow)?;
            for beat in &lane.beats {
                let start = add(offset, beat.onset)?;
                let length = beat.duration.length();
                let end = add(start, length)?;
                let change = |now: Vec<NoteValue>| StepChange {
                    lane: lane.lane,
                    repetition: rep,
                    step: beat.step,
                    onset: beat.onset,
                    written: beat.duration,
                    now,
                };
                if start >= cut_at {
                    report.dropped.push(change(Vec::new()));
                    continue;
                }
                if end > cut_at {
                    // Only RepeatAndCut reaches here (whole repetitions end at cut_at).
                    if beat.duration.tuplet().is_some() {
                        report.dropped.push(change(Vec::new()));
                        continue;
                    }
                    let left = sub(cut_at, start)?;
                    let now = values_for(left).ok_or(PlaceError::UnwritableLength {
                        lane: lane.lane,
                        at: start,
                        length: left,
                    })?;
                    report.shortened.push(change(now));
                    events.push(Event {
                        start,
                        length: left,
                        written: None,
                        notes: beat.notes.clone(),
                        stroke: beat.stroke,
                        repetition: rep,
                        step: beat.step,
                    });
                    continue;
                }
                events.push(Event {
                    start,
                    length,
                    written: Some(beat.duration),
                    notes: beat.notes.clone(),
                    stroke: beat.stroke,
                    repetition: rep,
                    step: beat.step,
                });
            }
        }
        report.notes += events
            .iter()
            .map(|e| e.notes.iter().filter(|n| !n.tie).count())
            .sum::<usize>();
        voices.push((lane.lane, lay_out(lane.lane, &events, &starts)?));
    }
    let bars_out = (0..bars.len())
        .map(|k| PlacedBar {
            voices: voices
                .iter()
                .map(|(lane, per_bar)| (*lane, per_bar[k].clone()))
                .collect(),
        })
        .collect();
    Ok(Placement {
        bars: bars_out,
        report,
    })
}

fn rests(
    lane: LaneId,
    at: Rational,
    length: Rational,
    out: &mut Vec<FragmentBeat>,
) -> Result<(), PlaceError> {
    if length.is_zero() {
        return Ok(());
    }
    let values = values_for(length).ok_or(PlaceError::UnwritableLength { lane, at, length })?;
    for value in values {
        out.push(FragmentBeat {
            value,
            notes: Vec::new(),
            stroke: None,
        });
    }
    Ok(())
}

/// Splits events at bar lines and fills gaps with rests, bar by bar.
fn lay_out(
    lane: LaneId,
    events: &[Event],
    starts: &[Rational],
) -> Result<Vec<Vec<FragmentBeat>>, PlaceError> {
    let bar_count = starts.len() - 1;
    let mut out: Vec<Vec<FragmentBeat>> = vec![Vec::new(); bar_count];
    let bar_of = |t: Rational| {
        starts[1..]
            .iter()
            .position(|s| t < *s)
            .unwrap_or(bar_count - 1)
    };
    // Cursor per bar: where the written content of the bar ends so far.
    let mut cursor = Rational::ZERO;
    for ev in events {
        let mut bar = bar_of(ev.start);
        // Rests from the cursor to the event start, split at bar lines.
        while cursor < ev.start {
            let cb = bar_of(cursor);
            let until = ev.start.min(starts[cb + 1]);
            rests(lane, cursor, sub(until, cursor)?, &mut out[cb])?;
            cursor = until;
        }
        let end = add(ev.start, ev.length)?;
        let mut piece_start = ev.start;
        let mut first = true;
        while piece_start < end {
            let piece_end = end.min(starts[bar + 1]);
            let crosses = piece_end < end;
            if crosses || !first {
                if ev.written.is_some_and(|w| w.tuplet().is_some()) {
                    return Err(PlaceError::TupletCrossesBar {
                        lane,
                        repetition: ev.repetition,
                        step: ev.step,
                        bar,
                    });
                }
                if ev.notes.iter().any(short) {
                    return Err(PlaceError::ShortNoteCrossesBar {
                        lane,
                        repetition: ev.repetition,
                        step: ev.step,
                        bar,
                    });
                }
            }
            let length = sub(piece_end, piece_start)?;
            let values = match (first && !crosses, ev.written) {
                (true, Some(w)) => vec![w],
                _ => values_for(length).ok_or(PlaceError::UnwritableLength {
                    lane,
                    at: piece_start,
                    length,
                })?,
            };
            for (k, value) in values.into_iter().enumerate() {
                let attack = first && k == 0;
                out[bar].push(FragmentBeat {
                    value,
                    notes: ev
                        .notes
                        .iter()
                        .map(|n| FragmentNote {
                            position: n.position,
                            // The first value keeps the bound tie (a hold); later values
                            // continue it.
                            tie: if attack { n.tie } else { true },
                            effects: if attack {
                                n.effects
                            } else {
                                NoteEffects {
                                    let_ring: n.effects.let_ring,
                                    ..NoteEffects::default()
                                }
                            },
                        })
                        .collect(),
                    stroke: if attack { ev.stroke } else { None },
                });
            }
            first = false;
            piece_start = piece_end;
            if piece_start < end {
                bar += 1;
            }
        }
        cursor = end;
    }
    // Trailing rests to the range end.
    let range = starts[bar_count];
    while cursor < range {
        let cb = bar_of(cursor);
        let until = starts[cb + 1];
        rests(lane, cursor, sub(until, cursor)?, &mut out[cb])?;
        cursor = until;
    }
    Ok(out)
}
