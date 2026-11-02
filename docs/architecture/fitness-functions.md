# Architecture Fitness Functions

Purpose: Define the automated checks that keep the architecture rules true over time, with criticality, tool, activation phase and threshold justification.
Status: Accepted (catalogue). Each function is implemented in its activation phase; none exist yet.
Related documents: [dependency-rules.md](dependency-rules.md), [domain-invariants.md](domain-invariants.md), [project-format.md](project-format.md), [performance-requirements.md](performance-requirements.md), [decisions/README.md](decisions/README.md), `.kiro/steering/architecture-guardrails.md`.

Criticality: Critical blocks merge. Important needs review when it fails (does not block automatically). Advisory produces reports.

Planned implementation home: a Rust `xtask` crate (`cargo xtask fitness`) plus configuration files under `architecture/` at the repository root, run by CI on every push. Dart checks run as Dart tests in `app/test/architecture/`.

## Catalogue

| Id | Check | Rules | Tool | Criticality | Active from | Threshold and justification |
|---|---|---|---|---|---|---|
| FF-01 | Internal crate dependency matrix | GR-01, GR-03, GR-04, GR-05 | xtask reading `cargo metadata` and `architecture/allowed-deps.toml` | Critical | Phase 1 | Binary: any unlisted edge fails |
| FF-02 | External crate bans (frb only in mc-ffi, cpal and rustysynth only in mc-audio, rusqlite nowhere in MVP) | GR-01, GR-02, GR-03 | cargo-deny `bans` with `wrappers` | Critical | Phase 1 | Binary |
| FF-03 | Domain purity lints (no fs, net, thread, clock, env in domain crates) | GR-03, GR-11 | clippy `disallowed-methods` and `disallowed-types`, `-D warnings` on domain crates | Critical | Phase 1 | Binary |
| FF-04 | Domain crates compile for `wasm32-unknown-unknown` | GR-11, ADR-014 | `cargo check --target wasm32-unknown-unknown -p mc-theory -p mc-score ...` | Critical | Phase 1 | Binary; keeps Web option open cheaply |
| FF-05 | Dart import boundaries (features import only `core_bridge` facade, `ui_kit`, `app_platform`; no generated bindings; no feature-to-feature imports) | GR-06 | `app/test/architecture/dart_fitness_test.dart` (FF-05 test), run in CI `bridge` job `app analysis and tests` | Critical | Phase 3 | Binary |
| FF-06 | Dart package cycles | GR-07 | Same test file; checks direct pubspec edges in the local workspace | Critical | Phase 3 | Binary |
| FF-07 | `cfg(target_os)` and platform crates only in adapter crates | GR-11 | xtask source scan | Important | Phase 1 | Binary; Important because rare legitimate exceptions need review |
| FF-08 | `dart:io` and `Platform` only in the `app_platform` package and `app` (generated bindings in `core_bridge/lib/src/generated` excepted) | GR-11 | Same test file (FF-08 test) | Important | Phase 3 | Binary |
| FF-09 | License policy (Rust and Dart) | ADR-015 | cargo-deny `licenses` allow-list; Dart license check script over `pubspec.lock` [tool To verify] | Critical | Phase 0 | Allow-list from ADR-015 |
| FF-10 | Security advisories and sources | Security | cargo-deny `advisories`, `sources` | Important | Phase 0 | Any advisory requires triage within the PR |
| FF-11 | Project format round-trip suite ([project-format.md](project-format.md) section 14). Item 4 covers U1 detection (4a) and U2 Save As protection (4b) from Phase 1; 4c (U3 full preservation) is added with format 1.1 | GR-08, P-01 to P-06 | Rust tests with fixture corpus. Fixtures are embedded with `include_bytes!` (no file I/O in domain test code, FF-03), created by `mc-cli format-fixtures` without overwriting, and FF-20 fails if a fixture file is not referenced by the test | Critical | Phase 1 | Binary; fixture corpus is append-only (protected path) |
| FF-12 | Generated bindings in sync | ADR-005, ADR-022 | CI job `bridge`: pinned Flutter, `flutter_rust_bridge_codegen` 2.13.0 and `cargo-expand` 1.0.127; runs `flutter_rust_bridge_codegen generate --no-dart-format` in `packages/core_bridge`, then `git diff --exit-code` on `crates/mc-ffi/src` and `packages/core_bridge/lib/src/generated`. `--no-dart-format` is used because `dart format` versions differ between local and CI, which would make the diff non-deterministic. The generated Rust module is excluded from rustfmt (`#[rustfmt::skip]`) because the generator owns its formatting | Critical | Phase 3 | Binary |
| FF-13 | Hard invariants hold under random command sequences | H rules | proptest | Critical | Phase 1 | Fixed case count (for example 256 per run in CI, more nightly); seed failures saved as regressions |
| FF-14 | Undo and redo restore identical state | ADR-016 | proptest | Critical | Phase 1 | Same as FF-13 |
| FF-15 | Importer and loader fuzzing | GR-09 | cargo-fuzz (Linux CI, scheduled) | Important | Phase 4 | Any crash creates a regression fixture that then becomes part of FF-11 (Critical) |
| FF-16 | No allocation in RT callback bodies | GR-10 | assert_no_alloc in test harness | Critical | Phase 4 | Binary |
| FF-17 | RT module forbidden-construct lint | GR-10 | xtask scan of `mc-audio/src/rt/` | Critical | Phase 4 | Binary; list in [audio-dsp-architecture.md](audio-dsp-architecture.md) |
| FF-18 | Playback glitch test (10 min, UI load simulated). Gaps are detected from callback intervals compared with the reported output latency, because cpal reports no xruns on WASAPI shared render (S2) | GR-10, A4 | Scheduled job on a real Windows machine (not hosted CI; hosted runners lack audio devices [Assumption]) | Important | Phase 4 | Zero callback intervals above the reported output latency; intervals above 1.5x the period are reported. Thresholds from S2 measurements |
| FF-19 | Protected paths require `Architecture-Change: ADR-NNN` trailer referencing an existing ADR file | GR-13, GR-14 | xtask or CI script on the commit range | Critical | Phase 0 | Binary |
| FF-20 | Fitness inventory cannot shrink | GR-14 | `architecture/fitness-inventory.toml` lists check ids; xtask verifies each exists and runs; removals require FF-19 trailer | Critical | Phase 0 | Binary |
| FF-21 | Benchmark regression | Performance | criterion with stored baselines | Important | Phase 1 | More than 10 percent regression in 3 consecutive runs on the reference machine; see [performance-requirements.md](performance-requirements.md) |
| FF-22 | Public API diff of crates | GR-06 | cargo-public-api 0.52.0 with nightly rustdoc JSON [Verified: Phase 1], weekly against `architecture/public-api/` snapshots | Important | Phase 1 | Any diff shown for review; Critical with cargo-semver-checks once SDK is published (Phase 11) |
| FF-23 | Memory baselines (large score, undo step) | Performance | Test with counting allocator. Implemented as a deterministic structural measurement (`Score::heap_bytes`, history byte accounting: element sizes times counts) against `benchmarks/baselines/memory.toml`. Limitation: this is not process-level or peak memory; it excludes strings, allocator overhead and transient buffers (for example during file open). A counting allocator would need `unsafe`, and the workspace-wide `unsafe_code = "forbid"` stays (user decision 2026-10-31; the only exception is the generated FFI module, [ADR-022](decisions/ADR-022-unsafe-confined-to-generated-ffi.md), which does not extend to tests). Revisit a test-only exception only if peak memory becomes necessary for a concrete product or architecture decision and no safe measurement is adequate | Important | Phase 1 | Regression over 10 percent versus baseline |
| FF-24 | Invariant rule ids without tests | Domain invariants | xtask matching rule ids in [domain-invariants.md](domain-invariants.md) to test names | Advisory | Phase 1 | Report only |
| FF-25 | Coupling report (fan-in and fan-out per crate and module) | Modularity | `cargo xtask fitness coupling`: crate edges from the workspace manifests, module edges from `crate::` paths (re-exports attributed to their module; `super::` and macro paths not followed) | Advisory | Phase 2 | Report trends; no threshold until data exists |
| FF-26 | Intra-crate module cycles | GR-07 | `cargo xtask fitness module-cycles`: strongly connected components of the FF-25 module graph (no external tool) | Advisory | Phase 2 | Report |
| FF-27 | Test coverage | Testability | cargo-llvm-cov, flutter test coverage | Advisory | Phase 1 | Report; no gate (coverage gates invite low-value tests) |
| FF-28 | Complexity lints | Maintainability | clippy `cognitive_complexity`, `too_many_lines` warnings | Advisory | Phase 1 | clippy defaults |
| FF-29 | Binary and bundle size | Distribution | CI artifact size report; the Windows build produces a measurable artifact from Phase 4 | Advisory | Phase 4 | Report trends |
| FF-30 | Dependency freshness | Maintenance | cargo-outdated, `flutter pub outdated` | Advisory | Phase 1 | Monthly report |
| FF-31 | Candidate acceptance passes domain validation | GR-04, AI-02 | Integration test with invalid candidate | Critical | Phase 8 | Binary |
| FF-32 | DTO JSON schema snapshot requires version bump | GR-08 | schemars snapshot test | Critical | Phase 1 | Binary |
| FF-33 | Documentation links and Mermaid diagrams valid | GR-12 | Link check script, mermaid-cli | Important | Phase 0 | Binary |
| FF-34 | Unsafe code confined to the generated FFI module ([ADR-022](decisions/ADR-022-unsafe-confined-to-generated-ffi.md)): every crate except `mc-ffi` uses the workspace lints (`unsafe_code = "forbid"`); the `mc-ffi` lint table equals the workspace table except `unsafe_code = "deny"`; the only `allow(unsafe_code)` is on `mod frb_generated;` in `crates/mc-ffi/src/lib.rs` | Security, ADR-005 | `cargo xtask fitness unsafe-scope` (source and manifest scan) | Critical | Phase 3 | Binary |

