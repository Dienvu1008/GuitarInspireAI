//! Semantic validation (ADR-034 section 3, stage 2): pure, with no voicing. Checks
//! PT-01 and PT-03 to PT-07 and returns every violation, in step order.

use mc_theory::Rational;

use crate::error::{PatternError, StepRef, StrikeAt};
use crate::ids::GestureId;
use crate::model::{Action, Lane, LaneId, PatternParts, Strike, StringSelector};

/// Looks up the strike of a gesture.
pub(crate) trait Gestures {
    fn strike(&self, id: GestureId) -> Option<&Strike>;
}

/// Structural checks of one strike: at least one target, and `Treble(n)` with n >= 1.
pub(crate) fn check_strike(strike: &Strike, at: &StrikeAt, out: &mut Vec<PatternError>) {
    if strike.targets.is_empty() {
        out.push(PatternError::EmptyTargets { at: at.clone() });
    }
    check_selectors(&strike.targets, at, out);
}

fn check_selectors(selectors: &[StringSelector], at: &StrikeAt, out: &mut Vec<PatternError>) {
    for s in selectors {
        if *s == StringSelector::Treble(0) {
            out.push(PatternError::InvalidSelector {
                at: at.clone(),
                selector: *s,
            });
        }
    }
}

/// True when `outer` selects `inner` for every voicing, so a hold of `inner` after a
/// strike of `outer` can never ask for a string the strike did not play (PT-05).
/// Containment that depends on the voicing counts as not contained.
pub fn covers(outer: StringSelector, inner: StringSelector) -> bool {
    use StringSelector::*;
    let within = |a: u8, b: u8, s: u8| a.min(b) <= s && s <= a.max(b);
    if outer == inner {
        return true;
    }
    match (outer, inner) {
        (Strum, _) => true,
        (StrumRange(a, b), Absolute(s)) => within(a.get(), b.get(), s.get()),
        (StrumRange(a, b), StrumRange(p, q)) => {
            within(a.get(), b.get(), p.get()) && within(a.get(), b.get(), q.get())
        }
        (Treble(n), Treble(m)) => m <= n,
        _ => false,
    }
}

/// What a hold can continue at the current position of a lane.
enum Held {
    /// Nothing: the lane starts here, or a rest or a gap came before.
    Nothing,
    /// A strike whose targets are unknown (an unknown gesture, already reported).
    Unknown,
    /// The selectors still sounding.
    Set(Vec<StringSelector>),
}

pub(crate) fn validate(parts: &PatternParts, gestures: &dyn Gestures) -> Vec<PatternError> {
    let mut out = Vec::new();
    if !parts.length.is_positive() {
        out.push(PatternError::NonPositiveLength {
            length: parts.length,
        });
    }
    for (id, lane) in parts.lanes.iter() {
        validate_lane(id, lane, parts.length, gestures, &mut out);
    }
    for (index, span) in parts.spans.iter().enumerate() {
        match parts.lanes.get(span.lane) {
            None => out.push(PatternError::SpanLaneMissing {
                span: index,
                lane: span.lane,
            }),
            Some(lane) => {
                if span.first > span.last || span.last >= lane.steps.len() {
                    out.push(PatternError::SpanOutOfRange {
                        span: index,
                        lane: span.lane,
                        first: span.first,
                        last: span.last,
                        steps: lane.steps.len(),
                    });
                }
            }
        }
    }
    out
}

fn validate_lane(
    id: LaneId,
    lane: &Lane,
    length: Rational,
    gestures: &dyn Gestures,
    out: &mut Vec<PatternError>,
) {
    let mut previous: Option<(Rational, Option<Rational>)> = None; // (onset, end)
    let mut held = Held::Nothing;
    for (index, step) in lane.steps.iter().enumerate() {
        let at = StepRef {
            lane: id,
            index,
            onset: step.onset,
        };
        // PT-04
        if step.onset.is_negative() {
            out.push(PatternError::NegativeOnset { step: at });
        }
        let end = step.onset.checked_add(step.duration.length());
        match end {
            None => out.push(PatternError::TimeOverflow { step: at }),
            // PT-01
            Some(end) if end > length => out.push(PatternError::OutsidePattern {
                step: at,
                end,
                length,
            }),
            Some(_) => {}
        }
        // PT-03
        let mut contiguous = false;
        if let Some((prev_onset, prev_end)) = previous {
            if step.onset < prev_onset {
                out.push(PatternError::OutOfOrder {
                    step: at,
                    previous_onset: prev_onset,
                });
            } else if let Some(prev_end) = prev_end {
                if step.onset < prev_end {
                    out.push(PatternError::Overlap {
                        step: at,
                        previous_end: prev_end,
                    });
                }
                contiguous = step.onset == prev_end;
            }
        }
        // A gap before this step is a rest (PT-09): nothing is held across it.
        if !contiguous {
            held = Held::Nothing;
        }
        let site = StrikeAt::Step(at);
        held = match &step.action {
            Action::Strike(strike) => {
                check_strike(strike, &site, out);
                Held::Set(strike.targets.clone())
            }
            // PT-06; the gesture's own strike was checked when it was defined.
            Action::Gesture(g) => match gestures.strike(*g) {
                Some(strike) => Held::Set(strike.targets.clone()),
                None => {
                    out.push(PatternError::UnknownGesture {
                        step: at,
                        gesture: *g,
                    });
                    Held::Unknown
                }
            },
            // PT-05
            Action::Hold(selectors) => check_hold(at, selectors.as_deref(), held, out),
            Action::Rest => Held::Nothing,
        };
        previous = Some((step.onset, end));
    }
}

fn check_hold(
    at: StepRef,
    selectors: Option<&[StringSelector]>,
    held: Held,
    out: &mut Vec<PatternError>,
) -> Held {
    let site = StrikeAt::Step(at);
    if let Some(s) = selectors {
        if s.is_empty() {
            out.push(PatternError::EmptyTargets { at: site.clone() });
        }
        check_selectors(s, &site, out);
    }
    match (held, selectors) {
        (Held::Nothing, _) => {
            out.push(PatternError::HoldWithoutStrike { step: at });
            Held::Nothing
        }
        (Held::Unknown, _) => Held::Unknown,
        (Held::Set(set), None) => Held::Set(set),
        (Held::Set(set), Some(s)) => {
            for h in s {
                if !set.iter().any(|o| covers(*o, *h)) {
                    out.push(PatternError::HoldNotInStrike {
                        step: at,
                        selector: *h,
                    });
                }
            }
            // A hold narrows what the next hold can continue.
            Held::Set(s.to_vec())
        }
    }
}
