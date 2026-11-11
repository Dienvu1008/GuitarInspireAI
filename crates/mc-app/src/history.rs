//! Undo and redo with structural-sharing snapshots (ADR-016). Each entry stores the
//! score before a step; snapshots share unchanged tracks and bars with each other.
//!
//! From 6a-5 an entry may also hold the pattern library before the step (ADR-034
//! section 22). It is stored only for steps that changed the library, so score edits
//! never copy the library.

use mc_pattern::PatternLibrary;
use mc_score::Score;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryLimits {
    /// Default 200 steps (score-model-and-undo section 5).
    pub max_steps: usize,
    /// Default 64 MB of estimated unshared bytes.
    pub max_bytes: usize,
}

impl Default for HistoryLimits {
    fn default() -> Self {
        HistoryLimits {
            max_steps: 200,
            max_bytes: 64 << 20,
        }
    }
}

/// The document state a history entry restores: the score, and the library if the
/// step changed it.
#[derive(Debug, Clone)]
pub(crate) struct State {
    pub(crate) score: Score,
    pub(crate) patterns: Option<PatternLibrary>,
}

#[derive(Debug, Clone)]
struct Entry {
    before: State,
    /// Estimated bytes this snapshot holds that the next state does not share.
    bytes: usize,
    coalesce_key: Option<u64>,
}

/// A rough size of a library copy, for the byte budget: libraries are small next to
/// scores, but a step that copies one must still count.
fn library_bytes(lib: &PatternLibrary) -> usize {
    let steps: usize = lib
        .patterns()
        .map(|p| {
            p.parts()
                .lanes
                .iter()
                .map(|(_, l)| l.steps.len())
                .sum::<usize>()
        })
        .sum();
    256 + 96 * lib.gestures().count() + 256 * lib.patterns().count() + 96 * steps
}

fn state_bytes(before: &State, after: &Score) -> usize {
    before.score.unshared_heap_bytes(after) + before.patterns.as_ref().map_or(0, library_bytes)
}

#[derive(Debug, Clone, Default)]
pub(crate) struct History {
    undo: VecDeque<Entry>,
    redo: Vec<Entry>,
    bytes: usize,
    limits: HistoryLimits,
}

impl History {
    pub(crate) fn new(limits: HistoryLimits) -> Self {
        History {
            limits,
            ..History::default()
        }
    }

    /// Records a committed step. `patterns` is the library before the step, if the step
    /// changed it. If `key` matches the previous step and nothing was undone in between,
    /// the steps merge into one (the older snapshot is kept).
    pub(crate) fn record(
        &mut self,
        before: Score,
        patterns: Option<PatternLibrary>,
        after: &Score,
        key: Option<u64>,
    ) {
        self.redo.clear();
        if key.is_some() && self.undo.back().is_some_and(|e| e.coalesce_key == key) {
            let last = self.undo.back_mut().expect("checked");
            // The library before the merged step is the older one, if the older step
            // changed it; otherwise the library before this step.
            if last.before.patterns.is_none() {
                last.before.patterns = patterns;
            }
            let new_bytes = state_bytes(&last.before, after);
            self.bytes = self.bytes - last.bytes + new_bytes;
            last.bytes = new_bytes;
            return;
        }
        let before = State {
            score: before,
            patterns,
        };
        let bytes = state_bytes(&before, after);
        self.undo.push_back(Entry {
            before,
            bytes,
            coalesce_key: key,
        });
        self.bytes += bytes;
        // The most recent step is always kept, even if it alone exceeds the byte budget,
        // so the user can always undo the last edit.
        while self.undo.len() > self.limits.max_steps
            || (self.bytes > self.limits.max_bytes && self.undo.len() > 1)
        {
            let dropped = self.undo.pop_front().expect("non-empty");
            self.bytes -= dropped.bytes;
        }
    }

    /// Returns the state to restore, taking the current one onto the redo stack. The
    /// current library is copied only if the restored step changed it.
    pub(crate) fn undo(&mut self, current: Score, patterns: &PatternLibrary) -> Option<State> {
        let entry = self.undo.pop_back()?;
        self.bytes -= entry.bytes;
        let back = State {
            patterns: entry.before.patterns.as_ref().map(|_| patterns.clone()),
            score: current,
        };
        let bytes = state_bytes(&back, &entry.before.score);
        self.redo.push(Entry {
            before: back,
            bytes,
            coalesce_key: None,
        });
        Some(entry.before)
    }

    pub(crate) fn redo(&mut self, current: Score, patterns: &PatternLibrary) -> Option<State> {
        let entry = self.redo.pop()?;
        let back = State {
            patterns: entry.before.patterns.as_ref().map(|_| patterns.clone()),
            score: current,
        };
        let bytes = state_bytes(&back, &entry.before.score);
        self.undo.push_back(Entry {
            before: back,
            bytes,
            coalesce_key: None,
        });
        self.bytes += bytes;
        Some(entry.before)
    }

    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub(crate) fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub(crate) fn estimated_bytes(&self) -> usize {
        self.bytes
    }

    /// Stops the current coalescing run (for example after a save or a cursor move).
    pub(crate) fn break_coalescing(&mut self) {
        if let Some(e) = self.undo.back_mut() {
            e.coalesce_key = None;
        }
    }
}
