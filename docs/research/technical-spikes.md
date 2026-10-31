# Technical Spikes

Purpose: Define the proof-of-concept experiments (S1 to S12) that answer questions documentation cannot, with question, timebox, success criteria and the phase gate each one unblocks.
Status: S1, S2, S4, S5 and S7 executed in M0 (results below). S3, S6, S8 to S12 not started.
Related documents: [implementation-phases.md](../roadmap/implementation-phases.md), [dependency-map.md](../roadmap/dependency-map.md), [technology-evaluation.md](technology-evaluation.md), [audio-dsp-architecture.md](../architecture/audio-dsp-architecture.md), [risk-register.md](../roadmap/risk-register.md).

Rules:

- Spike code is throwaway, kept in a `spikes/` folder or a branch, never merged into production crates.
- Timeboxes are maxima. When a timebox expires, the spike stops and records what is known; the decision is then made with that information or the risk is escalated in the [risk register](../roadmap/risk-register.md).
- Each spike ends with a short result note (pass, fail, findings, decision impact) in the Results section below and in the related ADR.
- A failed criterion is recorded as Fail with its measured value. The spike is not extended to make it pass. The affected ADR is revised, or an alternative is recorded with its own follow-up, and the risk register is updated.
- Criteria may only be changed before a spike runs. The S1 and S4 criteria were made measurable on 2026-10-31, before any spike code existed (warm-up excluded, jank defined).
- Measurements on the developer machine are evidence for that machine only. They are recorded with the machine description in the results.

| Id | Question | Timebox (FTE days, max) | Success criteria | Gate |
|---|---|---|---|---|
| S1 | Does frb v2 on Windows support our interop shape: opaque document handle, async execute, event `StreamSink`, 60 Hz position stream, mirroring of `mc-app` types without frb annotations inside `mc-app`, acceptable build and reload workflow? | 2 | All features work in a minimal Flutter Windows app (profile build); no-op round trip p99 under 2 ms; 60 Hz stream for 10 min: average delivered rate at least 59 Hz, no delivery gap over 50 ms, UI frame p99 at most 16 ms after a 2 s warm-up; mirroring works or a DTO fallback is documented | Phase 0 exit; ADR-005 |
| S2 | Are cpal (WASAPI), rustysynth and rtrb real-time safe and fast enough together? Assumptions A1 to A7 | 3 | Criteria A1 to A7 in [audio-dsp-architecture.md](../architecture/audio-dsp-architecture.md) | Phase 0 exit; ADR-007 |
| S3 | Can owned YIN or MPM give a usable tuner from the Windows microphone? Assumptions A8 to A10 | 2 | Criteria A8 to A10 | Phase 5 entry (budgeted inside Phase 5) |
| S4 | Can Rust layout + Flutter `CustomPainter` render and scroll a 500-bar TAB track smoothly? | 2 | Profile build, continuous scroll through all 500 bars, 2 s warm-up excluded: frame p99 (max of build and raster) at most 16 ms and no frame over 33 ms; Rust layout per system p99 under 2 ms; layout payload per system measured; metrics contract workable | Phase 0 exit; ADR-018 |
| S5 | ZIP + JSON format: size and load time for small, typical, large scores; Windows atomic replace semantics including OneDrive and antivirus locks; `zip` crate with deflate only; `flatten` performance | 2 | Typical open under 500 ms, large under 2 s; replace strategy chosen with failure behaviour under a simulated lock documented; U1 detection prototype reports unknown fields at every nesting level with correct paths; flatten parse overhead measured (no U3 work) | Phase 0 exit; ADR-008 |
| S6 | What blocks a Web build? frb Web smoke build with a domain crate | 1 (optional, outside Phase 0 estimate) | Blocker list documented | Informs ADR-014 |
| S7 | License audit of Phase 1 to 5 dependencies (transitive) and SoundFont selection | 1 | cargo-deny config passes on a dependency list; SoundFont with permissive license selected; NOTICE requirements listed | Phase 0 exit (before Phase 1 dependencies are added) |
| S8 | ONNX runtime choice: ort vs tract running Basic Pitch on Windows CPU | 3 | Model runs; latency and memory measured; packaging size known; licenses confirmed | Phase 9 entry |
| S9 | Source separation (Demucs class) on desktop CPU and GPU; weights license | 4 | Time per minute of audio, memory, quality on a test set; license decision | Phase 9 |
| S10 | Guitar Pro import: GP3 to GP5 and GP7 structure, legal and format risks, effort estimate | 3 | Parse fixture files of each version into a test model; risk note | Phase 6d entry |
| S11 | Standard notation engraving: own engine vs Verovio (LGPL-3.0 [To verify]) vs alternatives | 4 | Quality comparison on sample scores; licensing and linking plan; effort estimate | Phase 6c entry |
| S12 | Android and iOS: frb packaging, cpal AAudio and CoreAudio latency, AVAudioSession handling | 4 | Tuner and playback run on one Android and one iOS device; latency measured | Phase 7 entry |

