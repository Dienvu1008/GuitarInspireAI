# Score Model, Document Session and Undo

Purpose: Separate the document session, aggregate boundary, internal data structures, persistence structures and read projections; evaluate the single Score aggregate and undo strategies for large scores.
Status: Accepted (single aggregate, structural-sharing snapshots). Sizes and costs are [Assumption] until Phase 1 benchmarks.
Related documents: [bounded-contexts.md](bounded-contexts.md), [domain-invariants.md](domain-invariants.md), [interop-architecture.md](interop-architecture.md), [project-format.md](project-format.md), [ADR-016](decisions/ADR-016-undo-structural-sharing-snapshots.md), [performance-requirements.md](performance-requirements.md).

## 1. Five distinct concepts

| Concept | Owner | What it is | What it is not |
|---|---|---|---|
| Document session | `mc-app` | One open project: current Score, undo and redo stacks, revision counter, dirty flag, event sequence counter, last saved revision. Single writer: commands for one document are serialized | Not a domain object. Not persisted |
| Aggregate boundary | `mc-score` | `Score` is the consistency boundary: every committed state satisfies hard invariants | Not a performance or locking unit |
| Internal data structures | `mc-score` | Arc-based tree enabling cheap snapshots | Not the persisted shape |
| Persistence structures | `mc-project` | Versioned DTOs, may be normalized differently (for example per-track entries later) | Not used in domain logic |
| Read projections | `mc-notation`, `mc-app` queries | TabLayout per (track, bar range), TrackSummary, BarView; cached by (track, bar, revision) | Never mutated, never authoritative |

## 2. Why one Score aggregate

Invariants cross tracks: every track aligns with the master bar list (H-09), time signature and tempo changes are song-level, and inserting or deleting a master bar must update all tracks atomically.

The usual reasons to split aggregates (database write contention, transactional load, distributed writers) do not apply to a single-user, in-memory, single-writer editor. Splitting now would require cross-aggregate consistency protocols for no measured benefit.

Risks of one aggregate and their mitigations:

| Risk | Mitigation |
|---|---|
| Cloning the whole score on each edit | Copy-on-write via `Arc::make_mut` (section 3) |
| Full re-validation on each edit | Commands validate only affected bars; full validation on load and in tests |
| Large events or projections | Events carry ranges, not content; projections are per bar range |
| Background tasks (save, playback compile) block editing | They operate on immutable snapshots (`Arc` clones), never on the live score |

Reconsideration conditions (any one triggers an ADR review):

- Real-time collaboration (CRDT or OT) is planned.
- Linked multi-document parts (part extraction with shared edits).
- Scores beyond the large reference size with measured budget violations.
- Background writers that must mutate concurrently (for example, AI suggestions applied live).

## 3. Internal structure (illustrative)

```rust
pub struct Score {
    master_bars: Arc<Vec<MasterBar>>,
    tracks: Vec<Arc<Track>>,
}
pub struct Track {
    id: TrackId,
    instrument: InstrumentSpec,
    bars: Vec<Arc<BarContent>>, // len == master_bars.len()
}
pub struct BarContent { voices: Vec<Voice> }
pub struct Voice { beats: Vec<Beat> }
pub struct Beat { id: BeatId, duration: NoteValue, notes: Vec<Note> }
pub struct Note { id: NoteId, value: NoteValueKind, effects: NoteEffects }
```

Editing one bar: `Arc::make_mut` on the track (copies its bar pointer vector) and on the bar (copies one bar). All other tracks and bars stay shared with previous snapshots. Ids are stable across edits and saves.

## 4. Size reasoning [Assumption, to benchmark in Phase 1]

| Reference score | Shape | Notes | In-memory estimate |
|---|---|---|---|
| Small | 1 track x 50 bars | about 1k | under 0.2 MB |
| Typical | 4 tracks x 150 bars | about 10k | under 1 MB |
| Large (GP-style band score) | 20 tracks x 300 bars x about 16 beats x about 3 notes | about 288k | 15 to 30 MB |

Measured in Phase 1 (reference machine, `benchmarks/baselines/memory.toml`; structural bytes, excluding strings and allocator overhead): typical 0.39 MB, large 8.8 MB. One undo step for a single-note edit retains 1.9 KB (typical) and 3.9 KB (large), in line with the section 5 estimate. 200 undo steps of such edits on the large score hold about 0.8 MB.

## 5. Undo and redo options

| Option | Memory per step (large score) | Complexity | Assessment |
|---|---|---|---|
| (a) Full-clone snapshots | 15 to 30 MB; GBs for 100 levels | Lowest | Rejected for large scores |
| (b) Structural-sharing snapshots | Changed track spine (300 pointers, about 2.4 KB) plus changed bars (about 1 KB each) [Assumption]; 100 levels well under 1 MB for typical edits | Low: undo is generic, no per-command inverse | Recommended |
| (c) Inverse commands or patches | Minimal | High: every command needs a correct inverse; easy to get wrong | Rejected as default; may be used for bulk edits if (b) is too costly |
| (d) Event sourcing | Event log plus checkpoints | High | Rejected (overkill now) |

Under (b), a snapshot is `score.clone()`: one `Arc` increment for master bars and one per track. Bulk edits (transpose whole song) copy all touched bars once per command, which is acceptable because it is one undo step.

Undo stack policy: bounded by step count (default 200, configurable) and an approximate byte budget (default 64 MB). The most recent step is always kept, even if it alone exceeds the byte budget, so the last edit can always be undone. Coalescing: commands with the same `coalesce_key` are one undo step until another key, an undo or a save breaks the run; the mechanism is in `mc-app`, and the exact typing rules are decided in Phase 3 (Q-15).

Dirty state (Phase 1): a document is dirty when its content differs from the content last saved or opened, not when its revision differs. Undoing back to the saved state makes it clean again, while the revision stays monotonic (P-04). The comparison is cheap because unchanged tracks and bars are shared and compare by pointer.

Snapshots also serve autosave and save: the save thread serializes an `Arc` snapshot while editing continues.

## 6. Benchmarks (Phase 1, criterion)

| Benchmark | Metric | Purpose |
|---|---|---|
| apply_note_edit (small, typical, large) | p50 and p99 latency | Command budget (16 ms total with projection) |
| snapshot_memory_per_step | bytes retained per undo step | Validate section 5 estimates |
| undo_redo_100 | latency, equality with original | Correctness and cost |
| bulk_transpose_large | latency, bytes | Worst-case single step |
| full_validate_large | latency | Load budget (500 ms) |

Budgets are provisional until baselines exist ([performance-requirements.md](performance-requirements.md)).

## 7. Property tests

- Random command sequences: all hard invariants hold after each committed command.
- Undo of N commands followed by redo of N produces a structurally equal score.
- Undo of all commands returns the initial score.
- Rejected commands leave the score and revision unchanged.
