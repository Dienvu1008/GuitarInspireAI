# ADR-012 Interchange formats are adapters

Status: Accepted
Date: 2026-10-31
Related documents: [data-architecture.md](../data-architecture.md), [project-format.md](../project-format.md), [domain-invariants.md](../domain-invariants.md)

## Context
MIDI, MusicXML and Guitar Pro each lose information relative to the domain model, and evolve outside our control.

## Decision
Interchange formats are import and export adapters only. Imports build scores through `ScoreBuilder` and produce an ImportReport; exports read snapshots and produce an ExportReport. No interchange format is canonical.

## Alternatives
MusicXML or GP7 as canonical storage.

## Trade-offs
Full control of the domain model; each adapter is extra work and is lossy by design.

## Consequences
Adapters are fuzzed and fixture-tested. Users see reports of lossy conversions.

## Related fitness functions
FF-11, FF-15, IM-01 tests.

## Reconsideration conditions
None expected; individual adapter scope reviewed per phase.