## Phase 0 budget reconciliation

- Mandatory Phase 0 spikes: S1, S2, S4, S5, S7 = at most 10 FTE days.
- Foundation work (repository, CI skeleton, cargo-deny, protected-path check, steering, reference machine record): 3 to 5 FTE days.
- Phase 0 total: at most 15 FTE days (3 FTE weeks) when every timebox is used. The lower bound of 2 FTE weeks assumes the spikes finish in about 5 to 7 days. This matches the 2 to 3 FTE week estimate in [implementation-phases.md](../roadmap/implementation-phases.md).
- S3 is budgeted in Phase 5; S6 is optional and outside the estimate.

Priority order if time is short: S1, S2, S4, S5 (highest architectural risk), then S7.

## Results (M0, 2026-10-31)

All measurements are from the reference machine in [benchmarks/reference-machine.md](../../benchmarks/reference-machine.md). Raw output is in each spike's `results/` folder. The summary and recommendation are in [m0-exit-report.md](../roadmap/m0-exit-report.md). Elapsed effort stayed inside every timebox.

### S1 frb interop (Pass, one sub-criterion failed)

| Criterion | Measured | Result |
|---|---|---|
| Opaque handle, async execute, event stream, fixed-rate stream | All work in a Windows profile build | Pass |
| Mirroring `mc-app`-like types from a crate with no frb dependency | Works for a struct and a payload-carrying enum (`#[frb(mirror)]`) | Pass |
| No-op round trip p99 under 2 ms | Thread-pool function 160 µs; async Rust function 208 µs; sync query 3 µs (10 000 calls each) | Pass |
| Events | 20 502 of 20 502 delivered, gapless and in order | Pass |
| 10 min stream: average rate at least 59 Hz | 59.99999 Hz, 36 000 of 36 000 delivered | Pass |
| 10 min stream: no gap over 50 ms | 1 gap of 56 ms in 36 000 | Fail |
| UI frame p99 at most 16 ms after warm-up | 2.3 ms (43 209 frames; max 22 ms, 1 over 16 ms) | Pass |

Findings that affect the design:

- Payload enums map to Dart `freezed` sealed classes, so the Dart side needs `freezed`, `freezed_annotation` and `build_runner`.
- Rust `u64` maps to Dart `BigInt`. Revision, seq and ids crossing FFI should use types that map to Dart `int`.
- Cancelling a Dart subscription does not complete while Rust still holds the `StreamSink`, and later events are silently dropped. Stream lifecycle must be owned by Rust: an explicit unsubscribe call and closing on handle drop. Dart must not await `cancel()` as a release mechanism.
- frb codegen auto-installed `cargo-expand` (unpinned). CI must preinstall pinned versions of both tools.
- frb's project template ships a stale `Cargo.lock` (yanked `futures-util`, unmaintained `adler`). Run `cargo update` and the S7 policy after scaffolding.

