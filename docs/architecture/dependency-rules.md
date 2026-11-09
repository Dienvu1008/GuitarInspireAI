# Dependency and Governance Rules

Purpose: Enumerate the architecture governance rules GR-01 to GR-14 and map each to an enforcement mechanism, criticality and activation phase.
Status: Accepted. Mechanisms become active in the phase listed. Until then, the rule is enforced by review.
Related documents: [fitness-functions.md](fitness-functions.md), [architecture-overview.md](architecture-overview.md), [decisions/README.md](decisions/README.md), `.kiro/steering/architecture-guardrails.md`.

Criticality: Critical blocks merge. Important requires review. Advisory reports only.

## 1. Allowed dependency matrix (Rust, internal crates)

Rows may depend on columns marked Y. Everything else is forbidden. The matrix is stored as a version-controlled file (planned `architecture/allowed-deps.toml`) and checked by FF-01.

| From \ To | theory | score | guitar | pattern | notation | playback | project | dsp | app | audio | midi | musicxml | ffi |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| theory | | | | | | | | | | | | | |
| score | Y | | | | | | | | | | | | |
| guitar | Y | | | | | | | | | | | | |
| pattern | Y | Y | Y | | | | | | | | | | |
| notation | Y | Y | | | | | | | | | | | |
| playback | Y | Y | | | | | | | | | | | |
| project | Y | Y | | Y (Phase 6) | | | | | | | | | |
| dsp | | | | | | | | | | | | | |
| app | Y | Y | Y | Y (Phase 6) | Y | Y | Y | Y | | | | | |
| audio | Y | | | | | Y | | Y | Y | | | | |
| midi | Y | Y | Y | | | Y | | | | | | | |
| musicxml | Y | Y | | | | | | | | | | | |
| ffi | | | | | | | | | Y | Y | Y (Phase 4) | Y (Phase 6) | |
| cli | | | | | | | Y | | Y | Y (optional) | Y (Phase 4) | Y (Phase 6) | |

`midi` (`mc-midi`, Phase 4) is the SMF import and export adapter ([ADR-023](decisions/ADR-023-midi-adapter-crate.md)); only the adapters `ffi` and `cli` use it. `musicxml` (`mc-musicxml`, Phase 6) is the MusicXML export adapter ([ADR-033](decisions/ADR-033-musicxml-export-adapter.md)); it reads the score through `mc-score` and spells pitches through `mc-theory`, and only `ffi` and `cli` use it.

Future `mc-analysis` (Phase 8): may depend on `theory` and `dsp` only. `app` may depend on it. It is added to the matrix file in Phase 8 by ADR.

`ffi` depends only on `app` and `audio`. Types it needs for DTO mapping are re-exported by `mc-app` (for example `mc_app::types::*`). This makes it impossible for the bridge to call aggregate mutation methods directly, and FF-01 enforces it. `cli` depends on `app`, `project` (file adapter) and optionally `audio`, matching the DAG in [architecture-overview.md](architecture-overview.md). This was tightened during the independent review (see [architecture/README.md](README.md)).

## 2. Rules

### GR-01 Domain logic must not depend on Flutter
- Mechanism: FF-01 (matrix), FF-02 (cargo-deny bans `flutter_rust_bridge` everywhere except `mc-ffi`).
- Criticality: Critical. Active: Phase 1 (matrix), Phase 3 (frb ban).

### GR-02 Domain logic must not depend on SQLite
- Mechanism: FF-02 bans `rusqlite` and `libsqlite3-sys` except in an approved adapter crate (none in MVP).
- Criticality: Critical. Active: Phase 1.

### GR-03 DSP must not depend on presentation
- Mechanism: FF-01 (`mc-dsp` has no internal deps), FF-02 (external allow-list: FFT crates only), FF-03 (no std::fs, std::net, std::thread).
- Criticality: Critical. Active: Phase 5.

### GR-04 AI must not bypass domain validation
- Mechanism: Analysis crates may not depend on `mc-score` mutation APIs (FF-01 matrix gives analysis only theory and dsp). Candidates are accepted only through the `AcceptCandidate` handler, which issues ordinary commands. FF-31 integration test asserts invalid candidates are rejected by normal validation. Review checklist.
- Criticality: Critical plus review. Active: Phase 8.

### GR-05 Infrastructure must not own domain decisions
- Mechanism: FF-01 (domain crates cannot depend on adapters). Review checklist: adapters translate and call application APIs; no musical rules in adapters or Dart.
- Criticality: Important (partly review-only: "no rules in adapters" cannot be fully automated).

### GR-06 Contexts must not access each other's internals
- Mechanism: Rust visibility (`pub(crate)` by default, explicit public API), FF-22 (`cargo public-api` diff reviewed), Dart `lib/src` privacy plus FF-05 import-boundary test.
- Criticality: Critical (FF-05), Important (FF-22). Active: Phase 1 (Rust), Phase 3 (Dart).

