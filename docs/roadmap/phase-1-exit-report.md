# Phase 1 Exit Report

Purpose: Evidence against the Phase 1 acceptance criteria (theory, score, project format, application layer, CLI), benchmark results, remaining risks, architecture changes supported by implementation evidence, and decisions needed.
Status: Final. Phase 1 accepted by the user on 2026-10-31 with the decisions in section 9. Milestone M1 reached. CI status in section 7.
Related documents: [implementation-phases.md](implementation-phases.md), [milestones.md](milestones.md), [m0-exit-report.md](m0-exit-report.md), [risk-register.md](risk-register.md), [fitness-functions.md](../architecture/fitness-functions.md), [domain-invariants.md](../architecture/domain-invariants.md), [benchmarks/reference-machine.md](../../benchmarks/reference-machine.md).

## 1. Outcome

Phase 1 delivered its planned production scope and nothing beyond it: `mc-theory`, `mc-score`, `mc-project`, `mc-app` and `mc-cli`. No UI, audio, AI, tuner, Guitar context or platform work was pulled forward. Spike code was not copied; the format and file-store code was reimplemented against the architecture, using S5's decisions.

Order of work as requested: `architecture/allowed-deps.toml` and FF-01 were committed before the first production crate (commit `3705896`, then crates in `19f2f88` and later).

## 2. Acceptance criteria

| Criterion ([implementation-phases.md](implementation-phases.md) Phase 1) | Evidence | Result |
|---|---|---|
| All H rules covered by tests (FF-24 report clean for H rules) | `cargo xtask fitness rule-coverage`: 38 of 55 rule ids have tests; H-01 to H-12 all covered. Uncovered ids belong to later phases (G, PT, AU, AI, IM, D-02 to D-06, PB-09) or have no Phase 1 surface (E-05 selection, P-01 asset references; format 1.0 has no assets) | Met |
| PB-01 to PB-08 (effective bar duration) pass | `crates/mc-score/tests/effective_duration.rs`, including the PB-08 property test | Met |
| Round-trip items 1 to 8 pass; item 4 at 4a and 4b only | `crates/mc-project/tests/roundtrip.rs` (rt1 to rt8, plus rt9 mutation property tests); 4b in `crates/mc-app/tests/session.rs` (`rt4b_...`) | Met, with item 5 vacuous (section 3) |
| CLI builds the large reference score and round-trips it | `mc-cli fixture large` wrote 288 000 notes (1.69 MB package) in 440 ms; `mc-cli validate` and `mc-cli roundtrip` passed with identical score and identical score.json bytes | Met |
| Benchmarks recorded | Section 4 | Met |

Milestone M1 demo ("CLI builds a song, edits, undoes, saves, reloads; large reference score round trip"): `mc-cli run crates/mc-cli/examples/song.script --out song` (written as `song.giproj`) applies 28 steps including `undo` and `redo`, reports one intended D-01 diagnostic, and `info`, `validate` and `roundtrip` pass on the result.

## 3. Round-trip suite detail (project-format section 14)

| Item | Test | Status |
|---|---|---|
| 1 every released fixture loads | `rt1_p03_p05_...` (2 fixtures of format 1.0) | Pass |
| 2 load, save, load equality | `rt2_...` | Pass |
| 3 byte-stable output | `rt3_...` against golden score.json; CLI regeneration was byte-identical | Pass |
| 4a U1 detection | `rt4a_...` x3: 56 injected unknown fields at all DTO levels, reserved entries, unknown optional features, newer minor | Pass |
| 4b U2 Save As protection | `rt4b_...` in mc-app | Pass |
| 4c U3 preservation | Format 1.1 | Not in scope |
| 5 migration chain | `rt5_...`: no older version exists; major 0 refused; the test fails once a second version appears without migration tests | Vacuous by design |
| 6 newer major, unknown required features | `rt6_p03_...` | Pass |
| 7 corruption and malicious packages | `rt7_...` x3 | Pass |
| 8 schema snapshot (FF-32) | `rt8_ff32_...`; negative control: adding a DTO field failed the test | Pass |
| 9 fuzzing | Property-test mutation of packages and JSON (512 cases); cargo-fuzz is FF-15 in Phase 4 | Partial by plan |

## 4. Benchmarks (reference machine, release, criterion)

| Benchmark | Result | Budget | Margin |
|---|---|---|---|
| Note edit incl. undo snapshot, typical (p99 of 1 000) | 11.7 µs | 16 ms | about 1 300x |
| Note edit incl. undo snapshot, large (p99 of 1 000) | 33.5 µs | 33 ms | about 1 000x |
| Undo 100 + redo 100, large | 2.2 ms | none | |
| Fret shift of all 20 tracks x 300 bars (20 commands) | 14.3 ms | none | |
| Full validation, large | 36.8 ms | part of open | |
| Bar start times, large (with repeats) | 10.5 ms | none | |
| Open typical (ZIP, SHA-256, JSON, U1, validation) | 24 ms | 500 ms | 20x |
| Open large | 387 ms | 2 s | 5x |
| Save typical | 17 ms | 300 ms | 17x |
| Save large | 342 ms | off UI thread | |
| Undo history after 1 000 edits, large | 200 steps, 0.8 MB | 64 MB | |

Memory baselines (FF-23, structural, deterministic): large score 8.8 MB (design assumption was 15 to 30 MB), typical 0.39 MB, undo step 3.9 KB (large) and 1.9 KB (typical). Command budgets do not yet include projection time; that is measured in Phase 3.

## 5. Fitness functions

21 of 33 active (5 from Phase 0 and all 16 of Phase 1); `current_phase = 1`.

