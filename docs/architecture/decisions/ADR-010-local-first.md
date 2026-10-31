# ADR-010 Local-first, no network in MVP

Status: Accepted
Date: 2026-10-31
Related documents: [security-and-privacy.md](../security-and-privacy.md), [data-architecture.md](../data-architecture.md)

## Context
Music practice and composition must work offline. Network features add privacy, security and operational burden.

## Decision
All MVP features work offline. No network calls, no telemetry. The project file is the unit of ownership and future sync.

## Alternatives
Cloud-first with accounts; optional telemetry from day one.

## Trade-offs
No usage data and no sync early; simpler, private, no server costs.

## Consequences
Crash diagnosis relies on local logs the user may share manually.

## Related fitness functions
FF-02 can ban HTTP client crates in MVP (for example `reqwest`, `ureq`) [Proposed].

## Reconsideration conditions
Cloud sync, remote AI or collaboration phases (Phase 11), each with explicit opt-in.