### S2 real-time playback (Pass with conditions; A4 partially met)

| Assumption | Measured | Result |
|---|---|---|
| A1 no allocation in synth render, note on and off | 0 allocations: 300 s offline with a generated font and with a 261-preset GM bank (GeneralUser GS, drums included); 600 s and 120 s on the device. Negative control detected | Pass |
| A2 callback p99 under 50 percent of period | 10 ms WASAPI period: p99 209 µs (2.1 percent) with the sine font, 320 µs (3.2 percent) with the GM bank; max under 2 ms with priority promotion | Pass |
| A3 rtrb push and pop do not allocate | Covered by the A1 runs (commands, swaps, metrics through rtrb) | Pass |
| A4 no underruns in 10 min under load | Default priority: 16 late callbacks in 60 s, max 37 ms (buffer about 33 ms). With priority promotion: 600 s run 0 late, max 11.4 ms; 4 of 5 later 120 s runs 0 late; 1 run (cpal `realtime`) had 6 intervals over 2x period, max 44 ms. cpal reported 0 xruns in every run, including runs with 37 to 44 ms gaps | Partially met |
| A5 latency | WASAPI shared period fixed at 480 frames (10 ms); `BufferSize::Fixed(256)` and `Fixed(128)` are accepted but not honored. Reported output latency 33 to 53 ms. Estimated play to sound 28 to 55 ms | Pass (under 100 ms) |
| A6 timeline swap | 99 swaps on the device and 100 offline: no allocation in callback, all old timelines freed off-thread, no cursor regressions, no swap-related late callbacks | Pass |
| A7 device rate | A4 renders 440.0000 Hz (0.000 cents) at 44.1 and 48 kHz | Pass |

Findings that affect the design:

- Thread priority promotion is required. Two equivalent mechanisms were measured: cpal's `realtime` feature, and an own call to `AvSetMmThreadCharacteristicsW("Audio")` through `windows-sys`. The `realtime` feature depends on `audio_thread_priority` 0.35.1 (MPL-2.0), which ADR-015 does not allow without a decision. The own call is about 20 lines and uses MIT OR Apache-2.0 code. Both ran 0 late in their best runs; the difference between runs is OS noise.
- Even with promotion, rare system-level stalls of up to 44 ms occur on this machine. They are outside the engine (callback work stayed under 2 ms). Residual risk is tracked as R-02.
- Underruns cannot be detected through cpal error callbacks on WASAPI shared render. FF-18 must measure callback intervals against the reported output latency.
- After loading a large SoundFont, the first seconds showed a few late callbacks up to 17 ms. A warm-up render after loading is recommended.
- Swaps currently release sounding notes (`note_off_all(false)`). Carrying held notes across a swap is a Phase 4 design detail.
- Audible quality was not evaluated by listening. Gap detection is inferred from timing.

### S4 TAB rendering (Pass)

| Criterion | Measured | Result |
|---|---|---|
| Frame p99 at most 16 ms, no frame over 33 ms (500 bars, 9 615 notes, 125 systems, 2 s warm-up excluded) | Pass 1 (layout on first view): p99 11.3 ms, max 17.4 ms, 1 of 1 467 over 16 ms, 0 over 33 ms. Pass 2 (no layout cache): p99 9.8 ms, max 12.1 ms | Pass |
| Rust layout per system p99 under 2 ms | 5 µs in Rust; 46 to 56 µs including FFI conversion | Pass |
| Payload per system | 1.5 KB median, 2.1 KB max (`Float32List` and `Uint8List`) | Measured |
| Metrics contract | Digit width measured once in Dart and passed to Rust; worked without adjustment | Pass |

Frame cost is max(build, raster) from `FrameTiming`. The default runner window was used; the window size was not recorded. Painting is per glyph with cached `TextPainter`s.

