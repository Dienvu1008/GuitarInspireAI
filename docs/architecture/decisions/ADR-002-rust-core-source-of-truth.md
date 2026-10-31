# ADR-002 Rust Music Core is the source of truth

Status: Accepted
Date: 2026-10-31
Related documents: [interop-architecture.md](../interop-architecture.md), [cross-platform-ui.md](../cross-platform-ui.md)

## Context
Music data has many invariants. Implementing them in both Dart and Rust would cause drift. A reusable Music Core SDK and CLI are long-term goals.

## Decision
All music data and rules live in Rust. Dart holds opaque document handles, UI-only state (cursor, selection, in-progress input), and cached read projections invalidated by events.

## Alternatives
Duplicate models in Dart and Rust; Dart model with Rust only for DSP.

## Trade-offs
One implementation of invariants, testable without UI. Costs a query round trip for every view refresh and requires projection design per view.

## Consequences
Every UI view needs a projection query. Widget tests use a fake facade. Performance of command plus projection must be measured (S1, S4).

## Related fitness functions
FF-05, FF-12, interop benchmarks in [interop-architecture.md](../interop-architecture.md).

## Reconsideration conditions
Measured projection latency cannot meet frame budgets even with caching and batching.
