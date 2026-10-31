//! Undo and redo with structural-sharing snapshots (ADR-016). Each entry stores the
//! score before a step; snapshots share unchanged tracks and bars with each other.

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

#[derive(Debug, Clone)]
struct Entry {
    before: Score,
    /// Estimated bytes this snapshot holds that the next state does not share.
    bytes: usize,
    coalesce_key: Option<u64>,
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

    /// Records a committed step. If `key` matches the previous step and nothing was
    /// undone in between, the steps merge into one (the older snapshot is kept).
    pub(crate) fn record(&mut self, before: Score, after: &Score, key: Option<u64>) {
        self.redo.clear();
        if key.is_some() && self.undo.back().is_some_and(|e| e.coalesce_key == key) {
            let last = self.undo.back_mut().expect("checked");
            let new_bytes = last.before.unshared_heap_bytes(after);
            self.bytes = self.bytes - last.bytes + new_bytes;
            last.bytes = new_bytes;
            return;
        }
        let bytes = before.unshared_heap_bytes(after);
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

    /// Returns the score to restore, taking `current` onto the redo stack.
    pub(crate) fn undo(&mut self, current: Score) -> Option<Score> {
        let entry = self.undo.pop_back()?;
        self.bytes -= entry.bytes;
        let bytes = current.unshared_heap_bytes(&entry.before);
        self.redo.push(Entry {
            before: current,
            bytes,
            coalesce_key: None,
        });
        Some(entry.before)
    }

    pub(crate) fn redo(&mut self, current: Score) -> Option<Score> {
        let entry = self.redo.pop()?;
        let bytes = current.unshared_heap_bytes(&entry.before);
        self.undo.push_back(Entry {
            before: current,
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
