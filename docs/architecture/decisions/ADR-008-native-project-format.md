# ADR-008 Native project format: ZIP + JSON with versioned DTOs

Status: Proposed (Accepted when S5 passes)
Date: 2026-10-31
Related documents: [project-format.md](../project-format.md), [data-architecture.md](../data-architecture.md)

## Context
Interchange formats cannot hold the full domain. Users need a single shareable file. The format must evolve safely.

## Decision
Single-file ZIP package with `manifest.json`, `score.json` and reserved folders. JSON via serde into versioned DTOs in `mc-project`, converted to domain types through validating builders. Major and minor versioning, required and optional features, unknown-field preservation via `flatten` on every DTO, atomic save with backup, autosave recovery, strict ZIP reading limits.

## Alternatives
Directory project; SQLite document; binary serialization (CBOR, bincode); MusicXML or GP as canonical.

## Trade-offs
Shareable and inspectable; whole-file rewrite on save and not git-diffable. Unknown-field preservation adds DTO complexity.

## Consequences
DTOs are a deliberate second representation. A fixture corpus per released version is maintained forever.

## Related fitness functions
FF-11, FF-15, FF-32.

## Reconsideration conditions
Large audio projects (save times over budget); users need version control friendliness; collaboration.
