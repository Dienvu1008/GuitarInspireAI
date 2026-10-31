# ADR-003 Context map and crate DAG

Status: Accepted
Date: 2026-10-31
Related documents: [domain-map.md](../domain-map.md), [bounded-contexts.md](../bounded-contexts.md), [architecture-overview.md](../architecture-overview.md), [dependency-rules.md](../dependency-rules.md)

## Context
Initial hypotheses listed eight contexts. Several (Notation, Audio, Workspace) are projections, technical capabilities or UI state rather than domains.

## Decision
Contexts: Music Theory (shared kernel), Composition (core), Guitar, Pattern Engine (Phase 6), Project, Playback, Notation Layout (projection), DSP, Analysis (Phase 8). One crate per context, introduced only when needed, with the DAG in [architecture-overview.md](../architecture-overview.md) and the allowed matrix in [dependency-rules.md](../dependency-rules.md).

## Alternatives
One crate with modules; a crate per hypothesis context; layers as crates (domain, app, infra) rather than contexts.

## Trade-offs
Compiler-enforced boundaries and fast incremental builds, at the cost of more Cargo.toml files and public API design per crate.

## Consequences
11 crates by MVP. Crates under about 300 lines at Phase 5 are reviewed for merging.

## Related fitness functions
FF-01, FF-02, FF-22, FF-25.

## Reconsideration conditions
A crate remains trivially small; a cycle pressure appears (an edge needed against the DAG); new instruments require `mc-instrument` (see ADR-019).
