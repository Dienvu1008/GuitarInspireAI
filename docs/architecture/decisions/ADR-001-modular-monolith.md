# ADR-001 Modular monolith

Status: Accepted
Date: 2026-10-31
Related documents: [architecture-overview.md](../architecture-overview.md), [alternative-architectures.md](../../research/alternative-architectures.md)

## Context
Solo developer, local-first desktop app, low-latency in-process interaction between UI, music model and audio. No independent scaling or team ownership needs.

## Decision
Single process. One Rust Cargo workspace with crates as modules and one Flutter app with a small set of Dart packages. Boundaries enforced by Cargo dependencies, visibility and fitness functions.

## Alternatives
Microservices; core as a sidecar process with IPC; a single large crate without internal boundaries.

## Trade-offs
Gains simplicity, performance and one deployable. Loses process-level crash isolation (an ML or synth crash takes down the app) and independent deployment.

## Consequences
Boundaries rely on discipline plus automation (FF-01 to FF-06). A crash in native code affects the whole app, so untrusted parsers are fuzzed.

## Related fitness functions
FF-01, FF-02, FF-05, FF-06.

## Reconsideration conditions
Out-of-process ML runtime (Python or large native runtime) needed; third-party plugins requiring isolation; a server product sharing the core.
