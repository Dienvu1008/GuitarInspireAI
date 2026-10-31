# ADR-016 Undo via structural-sharing snapshots

Status: Accepted (memory and latency budgets provisional until Phase 1 benchmarks)
Date: 2026-10-31
Related documents: [score-model-and-undo.md](../score-model-and-undo.md), [performance-requirements.md](../performance-requirements.md)

## Context
Undo must be correct for every command. Full-clone snapshots are too expensive for large scores (15 to 30 MB each [Assumption]). Inverse commands are bug-prone.

## Decision
Score internals use `Arc` per track and per bar content with copy-on-write (`Arc::make_mut`). Each committed command pushes the previous score value (a cheap clone) onto the undo stack. Stack bounded by step count (default 200) and approximate bytes (default 64 MB). Coalescing rules live in `mc-app`. The same snapshots feed save, autosave and playback compilation.

## Alternatives
Full-clone snapshots; inverse commands or patches; event sourcing; persistent data structure crates (for example `im`).

## Trade-offs
Generic, correct undo with small code; `Arc` indirection on reads and some memory overhead per bar. Bulk edits copy all touched bars.

## Consequences
Domain structures are designed for sharing from Phase 1. Snapshots are immutable and safe to send to background threads.

## Related fitness functions
FF-14, FF-21, FF-23.

## Reconsideration conditions
Benchmarks show budget violations on the large reference score; collaboration requires operation-based history; bars become too coarse (for example very long bars with frequent edits).
