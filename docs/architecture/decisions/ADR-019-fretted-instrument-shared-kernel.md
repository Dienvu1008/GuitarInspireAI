# ADR-019 Fretted-instrument value objects in the shared kernel

Status: Accepted
Date: 2026-10-31
Related documents: [bounded-contexts.md](../bounded-contexts.md), [domain-map.md](../domain-map.md), [architecture-overview.md](../architecture-overview.md)

## Context
The earlier design had `mc-score` depend on `mc-guitar` for Tuning, string and fret types, making the core domain depend on a supporting context.

## Decision
Put fretted-instrument value objects (`StringIndex`, `Fret`, `Capo`, `Tuning`, `FretPosition`, `pitch_at`) in `mc-theory` module `fretted`. `mc-score` and `mc-guitar` depend only on `mc-theory`. Fretted notes store string and fret as authoritative data; pitch is derived. Retune semantics are a Guitar service applied via app-layer commands. Kernel rule: value objects and pure arithmetic only.

## Alternatives
A: score depends on guitar. B: pitch-only score with guitar-derived positions (loses user-authored string choice). C: new `mc-instrument` crate now.

## Trade-offs
No new crate and no core-to-supporting edge. The kernel becomes slightly instrument-aware.

## Consequences
Kernel changes are reviewed as shared-kernel changes (GR-13). Non-fretted instruments add new `InstrumentSpec` and `NoteValue` variants.

## Related fitness functions
FF-01 (no `mc-score -> mc-guitar` edge).

## Reconsideration conditions
Instrument ranges and technique catalogues, drum kits, transposing instruments, or a third context needing instrument data: extract `mc-instrument`.