## Activation by phase

Fitness functions are implemented when the code they guard first exists. M0 implements only the five Phase 0 checks. The inventory (`architecture/fitness-inventory.toml`) lists all 34 with status `active` or `planned`; FF-20 verifies that every `active` entry is implemented and that no entry is removed or downgraded without an ADR trailer.

| Phase | Checks activated | Count |
|---|---|---|
| 0 | FF-09 (Rust part; Dart part in Phase 3), FF-10, FF-19, FF-20, FF-33 | 5 |
| 1 | FF-01, FF-02, FF-03, FF-04, FF-07, FF-11, FF-13, FF-14, FF-21, FF-22, FF-23, FF-24, FF-27, FF-28, FF-30, FF-32 | 16 |
| 2 | FF-25, FF-26 | 2 |
| 3 | FF-05, FF-06, FF-08, FF-12, FF-34 | 5 |
| 4 | FF-15, FF-16, FF-17, FF-18, FF-29 | 5 |
| 8 | FF-31 | 1 |
| Total | | 34 |

## Notes on thresholds

- Binary checks need no threshold. Numeric thresholds exist only for performance and memory, and only after baselines exist. Until then those checks report values without failing.
- Hosted CI performance numbers are noisy; FF-21 and FF-23 compare on the reference machine or with instruction counts (for example iai-callgrind on Linux [To verify]) rather than wall time on shared runners.

## CI cost for a solo developer

- Every push (Windows runner): FF-01 to FF-04, FF-07, FF-09, FF-11 to FF-14, FF-16, FF-17, FF-19, FF-20, FF-32, unit tests. Target under 10 minutes [Assumption].
- Nightly or weekly: FF-15, FF-21 to FF-30, Linux and macOS builds (macOS runners cost more; weekly is enough until the macOS port).
- Manual before release: FF-18 on the reference machine.
