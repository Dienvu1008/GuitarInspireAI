# Risk Register

Purpose: Track major technical and project risks with likelihood, impact, mitigation and owning phase.
Status: Living document. Reviewed at each phase exit.
Related documents: [implementation-phases.md](implementation-phases.md), [technical-spikes.md](../research/technical-spikes.md), [feasibility-assessment.md](../research/feasibility-assessment.md).

Scale: Likelihood and impact are Low, Medium, High.

| Id | Risk | Likelihood | Impact | Mitigation | Phase |
|---|---|---|---|---|---|
| R-01 | frb build or codegen friction on Windows; single-maintainer dependency | Medium | High | S1; frb isolated in `mc-ffi`; C ABI fallback documented | 0, 3 |
| R-02 | Synth or audio path allocates or blocks in callback, causing glitches | Medium | High | S2 with assert_no_alloc; `Synth` trait for replacement; preallocated voices | 0, 4 |
| R-03 | Flutter canvas performance for large TAB scores | Low to Medium | High | S4; virtualization; repaint boundaries; texture fallback | 0, 3 |
| R-04 | TAB editor UX effort underestimated (largest effort item) | High | High | Fixed layout, keyboard-first input, scope list, upper estimate 10 weeks, cut multi-track view to Q-17 answer | 3 |
| R-05 | Dart text metrics and Rust layout mismatch | Medium | Medium | Metrics contract measured once per font and zoom; S4 | 3 |
| R-06 | Unknown-field preservation more complex than planned | Medium | Medium | S5 prototype; force Save As when preservation not possible | 1 |
| R-07 | Undo memory or latency on large scores | Low | Medium | Structural sharing; byte budget; benchmarks | 1 |
| R-08 | Audio device changes, permission denial, busy devices | Medium | Medium | Typed errors; stream rebuild; A10 tests | 4, 5 |
| R-09 | Chord naming ambiguity causes disagreement with users | Medium | Low | Golden set agreed with user; multiple names returned | 2 |
| R-10 | MIDI import string assignment produces unplayable TAB | High | Low | Report and playability heuristic; user edits | 4 |
| R-11 | Tuner unstable on low E or with noisy input | Medium | Medium | S3; window size tuning; clarity threshold | 5 |
| R-12 | Mobile packaging and audio latency | Medium | Medium | S12 before Phase 7 | 7 |
| R-13 | Engraving quality for standard notation consumes far more time than estimated | High | Medium | S11; consider third-party engine with license plan; keep TAB-first | 6c |
| R-14 | Guitar Pro format and legal uncertainty | Medium | Medium | S10; clean-room implementation; reports | 6d |
| R-15 | AI features fail to reach useful quality | High | Medium | Candidates only; timeboxes; agreed quality thresholds | 9, 10 |
| R-16 | License issues in dependencies or bundled data (SoundFont, model weights) | Medium | High | ADR-015; cargo-deny; S7, S9 | 0, 9 |
| R-17 | Solo developer capacity and motivation over 19 to 31 FTE weeks | High | High | Usable milestones every phase; CLI harness gives early value; scope cuts listed per phase | All |
| R-18 | Governance overhead slows a solo developer | Medium | Medium | Activate fitness functions per phase only; nightly for slow checks; review crate count at M5 | All |
| R-19 | Web support harder than expected later | Medium | Low | FF-04 for domain crates; S6 optional | 11 |
| R-20 | macOS minimum version from cpal (14.2) limits users | Medium | Low | Verify in port phase; Q-13 | Post-MVP |
