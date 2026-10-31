# Technology Evaluation

Purpose: Evaluate candidate libraries and tools for capability, platforms, maintenance, license, integration cost, limits and exit strategy, before any dependency is adopted.
Status: Proposed. Each item becomes Accepted only after the Phase 0 license audit (S7) and its owning spike.
Related documents: [feasibility-assessment.md](feasibility-assessment.md), [technical-spikes.md](technical-spikes.md), [ADR-015 license policy](../architecture/decisions/ADR-015-dependency-license-policy.md), [alternative-architectures.md](alternative-architectures.md).

## 1. Verification method

- `[Verified: crates.io API, 2026-10]` means the crates.io API record was read during planning (license field, latest version, release date).
- `[Verified: README]` means the project README was read during planning.
- `[To verify]` means the statement comes from prior knowledge and must be checked in S7 before adoption.
- Package registry license fields are metadata, not legal text. S7 must read the actual LICENSE files and check transitive dependencies with `cargo-deny`.

## 2. License usage categories

The project license is undecided. To stay compatible with both permissive open source and commercial distribution, we distinguish four kinds of use. They carry very different obligations.

| Category | Example | Obligation | Policy |
|---|---|---|---|
| (a) Learning from a project | Reading TuxGuitar UI behaviour or alphaTab layout ideas | None for ideas. Do not copy expression (code, comments, assets). | Allowed. Record which projects were studied. |
| (b) Implementing a file format | Writing our own Guitar Pro or MusicXML reader | Format specs are generally not copyrightable as code, but spec documents can carry their own license terms and some formats are proprietary. | Allowed after S10 (Guitar Pro) and a spec-license check (MusicXML). Clean-room implementation, no code copied. |
| (c) Linking a library | Depending on a crate, statically linked | License of the library applies to the distributed binary. Copyleft (GPL) affects the whole binary. LGPL static linking on iOS is problematic. MPL is file-level copyleft. | Permissive only in the core. Copyleft needs an ADR and an isolation strategy. |
| (d) Copying source | Porting a function from TuxGuitar | Full license applies to the copied code. | Prohibited from copyleft sources. Permissive sources only, with attribution in a NOTICE file. |

Allowed by default (via cargo-deny allow-list): MIT, Apache-2.0, BSD-1/2/3-Clause, ISC, Zlib, Unlicense, Unicode-3.0, CC0-1.0. Anything else blocks CI (FF-09).

## 3. Interop and application frameworks

### flutter_rust_bridge (frb) v2
- Why: Generates typed Dart bindings for Rust, including opaque handles, async functions, streams and Rust-to-Dart calls. Avoids hand-written FFI and a second schema.
- Alternatives: hand-written `dart:ffi` + C ABI; `uniffi` (no official Dart backend [To verify]); Protobuf over raw FFI; platform channels.
- Platforms: README claims Android, iOS, Windows, macOS, Linux, Web [Verified: README]. Web build requirements page not retrieved [To verify] (likely needs cross-origin isolation for threads).
- Maintenance: stable 2.13.0, 2.14.0-beta.2 released 2026-09 [Verified: crates.io API, 2026-10]. Effectively one main maintainer [Assumption]: bus-factor risk.
- License: MIT [Verified: crates.io API, 2026-10].
- Performance: SSE codec described as several times faster on some workloads [Verified: README]. Our own measurement in S1.
- Integration complexity: Medium. Codegen step, build integration via cargokit [To verify], hot reload does not reload Rust.
- Limits: Default features pull `tokio` and a thread pool [Verified: crates.io feature list]. Generated code must be kept in sync (FF-12).
- Exit strategy: All frb types live in `mc-ffi` only (GR-01). Replacing frb means rewriting `mc-ffi` and the Dart `core_bridge` package, not the core.

### Flutter (Dart)
- Why: One UI toolkit for Windows, macOS, Linux, Android, iOS and Web, with a custom painting API suitable for TAB canvases.
- Alternatives: see [alternative-architectures.md](alternative-architectures.md).
- License: BSD-3-Clause [To verify].
- Limits: Desktop docking and multi-window support are less mature than mobile [Assumption]. Text measurement for notation is done in Dart (see [ADR-018](../architecture/decisions/ADR-018-notation-layout-in-rust.md)).
- Exit strategy: UI holds no domain model (ADR-002), so a UI rewrite does not touch the core.

### State management: Riverpod vs flutter_bloc
- Both MIT [To verify]. Decision deferred to Phase 3 start. Criteria: testability with the fake `DocumentService`, boilerplate per feature, stream integration. Either is acceptable because document state lives in Rust.

## 4. Audio

