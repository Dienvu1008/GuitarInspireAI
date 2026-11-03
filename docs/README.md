# GuitarInspireAI Documentation

Purpose: Entry point and navigation for the architecture, research and roadmap package.
Status: Accepted (documentation baseline, pre-implementation).
Related documents: [Glossary](glossary.md), [Architecture summary](architecture/README.md), [Roadmap](roadmap/README.md).

This package is the accepted pre-implementation baseline (2026-10-31). M0 is complete: governance tooling (`xtask`, `architecture/`, `deny.toml`, CI workflow) and throwaway spikes (`spikes/`) exist; no production crates or packages exist yet. Results: [m0-exit-report.md](roadmap/m0-exit-report.md).

## Conventions

- Every document starts with Purpose, Status and Related documents.
- Fact labels: `[Verified: source]`, `[Assumption]`, `[To verify]`.
- Decision labels: Accepted, Proposed, Deferred.
- Governance rules are identified as GR-01 to GR-14, fitness functions as FF-xx, ADRs as ADR-001 to ADR-020, spikes as S1 to S12, audio assumptions as A1 to A10.
- Effort is stated in full-time-equivalent (FTE) weeks only.

## Reading paths

| Reader goal | Start with | Then |
|---|---|---|
| Understand the system in 10 minutes | [architecture/README.md](architecture/README.md) | [architecture-overview.md](architecture/architecture-overview.md) |
| Work on the music domain | [domain-map.md](architecture/domain-map.md) | [bounded-contexts.md](architecture/bounded-contexts.md), [domain-invariants.md](architecture/domain-invariants.md), [score-model-and-undo.md](architecture/score-model-and-undo.md) |
| Work on Flutter and Rust integration | [interop-architecture.md](architecture/interop-architecture.md) | [cross-platform-ui.md](architecture/cross-platform-ui.md) |
| Work on audio | [audio-dsp-architecture.md](architecture/audio-dsp-architecture.md) | [performance-requirements.md](architecture/performance-requirements.md) |
| Work on storage | [data-architecture.md](architecture/data-architecture.md) | [project-format.md](architecture/project-format.md) |
| Change an architecture rule | [dependency-rules.md](architecture/dependency-rules.md) | [fitness-functions.md](architecture/fitness-functions.md), [decisions/README.md](architecture/decisions/README.md) |
| Plan work | [roadmap/README.md](roadmap/README.md) | [implementation-phases.md](roadmap/implementation-phases.md), [milestones.md](roadmap/milestones.md) |

## Document index

### Architecture
- [architecture/README.md](architecture/README.md): executive summary, review findings and revisions.
- [system-context.md](architecture/system-context.md): actors, external systems, scope boundary.
- [architecture-overview.md](architecture/architecture-overview.md): style, crate DAG, Dart packages, runtime view.
- [domain-map.md](architecture/domain-map.md): context map.
- [bounded-contexts.md](architecture/bounded-contexts.md): per-context model.
- [score-model-and-undo.md](architecture/score-model-and-undo.md): session, aggregate, structures, undo.
- [domain-invariants.md](architecture/domain-invariants.md): invariant catalogue and categories.
- [guitar-pattern-engine.md](architecture/guitar-pattern-engine.md): pattern IR and DSL ambiguity.
- [dependency-rules.md](architecture/dependency-rules.md): GR-01 to GR-14.
- [interop-architecture.md](architecture/interop-architecture.md): Dart and Rust communication.
- [data-architecture.md](architecture/data-architecture.md): storage strategy.
- [project-format.md](architecture/project-format.md): native project format.
- [audio-dsp-architecture.md](architecture/audio-dsp-architecture.md): real-time audio and DSP.
- [ai-music-architecture.md](architecture/ai-music-architecture.md): MIR, ML and provenance.
- [cross-platform-ui.md](architecture/cross-platform-ui.md): adaptive Flutter UI.
- [security-and-privacy.md](architecture/security-and-privacy.md): threat surface and privacy.
- [performance-requirements.md](architecture/performance-requirements.md): budgets and degradation.
- [fitness-functions.md](architecture/fitness-functions.md): automated governance.
- [decisions/README.md](architecture/decisions/README.md): ADR process and index.

### Roadmap
- [roadmap/README.md](roadmap/README.md): MVP definition.
- [implementation-phases.md](roadmap/implementation-phases.md): phases 0 to 11.
- [milestones.md](roadmap/milestones.md): M0 to M5 and later.
- [risk-register.md](roadmap/risk-register.md): risks.
- [dependency-map.md](roadmap/dependency-map.md): phase and spike dependencies.
- [m0-exit-report.md](roadmap/m0-exit-report.md): M0 results and Phase 1 recommendation.
- [phase-1-exit-report.md](roadmap/phase-1-exit-report.md): Phase 1 evidence, benchmarks and decisions.
- [phase-2-exit-report.md](roadmap/phase-2-exit-report.md): Phase 2 (Guitar core) evidence, golden set, open interpretations and ambiguous cases.
- [phase-3-exit-report.md](roadmap/phase-3-exit-report.md): Phase 3 (desktop TAB editor) evidence, edit-feedback measurements, defects fixed, open points and the manual test script.

### Research
- [technology-evaluation.md](research/technology-evaluation.md): libraries and tools.
- [feasibility-assessment.md](research/feasibility-assessment.md): what is achievable.
- [alternative-architectures.md](research/alternative-architectures.md): stack alternatives.
- [technical-spikes.md](research/technical-spikes.md): S1 to S12.
- [open-questions.md](research/open-questions.md): unresolved questions.

### Agent guidance
- `.kiro/steering/architecture-guardrails.md`: always-included rules for coding agents (GR-14).