| Id | Check | Where | Evidence |
|---|---|---|---|
| FF-01 | Dependency matrix | `xtask fitness deps` | 5 crates checked; negative tests in xtask |
| FF-02 | External crate bans | cargo-deny `bans.deny` | Negative control: `rtrb` in `mc-score` rejected as banned |
| FF-03 | Domain purity | per-crate `clippy.toml` (template in `architecture/`), clippy `-D warnings` | Negative control: `Instant::now` rejected; it also caught file I/O in a domain test, fixed by embedding fixtures |
| FF-04 | wasm32 compile | `xtask fitness wasm` | mc-theory, mc-score, mc-project |
| FF-07 | Platform cfg | `xtask fitness platform` | 4 crates |
| FF-11, FF-32 | Round trip, schema | tests; fixture-reference wiring in FF-20 | Negative controls: DTO change, unlisted fixture |
| FF-13, FF-14 | Invariants and undo under random command sequences | proptest, 256 cases each, 15 operation kinds including invalid ones | Pass; rejected commands proven side-effect free |
| FF-21 | Benchmarks | 2 criterion suites; CI compiles them | Section 4 |
| FF-22 | Public API | weekly; snapshots in `architecture/public-api/` | Generated locally with cargo-public-api 0.52.0 |
| FF-23 | Memory | `crates/mc-app/tests/memory.rs` | Structural, not peak memory (decision D2) |
| FF-24 | Rule coverage | CI report | Section 2 |
| FF-27 | Coverage | weekly | Local run: 79.5% lines, 77.6% functions |
| FF-28 | Complexity | CI report | 0 cognitive-complexity warnings; 6 functions over 100 lines (advisory) |
| FF-30 | Freshness | weekly | `zip` 9.2.0 available (pinned 9.1.3); upgrade needs S7 re-audit |

Workspace tests: 125 passing (all crates including the xtask self-tests), 0 failing.

## 6. Architecture changes supported by implementation evidence

| Change | Evidence | Where recorded |
|---|---|---|
| Tempo offsets are bounded by the nominal bar length, not `E(m)` | `E(m)` changes on every content edit, so allowing offsets up to it would let a beat deletion break H-11 | domain-invariants 4a rule 6 |
| H-12 made precise: no nesting, counts 2 to 16, alternate-ending placement | Needed to implement playback order deterministically | domain-invariants H-12 |
| Dirty state is content-based; revision stays monotonic | Test showed "undo back to saved" stayed dirty with revision-based tracking | score-model-and-undo section 5 |
| The most recent undo step is always kept even above the byte budget | Test: a single 8 KB step exceeded a 4 KB budget | score-model-and-undo section 5 |
| Save retries classified by `ErrorKind`, not raw OS codes | Codes 5 and 32 mean other errors on Linux; FF-07 forbids `cfg` in mc-app | ADR-008 amendment 4 |
| `mc-app` needs no `tempfile`; `mc-cli` needs no `clap` | Implementation used std only | architecture-overview crate table |
| Domain test code embeds fixtures; generation moved to the CLI adapter | FF-03 rejected `std::fs` in mc-project tests | fitness-functions FF-11 |
| FF-23 measured structurally | `unsafe_code = "forbid"` blocks a counting allocator | fitness-functions FF-23; decision D2 |

No ADR was reversed. No crate boundary, dependency edge or domain rule from the accepted baseline was weakened.

## 7. CI status

Local runs of every CI step passed (fmt, clippy `-D warnings`, workspace tests, bench compilation, all xtask checks, Mermaid, cargo-deny). GitHub Actions passed on `16d621e` (branch `phase-1-core`): governance job 279 s (fmt, clippy, 125 tests, benchmark build, fitness checks, Mermaid) and cargo-deny 38 s. The new weekly workflow (FF-22, FF-27, FF-30, macOS and Linux tests) runs on schedule or manual dispatch and has not run on GitHub yet; its commands were run locally on Windows, except the macOS and Linux test jobs.

## 8. Remaining risks

| Risk | State |
|---|---|
| R-04 TAB editor effort | Unchanged; Phase 3 is next for the UI |
| R-06 unknown content | U1 and U2 done and tested; U3 deferred to format 1.1 |
| R-07 undo memory or latency | Lowered: about 1 000x margin on latency, 0.8 MB for 200 large-score steps |
| R-21 single-machine evidence | Unchanged; all timings come from the reference machine |
| macOS and Linux | Never built yet; first weekly run will show |
| Real OneDrive and antivirus locks | Still untested (simulated lock tests pass) |
| Projection cost | Not measured until Phase 3; command budgets currently cover the domain only |

## 9. Decisions (user, 2026-10-31)

- D1, Q-08: the product extension is `.giproj` (GuitarInspire Project). It stays outside the Music Core: `mc-project` reads bytes and `mc-app` accepts any path; `mc-cli` defines it and appends it to output paths that have no extension, without refusing other names. File identity and compatibility come from the manifest `format` and `format_version` and the entry schemas (project-format section 2a). Internal names are unchanged: test fixtures `.package`, recovery files `.recovery`.
- D2, FF-23: keep the deterministic structural measurement; the workspace-wide `unsafe_code = "forbid"` stays. Documented limitation: it is not a process-level or peak-memory measurement (strings, allocator overhead and transient buffers during file open are not counted). Revisit a test-only exception only if peak memory becomes necessary for a concrete decision and no safe approach is adequate.
- D3, `zip`: stay on 9.1.3; review 9.2.0 at the next scheduled dependency and license review, earlier only for a security fix or a concrete format issue. An upgrade reruns cargo-deny, the FF-11 round-trip suite and the format benchmarks.

## 10. Closure

Phase 1 is accepted and closed when this branch is merged into `main` with CI passing. Known limitations in section 8 are kept as they are; they belong to later phases. Next phase per the roadmap: Phase 2 (Guitar core), on a new branch.