### cpal
- Why: Cross-platform audio I/O in Rust with a callback model.
- Platforms: WASAPI (Windows default, optional ASIO and JACK, minimum Windows 10), CoreAudio (macOS minimum 14.2 per README, iOS), ALSA plus optional PipeWire/PulseAudio/JACK on Linux (needs ALSA dev libs), AAudio (Android 8.0, API 26), Web Audio API, AudioWorklet (nightly, atomics, SharedArrayBuffer and cross-origin headers). MSRV 1.85. Optional `realtime` feature raises callback thread priority [Verified: README].
- License: Apache-2.0 [To verify].
- Limits: Whether WASAPI exclusive mode is exposed and whether `BufferSize::Fixed` is honored on WASAPI is not documented in what was read [To verify, S2]. The macOS 14.2 minimum may matter for distribution [To verify].
- Alternatives: `miniaudio` via bindings (C, public domain or MIT-0 [To verify]); `rodio` (higher level, built on cpal); PortAudio (C, MIT-like); JUCE (GPL or commercial).
- Exit strategy: cpal is used only in `mc-audio` behind the `AudioPort` port.

### rustysynth
- Why: Pure-Rust SoundFont synth, no dependencies beyond std, real-time and offline rendering, MIDI file sequencer, dynamic tempo [Verified: README].
- License: MIT [Verified: README].
- Maintenance: [To verify] last release date.
- Limits: Allocation and locking behaviour in `render`, `note_on`, `note_off` is not documented [To verify, S2, assumption A1]. Reverb and chorus supported [Verified: README].
- Alternatives: FluidSynth (LGPL-2.1, C, linking concerns), `oxisynth` (port of FluidSynth, LGPL [To verify]), own simple sampler.
- Exit strategy: Wrapped behind a `Synth` trait inside `mc-audio`. If A1 fails, options are a patched fork, preallocated voices, or a different synth.

### SoundFonts
- TimGM6mb: GPL-2.0 [To verify]. Must not be the bundled default (bundling GPL data with a commercial app is legally unclear).
- FluidR3_GM: MIT [To verify]. About 140 MB [Assumption]; too large for a default mobile bundle.
- GeneralUser GS: custom permissive license [To verify].
- Decision: S7 selects the default. Optional user-supplied SoundFonts are always allowed.

### rtrb
- Why: Real-time-safe wait-free SPSC ring buffer for control-plane messages and sample transfer.
- License: MIT OR Apache-2.0. Latest 0.4.1, released 2026-10 [Verified: crates.io API, 2026-10]. Actively maintained.
- Limits: SPSC only. Wait-free claim from description [Verified: crates.io API]; allocation-free push and pop to be confirmed with assert_no_alloc (A3).
- Alternatives: `ringbuf` (MIT/Apache [To verify]), `crossbeam-queue::ArrayQueue` (MPMC, lock-free).
- Exit strategy: Small API surface, trivially replaceable.

### assert_no_alloc
- Why: Global allocator wrapper that aborts or warns on allocation in a marked scope. Used in tests and debug builds of the audio callback.
- License: BSD-1-Clause. Latest 1.1.2, released 2021-08 [Verified: crates.io API, 2026-10]. Default feature `disable_release` disables checks in release builds.
- Limits: Detects allocation only, not locks or syscalls. Unmaintained since 2021, but it is about 130 lines and easily vendored [Verified: crates.io linecount].
- Exit strategy: Vendor or replace with a 50-line custom allocator in the test harness.

### rubato
- Why: Sample-rate conversion if device rates differ from analysis rates. Not needed in MVP (render at device rate).
- License: MIT [To verify].

### rustfft / realfft
- Why: FFT for spectrogram and MPM autocorrelation.
- License: MIT OR Apache-2.0 [To verify]. Pure Rust, WASM compatible [Assumption].

### pitch-detection
- License MIT/Apache-2.0, latest 0.3.0 released 2022-06 [Verified: crates.io API, 2026-10].
- Decision: Use as reference only. Implement a small owned YIN/MPM in `mc-dsp` (a few hundred lines), because the crate is unmaintained and we need allocation-free processing with preallocated buffers.

## 5. MIDI and formats

### midly
- Why: Standard MIDI file decode and encode, live events, no_std capable.
- License: Unlicense. Latest 0.5.3 released 2023-01 [Verified: crates.io API, 2026-10].
- Limits: No release since 2023. The format is stable, so low release activity is acceptable [Assumption].
- Alternatives: `midi-msg`, own writer (SMF writing is small).
- Exit strategy: Used only in the MIDI adapter module.

### MusicXML
- Spec under the W3C Community Final Specification Agreement [To verify]. Implemented in our own adapter in Phase 6b. Candidate crates are not yet evaluated [To verify].

### Guitar Pro (GP3 to GP5 binary, GPX, GP7 and later zipped XML)
- Proprietary, partially documented by reverse-engineering projects. TuxGuitar (LGPL-2.1 [To verify]) and alphaTab (MPL-2.0 [To verify]) contain readers. Category (a) and (b) use only: learn and implement clean-room. S10 evaluates legal and format risks.

