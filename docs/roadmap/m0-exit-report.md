# M0 Exit Report

Purpose: Record the outcome of M0 (Phase 0 foundation and spikes S1, S2, S4, S5, S7): exact spike status, ADR changes caused by evidence, user decisions, remaining risks, CI validation status, and the Phase 1 recommendation.
Status: Final. M0 accepted by the user on 2026-10-31 as a successful risk-reduction milestone. Phase 1 is authorized once the commit recording this report passes CI.
Related documents: [technical-spikes.md](../research/technical-spikes.md) (full results), [milestones.md](milestones.md), [implementation-phases.md](implementation-phases.md), [risk-register.md](risk-register.md), [decisions/README.md](../architecture/decisions/README.md), [benchmarks/reference-machine.md](../../benchmarks/reference-machine.md).

## 1. Outcome

No spike result invalidates the architecture. Four ADRs were amended because of evidence; none was reversed. Phase 1 scope is unchanged. All measurements come from one reference machine (Windows 11, i5-12400F).

## 2. Spike status

| Spike | Status | Evidence | Exceptions and conditions |
|---|---|---|---|
| S1 frb interop | Pass for Phase 1 with an observed timing exception | No-op round trip p99 160 µs; sync query p99 3 µs; 20 502 events gapless and in order; mirroring from an frb-free crate works for structs and payload enums; 60 Hz stream for 10 min at 59.99999 Hz, 36 000 of 36 000 delivered; UI frame p99 2.3 ms | Failed sub-criterion: one delivery gap of 56 ms against a 50 ms limit, measured at the Dart listener of the position stream (details in technical-spikes S1). Not a blocker: the stream is UI control-plane data, not a real-time audio path. Kept as a monitoring metric in the interop benchmarks |
| S2 real-time playback | Accepted with conditions | 0 allocations in every run, including a 261-preset GM bank; callback p99 at most 3.2 percent of the 10 ms period; 99 timeline swaps without leaks or regressions; 440.0000 Hz at 44.1 and 48 kHz; estimated play to sound 28 to 55 ms | A4 partially met. Thread priority promotion is required. With it, rare OS-level stalls up to 44 ms remain (1 of 5 runs). cpal reports no underruns on WASAPI shared render, so callback timing stays part of audio diagnostics and testing |
| S4 TAB rendering | Pass, accepted | 500 bars: frame p99 11.3 ms and 9.8 ms over two passes, 0 frames over 33 ms; Rust layout 5 µs per system, about 50 µs with FFI; 1.5 KB per system | None |
| S5 project format | Pass, accepted | Typical open 7 to 8 ms, large 178 to 249 ms; U1 detected 55 of 55 at all 8 levels; all malicious ZIP cases rejected; `std::fs::rename` with retry works under simulated locks | Real OneDrive, antivirus holds, network shares and FAT not tested |
| S7 license audit | Accepted, subject to the two decisions in section 4 (now made) | All recommended dependency trees pass cargo-deny 0.20.2 with the ADR-015 policy; no copyleft license is selected | cpal `realtime` feature excluded (MPL-2.0) by decision 1 |

S3 (tuner), S6 (Web) and S8 to S12 were not part of M0.

## 3. ADR changes caused by evidence

| ADR | New status | What changed and why |
|---|---|---|
| [ADR-005](../architecture/decisions/ADR-005-dart-rust-interop-frb.md) | Accepted with amendments | S1: stream lifecycle owned by Rust (Dart `cancel()` did not complete while Rust held the sink); numeric FFI fields must map to Dart `int` (`u64` maps to `BigInt`); Dart `freezed` toolchain required for payload enums; pinned codegen and `cargo-expand` in CI; Dart cursor extrapolation between position updates |
| [ADR-007](../architecture/decisions/ADR-007-audio-thread-isolation.md) | Accepted with amendments; residual risk documented | S2: mandatory thread priority promotion, implemented as a small Windows-only module in `mc-audio` (decision 1); callback-timing-based underrun detection; WASAPI shared period fixed at 10 ms with `BufferSize::Fixed` ignored; warm-up after SoundFont load |
| [ADR-008](../architecture/decisions/ADR-008-native-project-format.md) | Accepted with amendments | S5: `std::fs::rename` with bounded retry (`tempfile::persist` rejected); `zip` with deflate only; decompression ratio cap 100:1 |
| [ADR-018](../architecture/decisions/ADR-018-notation-layout-in-rust.md) | Accepted | S4: layout and FFI cost negligible for TAB; synchronous layout calls acceptable; re-measure for standard notation |
| [ADR-015](../architecture/decisions/ADR-015-dependency-license-policy.md) | Accepted; decisions recorded | S7: allow-list unchanged; `allow-wildcard-paths = true` for internal path dependencies; MPL-2.0 rejected; FluidR3 GM with provenance conditions |
| [ADR-020](../architecture/decisions/ADR-020-invariants-vs-diagnostics.md) | Accepted | Not evidence-driven: user confirmation of Q-09 and the effective bar duration rule |

