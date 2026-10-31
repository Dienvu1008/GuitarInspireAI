# ADR-013 AI provenance and candidate layer

Status: Accepted (principle), Deferred (implementation to Phase 8)
Date: 2026-10-31
Related documents: [ai-music-architecture.md](../ai-music-architecture.md), [domain-invariants.md](../domain-invariants.md)

## Context
AI and MIR outputs are uncertain. They must not silently overwrite user content and must be traceable.

## Decision
Analysis produces `Candidate<T>` with provenance (source hash, producer id and version, artifact hash, parameters, confidence, status). Only a user `AcceptCandidate` command converts candidates into ordinary validated score commands. Accepted content records its origin.

## Alternatives
AI writes directly into the score with undo as the safety net; no provenance.

## Trade-offs
Safe and auditable; more UI for review and acceptance.

## Consequences
Analysis crates cannot depend on score mutation APIs.

## Related fitness functions
FF-01, FF-31.

## Reconsideration conditions
None for the principle. Schema details revisited in Phase 8.