### PDF
- Deferred to Phase 6c with engraving (S11). Candidate: own layout output to a permissive PDF writer (`printpdf` MIT [To verify]) or Flutter-side `pdf` package (Apache-2.0 [To verify]).

## 6. Persistence

### zip (zip-rs/zip2)
- License: MIT. Latest 9.1.3 released 2026-10, MSRV 1.88, repository moved to `zip-rs/zip2` [Verified: crates.io API, 2026-10]. Actively maintained under the new organization.
- Limits: Default features enable AES, bzip2, deflate64, lzma, ppmd, zstd, xz [Verified: crates.io feature list]. Use `default-features = false` with deflate only, to reduce attack surface and native dependencies.
- MSRV 1.88 is higher than cpal's 1.85. The workspace MSRV is the max of all dependencies.
- Exit strategy: Used only inside `mc-project` package I/O.

### serde, serde_json, schemars
- MIT OR Apache-2.0 [To verify]. Unknown-field behaviour and `flatten` caveats are documented in [project-format.md](../architecture/project-format.md).

### tempfile
- MIT OR Apache-2.0 [To verify]. `NamedTempFile::persist` is a candidate for atomic replace. Windows semantics checked in S5.

### rusqlite (deferred)
- MIT [To verify]. Bundled SQLite is public domain. Introduced only with the library index ([ADR-009](../architecture/decisions/ADR-009-defer-sqlite.md)).

## 7. Testing and governance tools

| Tool | Purpose | License | Notes |
|---|---|---|---|
| proptest | Property tests for invariants and undo | MIT OR Apache-2.0 [To verify] | Stable |
| criterion | Benchmarks | MIT OR Apache-2.0 [To verify] | Relative regression comparisons |
| cargo-deny | License, bans, advisories, sources | MIT OR Apache-2.0 [To verify] | Core of GR-01, GR-02, ADR-015 |
| cargo-fuzz | Fuzzing importers | MIT OR Apache-2.0 [To verify] | libFuzzer on Windows is limited [To verify]; run on Linux CI |
| cargo-public-api | Public API diff | MIT [To verify] | Needs nightly rustdoc JSON [To verify] |
| cargo-semver-checks | Semver checking when the SDK is published | MIT OR Apache-2.0 [To verify] | Phase 11 |
| @mermaid-js/mermaid-cli | Diagram validation | MIT [To verify] | Docs CI |

## 8. ML and MIR (Phase 9 and later)

| Technology | Why | License | Platforms | Limits | Spike |
|---|---|---|---|---|---|
| ort (ONNX Runtime bindings) | Run ONNX models with CPU, DirectML, CoreML, CUDA providers | Bindings MIT OR Apache-2.0, ONNX Runtime MIT [To verify] | Desktop and mobile, native binary download | Native library size, versioning | S8 |
| tract | Pure-Rust ONNX and NNEF inference | MIT OR Apache-2.0 [To verify] | All including WASM [Assumption] | Operator coverage gaps, CPU only | S8 |
| Basic Pitch (Spotify) | Polyphonic note transcription, lightweight | Apache-2.0 [To verify] | Model export to ONNX [To verify] | Not guitar-string aware | S8 |
| Demucs | Source separation | Code MIT [To verify], weights license separate [To verify] | Desktop, GPU preferred | Large models, slow on CPU | S9 |
| Spleeter | Source separation (older) | MIT [To verify] | TensorFlow based | Lower quality than Demucs [Assumption] | S9 |
| CREPE / pYIN | Monophonic pitch for vocals | CREPE MIT [To verify]; pYIN is an algorithm | pYIN deterministic | CREPE is ML | Phase 8 |

## 9. Notation and reference projects

| Project | Use category | License | Notes |
|---|---|---|---|
| TuxGuitar | (a) learn UI and features; (b) format knowledge | LGPL-2.1 [To verify] | No code copied |
| alphaTab | (a) learn TAB and score layout; (b) GP format knowledge | MPL-2.0 [To verify] | MPL is file-level copyleft. Linking a JS library is not relevant to a Rust core |
| Verovio | Candidate engraving engine for standard notation (S11) | LGPL-3.0 [To verify] | Linking LGPL statically on iOS is problematic. Would need dynamic linking or process isolation on desktop; evaluation in S11 |
| MuseScore | (a) learn | GPL-3.0 [To verify] | No linking, no copying |

## 10. Summary of adoption intent

| Area | MVP choice | Status |
|---|---|---|
| Interop | flutter_rust_bridge v2 | Proposed, pending S1 |
| Audio I/O | cpal | Proposed, pending S2 |
| Synth | rustysynth + permissive SoundFont | Proposed, pending S2 and S7 |
| RT queue | rtrb | Proposed, pending S2 |
| Pitch | Own YIN/MPM in `mc-dsp` | Proposed, pending S3 |
| MIDI | midly | Proposed |
| Package | zip (deflate only) + serde_json | Proposed, pending S5 |
| Tests | proptest, criterion, cargo-deny, assert_no_alloc | Proposed |