### S5 project format (Pass)

| Criterion | Measured | Result |
|---|---|---|
| Typical open under 500 ms | 4 tracks x 150 bars, 9 600 notes: 64 KB package (924 KB JSON); open 7 to 8 ms; save 5 ms | Pass |
| Large open under 2 s | 20 x 300 bars, 288 000 notes: 1.37 MB package (23.9 MB JSON); open 178 to 249 ms across runs; save 115 to 123 ms | Pass |
| U1 detection at every level | 55 of 55 injected unknown fields and 1 unknown effect variant detected with exact paths across all 8 DTO levels; plain DTOs drop the same content silently | Pass |
| flatten overhead | Parse 1.7 to 2.4x slower than without capture (large: 81 ms to 141 to 193 ms) | Measured, acceptable |
| Atomic replace with simulated lock | See below | Pass |
| ZIP safety | Traversal, absolute, drive, backslash, dot segment, unknown top-level, case-duplicate entries, truncated ZIP and hash mismatch all rejected with typed errors; 1 GiB zero bomb rejected after 104 MB in 61 ms with a 100:1 ratio cap | Pass |

Windows replace behaviour (target held open by another handle):

| Method | Unlocked | Reader with full sharing | Handle without FILE_SHARE_DELETE | Lock released after 400 ms (retry) | Creation time |
|---|---|---|---|---|---|
| `std::fs::rename` | Ok | Ok | Access denied (5) after 5 attempts, original intact | Ok on attempt 4 | Not preserved |
| `tempfile::persist` | Ok | Access denied (5) | Access denied (5) | Ok on attempt 4 | Not preserved |
| `ReplaceFileW` | Ok | Ok | Sharing violation (32) | Ok on attempt 4 | Preserved |

No temp files were left in any case. Real OneDrive sync, real antivirus holds, network shares and FAT volumes were not tested. Synthetic fixtures compress better than real scores, so package sizes are lower bounds.

### S7 license audit (Pass with one decision required)

- cargo-deny 0.20.2 with the ADR-015 allow-list over: root workspace, S2 (cpal, rustysynth, rtrb, windows-sys), S5 (zip deflate-only, serde, serde_json, sha2, tempfile, windows-sys), S1 (flutter_rust_bridge 2.13.0 and its tree).
- All pass advisories, bans, licenses and sources, except the cpal `realtime` feature (MPL-2.0 via `audio_thread_priority`).
- License expressions seen: MIT (170 crates), Apache-2.0 (143), Zlib (9), Unlicense (6), BSD-3-Clause (2), BSD-2-Clause, Unicode-3.0, and as `OR` alternatives only: 0BSD (adler2), Apache-2.0 WITH LLVM-exception (rustix, Linux only), LGPL-2.1-or-later (r-efi, resolved to MIT). No copyleft license is selected.
- `deny.toml` gained `allow-wildcard-paths = true` because internal path dependencies otherwise fail the wildcard ban.
- NOTICE requirement: Apache-2.0 and BSD dependencies require attribution in distributed binaries. A generated third-party notices file is a Phase 3 packaging task (for example `cargo about`, license to verify).
- SoundFont: FluidR3 GM is MIT according to the author's release statement as quoted on musescore.org and in redistributions [Verified via secondary sources, 2026-10; read the original COPYING file before bundling]. GeneralUser GS License v2.0 allows commercial use and software bundling, but its author states that some sample origins are uncertain [Verified: official LICENSE.txt, 2026-10]. TimGM6mb remains excluded (GPL). rustysynth documents SF2 only, so compressed SF3 banks need conversion. Recommendation for Q-11: FluidR3 GM (or a smaller MIT-licensed derivative evaluated in Phase 4), with user-supplied SoundFonts supported. User approval needed.
- Dart package licenses were not audited (no Dart dependencies outside the throwaway spike). The Dart half of FF-09 activates in Phase 3.
