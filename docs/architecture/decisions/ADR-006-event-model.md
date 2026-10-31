# ADR-006 Event model: invalidation events, no global bus

Status: Accepted
Date: 2026-10-31
Related documents: [interop-architecture.md](../interop-architecture.md), [score-model-and-undo.md](../score-model-and-undo.md)

## Context
Dart must know when to refresh views. A global event bus tends to become a hidden coupling channel and a God Object.

## Decision
Aggregates return domain events; the session wraps them in envelopes (seq, document id, revision, correlation, causation) and publishes to a per-document stream. Events carry ids and ranges only. Dart re-queries projections. Gaps trigger full resync. Telemetry uses `tracing`. High-rate data uses separate latest-value streams.

## Alternatives
Global bus; content-carrying events (Dart mirrors state); polling; Rust-to-Dart callbacks.

## Trade-offs
Simple and robust (resync on gap); costs an extra query per invalidation.

## Consequences
No in-process subscribers inside Rust domain code. Cross-context reactions are orchestrated explicitly in `mc-app` handlers.

## Related fitness functions
FF-01 (domain cannot depend on app), interop contract tests.

## Reconsideration conditions
Multiple in-process consumers need the same events (for example analysis reacting to edits); collaboration needs operation logs.
