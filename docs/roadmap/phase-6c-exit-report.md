# Phase 6c Exit Report (standard notation and PDF)

Purpose: Evidence for Phase 6c (standard notation above TAB, and PDF export), the evidence gaps proposed as conditions of acceptance, and what comes next.

Status: Final. Phase 6c accepted by the user on 2026-11-27, with the evidence gaps below as conditions of acceptance. **Read "Known evidence gaps" before relying on this report.**

Related documents: [implementation-phases.md](implementation-phases.md), [milestones.md](milestones.md), [mvp-known-limitations.md](mvp-known-limitations.md), [risk-register.md](risk-register.md), [phase-6d-exit-report.md](phase-6d-exit-report.md), [ADR-037](../architecture/decisions/ADR-037-bravura-smufl-font.md) to [ADR-041](../architecture/decisions/ADR-041-notation-engine-metrics-cache-pdf.md), [S11 spike](../../spikes/s11-notation/README.md).

## 1. Outcome

Spike S11 was the entry gate. ADR-037 to ADR-041 were then accepted. Every slice had its criteria fixed in ADR-041 before any code, and thresholds were never lowered to match a result.

| Slice | What the user gets | Result |
|---|---|---|
| S11 | Feasibility: an engine prototype, a MuseScore cross-check (229 of 229), timing, PDF routes | Go (user) |
| 6c-1 | Spelling, key alterations, bar accidentals and transposition in `mc-theory` | T1 to T7 met |
| 6c-2 | The notation staff in `mc-notation`: semantic layer, engraving, bundled Bravura 1.482 with a generated metrics table (FF-35), bar-plan cache, the S11 checks as tests, goldens | L1 to L13 met (L12 by the user) |
| 6c-3 | Notation painted above TAB in the app, switched per track | F1 to F9 met; LARGE end to end an accepted gap |
| 6c-4 | Clicking on notation places the cursor and selects (read-only) | H1 to H9 met (H7 and H8 by the user) |
| 6c-5a | Pagination (A4 and Letter, title, page numbers, split systems) and a print staff size of 0.6 (one Revise by the user) | PG1 to PG9 and PR1 to PR5 met |
| 6c-5b | Crate `mc-pdf`; `mc-cli export-pdf`; File > Export PDF... with paper and spacing | PD1 to PD9 met (PD9 the user's Edge check) |
| 6c-6 | Engraving polish: spacing, courtesy signatures, key-change naturals and double bar lines, centred bar rests, ties, no TAB rhythm under notation | EP1 to EP9 met; goldens replaced after the user's review |
| 6c-7 | Benchmarks, this report, the documents, the merge | EX1, EX3 to EX9 met; EX2 an accepted gap |

Per-criterion evidence and the user's decisions are in ADR-041 sections 8 to 14. This report does not repeat them.

## 2. Commits

All on `phase-6c-notation`, after `main` at `ba2227e`. Every commit that touches a protected path carries `Architecture-Change` with ADR-037, ADR-040 or ADR-041 (FF-19 ok).

- **S11 and design**: `06deca4`, `8e9207b`, `fa3856f`, `aefb4a3`, `912f2a1`.
- **6c-1** `e2edd1d`; **6c-2** `958bf6a`, `17c8296`; **6c-3** `1745a8d`; **6c-4** `4b4f6ee`; **6c-5a** `085ac87`, `1bf7091`; **6c-5b** `265e430`; **6c-6** `e131dda`.
- The other commits record criteria, the user's decisions and closures, then this report.

## Known evidence gaps

Proposed as conditions of acceptance. All evidence comes from one machine (ADR-025, R-21 open), which has a fast and a slow state of unknown cause (ADR-031).

- **(a) End to end in the app, TYPICAL with notation after 6c-6 (EX2; accepted as a gap by the user, 2026-11-27).**
  - Criterion: `p99_compute_bound_us` ≤ 16 ms (H7). 10 runs of the final build: 6 within (9.9 to 13.8 ms), 4 over (20.3 to 31.6 ms); the runs over also have a higher median.
  - An interleaved A/B against the build before 6c-6 (`27cbab7`), 4 runs each: before 9.8 to 12.0 ms with none over; after 9.9 to 31.6 ms with 1 over. The median compute rose from about 5.0 to 6.1 ms (+22 %), and the system 0 payload from 48 335 to 60 523 bytes, because EP1 puts 3 bars per system instead of 2.
  - Whether the runs over come from the machine state or from the larger frame is not settled by these runs. The threshold is unchanged.
  - To close: runs on a quiet machine or a second machine, or a reduction of the painted content per frame (its own criteria).
- **(b) End to end in the app, LARGE with notation** (6c-3, accepted then): within 33 ms only in the machine's fast state. Not re-measured.
- **(c) The Edge check covers Windows only.** PDF readers on other platforms were not tried.
- **(d) The MuseScore cross-check covers the S11 inputs only** (S11, 229 of 229). The 6c-6 polish was judged by the user's review of images, not compared with another engraver.
- **(e) The weekly portability and fuzz jobs were not run on this branch** (EX7). They run on `main` after the merge. Locally: the public-API diff (FF-22) shows no change against the snapshots, FF-04 passes, and coverage of the new code is high (lines: `mc-notation` layout 99.6 %, engrave 99.6 %, page 99.7 %; `mc-pdf` lib 96.5 %, fonts 98.8 %, paint 96.4 %; `mc-theory` key 100 %).
- **(f) Known limitations left until after 6c by the user** (ADR-041 section 13): no track names in print; markers and bar numbers repeated on continued parts; crossing voices overlap.

Carried over and unchanged by 6c: the gaps of Phases 6b, 6a and 6d. They were not re-measured.

## 3. Benchmarks at the final state (EX1)

Release build on the reference machine, 3 runs; p99 in ms unless stated.

| Budget | Threshold | Runs |
|---|---|---|
| L11 TYPICAL, notation and TAB, in view | 16 | 0.76 / 0.74 / 0.59 |
| L11 LARGE 4 tracks | 33 | 0.74 / 1.05 / 0.70 |
| L11 LARGE 20 tracks | 33 | 4.15 / 4.63 / 3.36 |
| Edit and TAB relayout TYPICAL / LARGE | 16 / 33 | 1.21, 1.20, 1.07 / 3.78, 2.89, 3.28 |
| P1 TYPICAL in view | 16 | 0.26 / 0.26 / 0.22 |
| P2 LARGE 4 tracks in view | 33 | 0.59 / 0.46 / 0.51 |
| P3 LARGE 20 tracks in view | 33 | 1.93 / 1.55 / 3.66 |
| PG7 paginate and lay out every system, LARGE 20 tracks | 2 000 (reference) | 338 / 321 / 327 |
| PD6 export TYPICAL (max of 5) | 2 000 | 195 / 119 / 93 |
| PD6 export LARGE 20 tracks (median) | reported | 2 584 / 2 848 / 2 552 |
| Chord voicings, worst (C9) | 50 | 3.24 / 1.96 / 2.48 |
| PE-15 pattern to 64 bars (TYPICAL) | 20 | 0.86 / 0.80 / 0.86 |

The "full" P2 and P3 rows of the bench (laying out every system) remain over 33 ms for P3 (66 to 75 ms). They were recorded as failing in ADR-031, and the app does not use that path (fallback 2, layout in view).

## 4. Faults found and corrected in 6c

Each was found by a planned test, a check rule or a review tool, and fixed with a test. None needed a change to the staff model or the boundaries.
- S11: six engine faults found by a second review (S11 README).
- 6c-2: rests of two voices on each other's heads; tuplet numbers close to another voice; the ledger-neighbour check comparing voices instead of columns; the outline header width.
- 6c-3: the view's layout cache key missed the staff configuration.
- 6c-5a: a stave taller than page 1's body but not later bodies failed instead of moving to page 2; at a layout margin of 0 the final bar line crossed the content edge (PR3).
- 6c-6: the N2 alignment check assumed centred columns (adjusted to compare with TAB's fret numbers, not loosened).
- Agent process faults, recorded for honesty: several edits were run in parallel with builds or commits, so some results were stale and were re-run; one commit message (`aeb5aea`) named content it did not hold, and one edit (`265e430`) removed an ADR heading. Both were corrected in later commits.

## 5. Fitness functions and governance

- New crate `mc-pdf` (adapter, ADR-041) with edges from `mc-ffi` and `mc-cli`, in `allowed-deps.toml` and the dependency-rules matrix. FF-01: 16 crates.
- New dependencies, all MIT or Apache-2.0 (cargo-deny ok):
  - `skrifa` (replacing `ttf-parser` after RUSTSEC-2026-0192, user decision);
  - `sha2` for FF-35, a dev-dependency;
  - `subsetter` without default features, `pdf-writer` and `miniz_oxide`.
- FF-35 (bundled Bravura checksums) was added. No fitness function, lint or rule was weakened.
- Governance changes, each approved by the user:
  - ADR-015 amendment 1 (the OFL font);
  - ADR-040 superseding ADR-033 choice 5.3;
  - ADR-041 decision 6 amended so that an engraving change may replace a golden only with an attributed review diff the user approves.
- No `unsafe` (FF-34: 17 crates). The bindings were regenerated by codegen only and were stable on a second run (FF-12).
- The project format is unchanged; the notation setting is view state.

## 6. Effort (EX4; measure (c), a record, not a check)

| Slice | Baseline (FTE weeks) | Agent time |
|---|---|---|
| 6c-1 | 0.5 to 1 | 14 min |
| 6c-2 | 3 to 5 | 76 min |
| 6c-3 | 2 to 3 | 71 min |
| 6c-4 | 0.5 to 1 | 18 min |
| 6c-5a and 6c-5b | 2 to 3 | 29 + 13 + 100 min |
| 6c-6 | 1 to 2 | 85 min |
| 6c-7 | 0.5 | about 1 h (2026-11-27 09:30 onwards) |
| Total | 9.5 to 15.5 | about 7.5 h |

- Calendar time: from S11 (2026-11-12) to this report (2026-11-27), 15 days, mostly waiting for the user's reviews and decisions.
- The user's time (reviews, H7 and H8, the Edge check, the image reviews) was not recorded.
- Agent time is not comparable with FTE weeks, which assumed a human developer (measure (c), user 2026-11-16).

## 7. R-13 at the exit (EX5)

- Condition 1 (fixed scope, no scope cut, no failed criterion except accepted gaps): met. 6c-5a was revised once by the user's choice; EX2 is an accepted gap.
- Condition 2 (Flutter painting within the decision 7 criteria): **not fully met.** TYPICAL end to end is within 16 ms in 6 of 10 runs after 6c-6 (gap a), and LARGE only in the fast state (gap b).
- Condition 3 (no new class of fault needing a model change): met.
- **Decision (user, 2026-11-27): R-13 stays at Medium** and is not closed, because of gap (a).

## 8. Closure

Decisions (user, 2026-11-27):
- Phase 6c is accepted, with gaps (a) to (f) on the record as conditions of acceptance (EX9).
- `phase-6c-notation` is merged into `main` with `--no-ff`. The merge commit `5a97a19` carries the trailer `Architecture-Change: ADR-041` (EX8). CI on the merge commit passed (run #139).

Next: Phase 6 is complete (6a, 6b, 6c and 6d). The next phase is the user's choice, from [implementation-phases.md](implementation-phases.md).
