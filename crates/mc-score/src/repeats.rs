//! Repeat structure (H-12) and playback order.
//!
//! Semantics (Guitar Pro style, no nesting):
//! - A section starts at a `repeat_start` bar, at the score start, or after the
//!   previous completed repeat.
//! - A `repeat_end(n)` bar jumps back to the section start until the section has been
//!   played `n` times.
//! - A bar with `alternate_endings` is played only on the listed passes. Alternate bars
//!   may sit inside the section or directly after its end bar.

use crate::{MasterBar, Violation};

/// Maximum number of played bars after expansion (audio-dsp-architecture 3a).
pub const MAX_PLAYED_BARS: usize = 10_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaybackOrder {
    /// Master bar indices in playback order.
    pub bars: Vec<usize>,
    /// True if expansion stopped at `MAX_PLAYED_BARS`.
    pub truncated: bool,
}

pub(crate) fn check(bars: &[MasterBar]) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut push = |i: usize, m: String| {
        out.push(Violation {
            rule: "H-12",
            location: format!("master bar {i}"),
            message: m,
        })
    };
    let mut open_start: Option<usize> = None;
    let mut last_end: Option<(usize, u8)> = None; // (bar, count) of the most recent repeat end
    for (i, b) in bars.iter().enumerate() {
        if b.repeat_start {
            if let Some(s) = open_start {
                push(
                    i,
                    format!(
                        "repeat start inside the open repeat started at bar {s} (nesting is not supported)"
                    ),
                );
            }
            open_start = Some(i);
        }
        if let Some(n) = b.repeat_end
            && !(2..=16).contains(&n)
        {
            push(i, format!("repeat count {n} outside 2..=16"));
        }
        if !b.alternate_endings.is_empty() {
            let a = &b.alternate_endings;
            if a.iter().any(|&p| !(1..=16).contains(&p)) || a.windows(2).any(|w| w[0] >= w[1]) {
                push(
                    i,
                    "alternate ending numbers must be 1..=16 and strictly increasing".into(),
                );
            }
            match owning_repeat_count(bars, i, last_end) {
                None => push(i, "alternate ending outside a repeated section".into()),
                Some(count) if a.iter().any(|&p| p > count) => push(
                    i,
                    format!("alternate ending number exceeds the repeat count {count}"),
                ),
                _ => {}
            }
        }
        if let Some(n) = b.repeat_end {
            open_start = None;
            last_end = Some((i, n));
        }
    }
    out
}

/// Repeat count of the section an alternate bar belongs to: a later end bar in the same
/// section, or the end bar directly before a contiguous run of alternate bars.
fn owning_repeat_count(bars: &[MasterBar], i: usize, last_end: Option<(usize, u8)>) -> Option<u8> {
    for (j, b) in bars.iter().enumerate().skip(i) {
        if j > i && b.repeat_start {
            break;
        }
        if let Some(n) = b.repeat_end {
            return Some(n);
        }
    }
    let (end, n) = last_end?;
    let contiguous = bars[end + 1..i]
        .iter()
        .all(|b| !b.alternate_endings.is_empty());
    contiguous.then_some(n)
}

pub(crate) fn playback_order(bars: &[MasterBar]) -> PlaybackOrder {
    let mut order = Vec::new();
    let mut section_start = 0usize;
    let mut pass: u8 = 1;
    let mut jumped = false;
    let mut completed = false;
    let mut i = 0usize;
    while i < bars.len() {
        if order.len() >= MAX_PLAYED_BARS {
            return PlaybackOrder {
                bars: order,
                truncated: true,
            };
        }
        let b = &bars[i];
        let is_alternate = !b.alternate_endings.is_empty();
        if completed && !is_alternate {
            section_start = i;
            pass = 1;
            completed = false;
        }
        if b.repeat_start && !(jumped && i == section_start) {
            section_start = i;
            pass = 1;
            completed = false;
        }
        jumped = false;
        if is_alternate && !b.alternate_endings.contains(&pass) {
            i += 1;
            continue;
        }
        order.push(i);
        match b.repeat_end {
            Some(count) if pass < count => {
                pass += 1;
                i = section_start;
                jumped = true;
            }
            Some(_) => {
                completed = true;
                i += 1;
            }
            None => i += 1,
        }
    }
    PlaybackOrder {
        bars: order,
        truncated: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars(n: usize) -> Vec<MasterBar> {
        vec![MasterBar::default(); n]
    }

    #[test]
    fn plain_score_plays_in_order() {
        assert_eq!(playback_order(&bars(3)).bars, [0, 1, 2]);
    }

    #[test]
    fn simple_repeat_plays_twice() {
        let mut b = bars(4);
        b[1].repeat_start = true;
        b[2].repeat_end = Some(2);
        assert_eq!(playback_order(&b).bars, [0, 1, 2, 1, 2, 3]);
        assert!(check(&b).is_empty());
    }

    #[test]
    fn repeat_end_without_start_repeats_from_the_beginning() {
        let mut b = bars(3);
        b[1].repeat_end = Some(3);
        assert_eq!(playback_order(&b).bars, [0, 1, 0, 1, 0, 1, 2]);
    }

    #[test]
    fn alternate_endings() {
        // |: A | B (1.) :| C (2.) | D
        let mut b = bars(4);
        b[0].repeat_start = true;
        b[1].alternate_endings = vec![1];
        b[1].repeat_end = Some(2);
        b[2].alternate_endings = vec![2];
        assert_eq!(playback_order(&b).bars, [0, 1, 0, 2, 3]);
        assert!(check(&b).is_empty(), "{:?}", check(&b));
    }

    #[test]
    fn h12_rejects_nesting_bad_counts_and_orphan_endings() {
        let mut nested = bars(4);
        nested[0].repeat_start = true;
        nested[1].repeat_start = true;
        nested[2].repeat_end = Some(2);
        assert_eq!(check(&nested).len(), 1);

        let mut count = bars(2);
        count[1].repeat_end = Some(1);
        assert_eq!(check(&count)[0].rule, "H-12");

        let mut orphan = bars(3);
        orphan[1].alternate_endings = vec![1];
        assert_eq!(check(&orphan).len(), 1);

        let mut too_high = bars(3);
        too_high[0].alternate_endings = vec![3];
        too_high[0].repeat_end = Some(2);
        assert_eq!(check(&too_high).len(), 1);

        let mut unsorted = bars(2);
        unsorted[0].alternate_endings = vec![2, 1];
        unsorted[0].repeat_end = Some(2);
        assert_eq!(check(&unsorted).len(), 1);
    }

    #[test]
    fn expansion_is_capped() {
        let mut b = bars(1000);
        b[999].repeat_end = Some(16);
        let o = playback_order(&b);
        assert!(o.truncated);
        assert_eq!(o.bars.len(), MAX_PLAYED_BARS);
    }
}
