# M0 Exit Report

Purpose: Record the outcome of M0 (Phase 0 foundation and spikes S1, S2, S4, S5, S7): pass and fail results, unresolved risks, decisions needed, and the recommendation on starting Phase 1.
Status: Final for M0, 2026-10-31.
Related documents: [technical-spikes.md](../research/technical-spikes.md) (full results), [milestones.md](milestones.md), [implementation-phases.md](implementation-phases.md), [risk-register.md](risk-register.md), [decisions/README.md](../architecture/decisions/README.md), [benchmarks/reference-machine.md](../../benchmarks/reference-machine.md).

## 1. Recommendation

Start Phase 1. No spike result invalidates the architecture. ADR-005, ADR-007, ADR-008 and ADR-018 are now Accepted with evidence-based amendments; no other ADR changed. Phase 1 scope is unchanged.

Two user decisions are open. Neither blocks Phase 1; both block Phase 4 (playback):

1. Thread priority promotion: own `windows-sys` MMCSS call (recommended, permissive) or cpal's `realtime` feature (adds one MPL-2.0 crate). See [ADR-015](../architecture/decisions/ADR-015-dependency-license-policy.md).
2. Default SoundFont (Q-11): FluidR3 GM (MIT) recommended. GeneralUser GS is permissive, but its author states that some sample origins are uncertain.

## 2. Deliverables

| Deliverable | State |
|---|---|
| Root Cargo workspace (governance tooling only), pinned toolchain 1.98.0 | Done |
| `xtask` with FF-19 (trailers), FF-20 (inventory), FF-33 (links); 17 self-tests using temporary git repositories | Done |
| `architecture/fitness-inventory.toml` (33 checks: 5 active, 28 planned), `architecture/protected-paths.txt` | Done |
| `deny.toml` with the ADR-015 allow-list (FF-09, FF-10) | Done |
| `tools/mermaid-check` with pinned mermaid 11.17.2 and jsdom 25.0.1 (FF-33) | Done |
| `.github/workflows/ci.yml`: Windows governance job and Linux cargo-deny job | Written; not yet run on GitHub |
| `benchmarks/reference-machine.md` | Done |
| Spikes S1, S2, S4, S5 in `spikes/` (excluded from the workspace) and S7 audit | Done; raw output in each `results/` folder |
| Spike results and ADR updates | Done |

No production crate, Dart package or Phase 1 feature was created.

## 3. Results

| Spike | Verdict | Key evidence | Failed or partial criteria |
|---|---|---|---|
| S1 frb interop | Pass with one failed sub-criterion | Round trip p99 160 µs; 20 502 events gapless; mirroring from an frb-free crate works; 60 Hz for 10 min at 59.99999 Hz; UI frame p99 2.3 ms | One delivery gap of 56 ms (criterion: none over 50 ms). Mitigation: Dart cursor extrapolation (ADR-005) |
| S2 real-time playback | Pass with conditions | 0 allocations in all runs, including a 261-preset GM bank; callback p99 at most 3.2 percent of the 10 ms period; 99 swaps clean; 440.0000 Hz at both rates; play to sound 28 to 55 ms | A4 partially met: priority promotion is required, and rare OS stalls up to 44 ms remain (1 of 5 runs with promotion) |
| S4 TAB rendering | Pass | 500 bars: frame p99 11.3 ms and 9.8 ms, 0 frames over 33 ms; layout 5 µs per system, about 50 µs with FFI | None |
| S5 project format | Pass | Typical open 7 to 8 ms, large 178 to 249 ms; U1 detected 55 of 55 at all 8 levels; all malicious ZIPs rejected; rename with retry works under simulated locks | None (real OneDrive and antivirus untested) |
| S7 license audit | Pass with one decision | All recommended trees pass cargo-deny 0.20.2; no copyleft license selected | cpal `realtime` pulls MPL-2.0 (decision 1 above) |

Fitness functions active in M0 (local runs): FF-09 and FF-10 pass on the root workspace and on every recommended spike tree. FF-19, FF-20 and FF-33 pass: 17 xtask self-tests, 53 Markdown files with 368 relative links and 0 broken, and 11 Mermaid diagrams with 0 failures. `cargo clippy -D warnings` and `cargo fmt --check` pass for `xtask`.

## 4. Phase 0 exit criteria

| Criterion ([implementation-phases.md](implementation-phases.md)) | Result |
|---|---|
| Each spike has a recorded result against its success criteria | Met ([technical-spikes.md](../research/technical-spikes.md) Results) |
| CI fails on a forbidden dependency | Met locally: cargo-deny rejected the MPL-2.0 crate with the project policy. Not yet observed in GitHub Actions |
| CI fails on a protected-path change without a trailer | Met locally: xtask tests cover a missing trailer, an unknown ADR, deletion of a protected file, and an attempt to empty the protected list. In a throwaway clone, the real M0 change committed without a trailer was rejected (16 protected files listed) and passed with the trailer in section 8. Not yet observed in GitHub Actions |
| No result invalidates the architecture, or affected ADRs are revised | Met: four ADRs amended, none reversed |

## 5. What M0 did not verify

- GitHub Actions execution (the workflow has not run; first run happens on the next push).
- Real OneDrive sync, real antivirus holds, network shares and FAT volumes for atomic save.
- Audible quality by listening; gap detection is inferred from timing.
- Behaviour on any machine other than the reference machine (R-21).
- Tuner assumptions A8 to A10 (S3, Phase 5).
- Web build (S6, optional), exclusive-mode WASAPI, panic unwinding across the cpal callback.
- Dart dependency licenses (Dart half of FF-09, Phase 3).
- FTE estimates: the spikes stayed inside their timeboxes, but this execution does not recalibrate human effort estimates.

## 6. Unresolved risks

| Risk | Status after M0 |
|---|---|
| R-02 audio glitches | Lowered for engine design; residual OS stalls up to 44 ms with promotion. Re-test in Phase 4 on a second machine |
| R-01 frb toolchain | Works; costs are `freezed`, pinned helper tools and stale template lockfiles |
| R-04 TAB editor effort | Unchanged (the largest effort item); S4 removed the rendering-performance part of the risk only |
| R-16 licenses | Rust trees clean; MPL decision and SoundFont pending; Dart audit in Phase 3 |
| R-21 single-machine evidence | New |

## 7. Phase 1 entry notes

- Prerequisites met: ADR-008, ADR-011, ADR-016, ADR-019 and ADR-020 Accepted; Q-07, Q-09 and Q-17 resolved.
- Still needed before Phase 1 exit: Q-08 (file extension).
- Phase 1 activates 16 fitness functions (see the activation table in [fitness-functions.md](../architecture/fitness-functions.md)). Before adding the first crate, add `architecture/allowed-deps.toml` and the FF-01 xtask check, and set `current_phase = 1` in the inventory when those checks are active.
- Carry-overs from the evidence: `std::fs::rename` with retry for saves, zip deflate-only features, a 100:1 ratio cap, U1 detection and U2 Save As protection, and the PB-01 to PB-08 tests for effective bar duration.
- Spike code is not copied into production. Reimplement against the architecture.

## 8. Committing M0

M0 changes protected paths: `architecture/`, `deny.toml`, `xtask/src/fitness/`, `.github/workflows/`, several ADR files and `docs/architecture/fitness-functions.md`. FF-19 therefore requires a trailer on the commit, for example:

```text
M0: governance foundation, spikes S1 S2 S4 S5, S7 audit

Architecture-Change: ADR-005, ADR-007, ADR-008, ADR-015, ADR-018, ADR-020
```