## 4. Decisions (user, 2026-10-31)

1. Audio priority: use the small Windows-specific `windows-sys` call (`AvSetMmThreadCharacteristicsW("Audio")`), not cpal's `realtime` feature, which adds an MPL-2.0 dependency. The code lives in a small platform module of the `mc-audio` adapter, outside the Music Core domain. No cross-platform priority abstraction now; other platforms are handled when they are supported.
2. Default SoundFont: FluidR3 GM (MIT), preferring clear distribution rights over marginal quality differences. When bundled (Phase 4): store the original license text, record provenance (source URL, version, size, SHA-256) in the repository, and include it in release third-party notices. User-supplied SoundFonts remain a future capability.

## 5. Remaining known risks

| Risk | State after M0 |
|---|---|
| R-02 audio glitches | Engine work is clean; residual OS-level stalls up to 44 ms with priority promotion on the reference machine. Re-test in Phase 4, including a second machine (amended 2026-10-31: the second machine is deferred, [ADR-025](../architecture/decisions/ADR-025-single-machine-evidence-accepted.md); Phase 4 re-tests on the reference machine only) |
| R-21 single-machine evidence | All numbers come from one machine; lower-end or different audio hardware is untested |
| R-01 frb toolchain | Works; costs are the `freezed` toolchain, pinned helper tools and stale template lockfiles |
| R-04 TAB editor effort | Unchanged and the largest effort item; S4 removed only the rendering-performance part |
| R-16 licenses | Rust trees clean; FluidR3 GM license to be read from the original source when bundled; Dart dependency audit in Phase 3 |
| Atomic save in the field | Real OneDrive and antivirus behaviour untested (ADR-008 reconsideration condition) |
| S1 timing exception | One 56 ms gap at 60 Hz in 10 min; monitored in interop benchmarks |
| Tuner (A8 to A10) | Untested until S3 in Phase 5 |

## 6. CI validation status

- The first GitHub Actions run, on commit `6dcb8bc` (branch `m0-foundation`), succeeded. It reported only maintenance warnings: Node 20 deprecation for `actions/checkout@v4` and `actions/setup-node@v4`, and the `ubuntu-latest` image migration notice.
- The commit that records this report also updates the workflow (`actions/checkout@v7`, `actions/setup-node@v7`, Linux runner pinned to `ubuntu-24.04`). This updated workflow has not yet been validated by GitHub Actions and will be validated on the next push.
- Phase 1 starts only after that run passes.

## 7. Phase 0 exit criteria

| Criterion | Result |
|---|---|
| Each spike has a recorded result against its success criteria | Met ([technical-spikes.md](../research/technical-spikes.md) Results) |
| CI fails on a forbidden dependency | Met locally (cargo-deny rejected the MPL-2.0 crate). Not deliberately exercised in GitHub Actions |
| CI fails on a protected-path change without a trailer | Met locally: xtask tests, and the real M0 change in a throwaway clone was rejected without the trailer and accepted with it. The trailer check passed in GitHub Actions on `6dcb8bc` |
| No result invalidates the architecture, or affected ADRs are revised | Met |

## 8. What M0 did not verify

Audible quality by listening; machines other than the reference machine; real OneDrive and antivirus behaviour; exclusive-mode WASAPI; panic unwinding across the cpal callback; the tuner; a Web build; Dart dependency licenses; recalibration of human FTE estimates (spikes stayed inside their timeboxes, which says nothing about human effort).

## 9. Phase 1 entry

- Scope stays as planned: `mc-theory`, `mc-score`, `mc-project`, `mc-app`, `mc-cli`, with no UI, audio, AI or cross-platform work pulled forward.
- Prerequisites met: ADR-008, ADR-011, ADR-016, ADR-019, ADR-020 Accepted; Q-07, Q-09, Q-17 resolved. Q-08 (file extension) is needed before Phase 1 exit.
- Before adding the first crate: add `architecture/allowed-deps.toml` and the FF-01 check, then activate the Phase 1 fitness functions and set `current_phase = 1` in the inventory.
- Carry-overs: `std::fs::rename` with retry, deflate-only `zip`, 100:1 ratio cap, U1 detection and U2 Save As protection, PB-01 to PB-08.
- Spike code is not copied into production; it is reimplemented against the architecture.

## 10. Required governance commit trailers

Commits that change protected paths must carry `Architecture-Change:` followed by existing ADR ids (FF-19).

- M0 foundation commit `6dcb8bc` (already pushed):

```text
Architecture-Change: ADR-005, ADR-007, ADR-008, ADR-015, ADR-018, ADR-020
```

- Commit recording this accepted baseline (changes ADR-007, ADR-015, the ADR index and the CI workflow):

```text
Architecture-Change: ADR-007, ADR-015
```
