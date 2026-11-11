# Risk Register

Purpose: Track major technical and project risks with likelihood, impact, mitigation and owning phase.
Status: Living document. Reviewed at each phase exit.
Related documents: [implementation-phases.md](implementation-phases.md), [technical-spikes.md](../research/technical-spikes.md), [feasibility-assessment.md](../research/feasibility-assessment.md).

Scale: Likelihood and impact are Low, Medium, High.

| Id | Risk | Likelihood | Impact | Mitigation | Phase |
|---|---|---|---|---|---|
| R-01 | frb build or codegen friction on Windows; single-maintainer dependency; `freezed` toolchain for payload enums; unpinned helper tools | Medium (S1 passed; friction seen in template lockfile and tool installs) | High | frb isolated in `mc-ffi`; pinned codegen and `cargo-expand` in CI; C ABI fallback documented | 3 |
| R-02 | Audio glitches: engine work was clean in S2 (0 allocations, callback p99 at most 3.2 percent of period), but OS-level stalls up to 44 ms occurred even with thread priority promotion | Medium | High | Priority promotion (ADR-007); FF-18 interval-based glitch test; warm-up after SoundFont load; re-measure in Phase 4 on the reference machine (second machine deferred, ADR-025); consider render-ahead if stalls prove audible | 4 |
| R-03 | Flutter canvas performance for large TAB scores | Low (S4: frame p99 under 12 ms for 500 bars) | High | Virtualization; repaint boundaries; re-measure for standard notation in Phase 6c | 3, 6c |
| R-04 | TAB editor UX effort underestimated (largest effort item) | High | High | Fixed layout, keyboard-first input, scope list, upper estimate 10 weeks, one active TAB track at a time (Q-17) | 3 |
| R-05 | Dart text metrics and Rust layout mismatch | Medium | Medium | Metrics contract measured once per font and zoom; S4 | 3 |
| R-06 | Unknown content handling more complex than planned (U1 detection cost, U3 later) | Medium | Medium | S5 detection prototype and flatten cost measurement; U2 Save As protection in MVP; U3 deferred to format 1.1 | 1 |
| R-07 | Undo memory or latency on large scores | Low | Medium | Structural sharing; byte budget; benchmarks | 1 |
| R-08 | Audio device changes, permission denial, busy devices | Medium | Medium | Typed errors; stream rebuild; A10 tests | 4, 5 |
| R-09 | Chord naming ambiguity causes disagreement with users | Medium | Low | Golden set agreed with user; multiple names returned | 2 |
| R-10 | MIDI import string assignment produces unplayable TAB | High | Low | Report and playability heuristic; user edits | 4 |
| R-11 | Tuner unstable on low E or with noisy input | Medium | Medium | S3; window size tuning; clarity threshold | 5 |
| R-12 | Mobile packaging and audio latency | Medium | Medium | S12 before Phase 7 | 7 |
| R-13 | Engraving quality for standard notation consumes far more time than estimated | High | Medium | S11; consider third-party engine with license plan; keep TAB-first | 6c |
| R-14 | Guitar Pro format and legal uncertainty | Medium | Medium | S10 done (2026-11-11): GP3 to GP5 5.00 read and GP5 written, checked against TuxGuitar 2.0.1 only. Mitigation: clean-room implementation (ADR-036 section 2), refusal of unverified layouts, and import reports. Open: no file written by Guitar Pro, no 5.10 file, brush direction, text encoding, and a legal review before a commercial release | 6d |
| R-15 | AI features fail to reach useful quality | High | Medium | Candidates only; timeboxes; agreed quality thresholds | 9, 10 |
| R-16 | License issues in dependencies or bundled data (SoundFont, model weights) | Medium (S7: Rust trees clean except optional MPL-2.0 `audio_thread_priority`; SoundFont choice pending) | High | ADR-015; cargo-deny in CI; own MMCSS call instead of MPL crate; FluidR3 GM (MIT) proposed; Dart audit in Phase 3; S9 for model weights | 0, 3, 9 |
| R-21 | Measurements come from one machine; lower-end or different audio hardware may behave worse | Medium | Medium | Open, accepted and deferred ([ADR-025](../architecture/decisions/ADR-025-single-machine-evidence-accepted.md)): no second machine available; not a condition for M4. Every FF-18 run records the machine configuration in `benchmarks/`. Re-run S2, S4 and FF-18 on another Windows machine when one is available, at the latest before the first public release | 4, release |
| R-17 | Solo developer capacity and motivation over 19 to 31 FTE weeks | High | High | Usable milestones every phase; CLI harness gives early value; scope cuts listed per phase | All |
| R-18 | Governance overhead slows a solo developer | Medium | Medium | Activate fitness functions per phase only; nightly for slow checks; review crate count at M5 | All |
| R-19 | Web support harder than expected later | Medium | Low | FF-04 for domain crates; S6 optional | 11 |
| R-20 | macOS minimum version from cpal (14.2) limits users | Medium | Low | Verify in port phase; Q-13 | Post-MVP |