### GR-07 Circular dependencies between contexts are prohibited
- Mechanism: Cargo rejects crate cycles by construction. FF-06 checks Dart package cycles. FF-26 reports intra-crate module cycles.
- Criticality: Critical (crates, packages), Advisory (modules).

### GR-08 Project format changes follow versioning and migration policy
- Mechanism: FF-11 (golden corpus and round-trip suite), FF-32 (DTO JSON schema snapshot; any diff requires a format version bump and a migration or explicit "additive" note).
- Criticality: Critical. Active: Phase 1.

### GR-09 All imported data passes validation
- Mechanism: Importers construct scores only through `ScoreBuilder` (validating). Adapter crates cannot access score internals (Rust visibility). FF-15 fuzzing, fixture tests with ImportReport assertions.
- Criticality: Critical (fixtures), Important (fuzzing, scheduled). Active: Phase 4 (MIDI import).

### GR-10 Audio real-time constraints are explicitly protected
- Mechanism: FF-16 (assert_no_alloc around callback bodies in tests), FF-17 (lint script over `mc-audio/src/rt/`), FF-18 (glitch test). Review checklist for RT code.
- Criticality: Critical (FF-16, FF-17), Important (FF-18). Active: Phase 4.

### GR-11 Platform-specific code stays behind adapters
- Mechanism: FF-07 (`cfg(target_os = ...)` only in adapter crates), FF-08 (Dart `dart:io` and `Platform` only in the `app_platform` package and `app`), FF-04 (domain crates compile for wasm32).
- Criticality: Important (FF-07, FF-08), Critical (FF-04).

### GR-12 Architecture fitness functions are version-controlled
- Mechanism: All checks live in the repository (`xtask`, `architecture/` config, `deny.toml`, `clippy.toml`, CI workflow). CI runs them from the repository, not from external configuration.
- Criticality: Critical by construction. Active: Phase 0.

### GR-13 Changes to critical architecture rules require explicit review
- Mechanism: Protected paths: `architecture/`, `deny.toml`, `clippy.toml`, `xtask/src/fitness/`, the CI workflow, `docs/architecture/dependency-rules.md`, `docs/architecture/fitness-functions.md`. FF-19: CI fails if a change touches a protected path without a commit trailer `Architecture-Change: ADR-NNN` referencing an existing ADR. CODEOWNERS on protected paths for when collaborators join.
- Criticality: Critical. Active: Phase 0.
- Solo-developer note: self-review is still review. The trailer forces a written decision.

### GR-14 Coding agents must not weaken or delete fitness functions to pass tests
- Mechanism: `.kiro/steering/architecture-guardrails.md` (always included), FF-19 (protected paths), FF-20 (fitness inventory manifest: a list of fitness test ids that cannot shrink without an ADR trailer).
- Criticality: Critical. Active: Phase 0.

## 3. Rust lint configuration for domain crates (FF-03)

Planned `clippy.toml` `disallowed-methods` and `disallowed-types` for `mc-theory`, `mc-score`, `mc-guitar`, `mc-pattern`, `mc-notation`, `mc-playback`, `mc-dsp`: `std::fs::*`, `std::net::*`, `std::thread::spawn`, `std::time::Instant::now`, `std::time::SystemTime::now`, `std::env::var`. Time and randomness come through parameters or ports.

`mc-project` is allowed `std::io::Read/Write/Seek` traits but not `std::fs`.

### 3a. Unsafe code (ADR-022, FF-34)

All crates use the workspace lint table (`[lints] workspace = true`), which sets `unsafe_code = "forbid"`. There are exactly two exceptions (ADR-022, extended by [ADR-027](decisions/ADR-027-unsafe-exception-mmcss-module.md)):

1. `mc-ffi` restates the workspace lints with `unsafe_code = "deny"` so that the generated module may carry `#[allow(unsafe_code)] mod frb_generated;`. Hand-written `mc-ffi` code has no `unsafe`, and generated files are never edited by hand (FF-12).
2. `mc-audio` restates the workspace lints with `unsafe_code = "deny"` so that `#[cfg(windows)] #[allow(unsafe_code)] mod mmcss;` may make the one MMCSS thread-promotion call. `mmcss.rs` contains exactly one `unsafe` token with a `// SAFETY:` comment directly above it; nothing else in `mc-audio` may use `unsafe`.

No other `allow(unsafe_code)` exists in the repository. Any other crate or module that needs `unsafe` requires a new ADR.

## 4. Review checklist (for changes not fully automatable)

- Does any adapter or Dart code decide a musical rule? (GR-05)
- Does a new public item expose internal representation? (GR-06)
- Does a new event carry content instead of ranges? ([ADR-006](decisions/ADR-006-event-model.md))
- Does RT code lock, allocate, log, or perform I/O? (GR-10)
- Does a change add `unsafe`, an `allow(unsafe_code)` or a crate-local `[lints]` table? (ADR-022, ADR-027, FF-34)
- Does a new dependency pass license policy and is it necessary? ([ADR-015](decisions/ADR-015-dependency-license-policy.md))
