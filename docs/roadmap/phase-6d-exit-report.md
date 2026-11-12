# Phase 6d Exit Report (Guitar Pro import and export)

Purpose: Evidence for Phase 6d (Guitar Pro GP3 to GP5 import, and export added by the user), the evidence gaps accepted at closure, and what comes next.
Status: Final. Phase 6d accepted by the user on 2026-11-11, with the evidence gaps below as conditions of acceptance. **Read "Known evidence gaps" before relying on this report.**
Related documents: [implementation-phases.md](implementation-phases.md), [milestones.md](milestones.md), [mvp-known-limitations.md](mvp-known-limitations.md), [mvp-release-notes.md](mvp-release-notes.md), [phase-6a-exit-report.md](phase-6a-exit-report.md), [ADR-036](../architecture/decisions/ADR-036-guitar-pro-adapter.md), [S10 spike](../../spikes/s10-guitar-pro/README.md), [benchmarks/tuxguitar-guitarpro.md](../../benchmarks/tuxguitar-guitarpro.md).

## 1. Outcome

Spike S10 was the entry gate. It read GP3 to GP5 5.00 with its own clean-room reader and wrote GP5. ADR-036 was then accepted with criteria G1 to G15 fixed before any code. Thresholds were never lowered to match a result.

| Slice | What the user gets | Result |
|---|---|---|
| S10 | Format knowledge: 54 fixtures written by TuxGuitar 2.0.1, eight corrections to the public format description, the version rules | Pass for GP3 to GP5 5.00; GP7 and GPX not assessed (deferred by the user) |
| 6d-1 | Crate `mc-guitarpro`: import of GP3, GP4 and GP5 5.00 into a validated score with a report; export to the same three versions with a report | G1 to G4, G6 to G8, G10 to G12 met |
| 6d-2 | `mc-cli` (`gp:` source, `export-gp`), the bridge API, `GuitarProService` (real and fake, one contract) | Part of G13 and G14 |
| 6d-3 | File > Import Guitar Pro... and File > Export Guitar Pro... | G13 met |
| 6d-4 | Fuzz target `import_gp` in the weekly job | G5 met except the first fuzz run (gap a) |
| 6d-5 | The TuxGuitar check, timing, and the docs | G9 met with a TuxGuitar limit (gap c); G15 met |

Per-criterion evidence is in ADR-036 section 8, and the user's decisions after the results are in section 9. This report does not repeat them.

## 2. Commits

All on `phase-6d-guitar-pro`, after `main` at `af06079`. Every commit that touches a protected path carries `Architecture-Change: ADR-036` (FF-19 ok).

- **S10**: `55a5b34` (plan), `8d28f8c` (fixtures), `68626f4` (reader), `e684c23` (chord diagrams, writer, mapping and estimate).
- **Design**: `d0fdd11` (ADR-036, S10 result, R-14).
- **6d-1** `9e47f04`, **6d-2** `7010a1c`, **6d-3** `e4edfd7`, **6d-4** `3b4dcb9`, **6d-5** `6bdc990`, then the closure commit with this report.

## Known evidence gaps

These are conditions of acceptance (user decision 2026-11-11), to be resolved later. All evidence comes from one machine (ADR-025, R-21 open).

- **(a) The fuzz target has not run in CI yet (G5).**
  - `import_gp` is a step of the weekly job, seeded with the 54 fixtures.
  - **Local run, 2026-11-11** (after the merge; Windows, `nightly-2026-10-31`, cargo-fuzz 0.13.1, AddressSanitizer runtime from MSVC 14.50): 300 s, **1 801 744 inputs, 0 crashes**, corpus grown from 54 to 856 files.
  - The CI job could not have reached this step yet. Its first step, the MIDI target, had failed since 2026-11-09 (see section 7), and a failed step stops the job.
  - To close: the first green weekly run on Linux. A crash becomes a regression test.
- **(b) No file written by Guitar Pro itself.**
  - Every fixture was written by TuxGuitar 2.0.1, and the export was read back only by TuxGuitar and by us.
  - Guitar Pro's own files may differ in padding values, optional sections and text encoding. The importer refuses rather than guesses where the layout is unknown, so such a file may fail where a guess might have worked.
  - Three questions stay open until such a file exists:
    - the meaning of the brush direction bytes (choice 7.1);
    - the text encoding (choice 7.3);
    - the chord layouts no fixture shows (choice 7.7).
  - To close: lawful sample files written by Guitar Pro.
- **(c) TuxGuitar cannot hold palm mute and let ring on one note (G9).** It keeps only the palm mute, so 2 of 1 849 lines differ. Our file sets both bits, as the format allows, and the encoding is not changed.
- **(d) GP5 5.10, GPX (GP6) and GP7+ are refused** (choice 7.8). They wait for lawful samples.
- **(e) No legal review.** The rules are clean-room and trademark (ADR-036 section 2, R-14). This is not legal advice, and a review is advised before a commercial release.

Smaller items recorded, not gaps in the criteria:
- The app has no About screen. The non-affiliation statement is in the docs and in the crate documentation (ADR-036 clarification 6).
- An import of a file with only percussion tracks is refused, by the user's decision (ADR-036 section 9).
- Effects the score does not hold (bends, slides, chord diagrams, lyrics, song information and more) are reported and dropped. They return when the score model gains them, each by its own ADR.

Carried over and unchanged by 6d: the gaps of Phase 6b and 6a ([phase-6b-exit-report.md](phase-6b-exit-report.md), [phase-6a-exit-report.md](phase-6a-exit-report.md)). They were not re-measured. The Windows integration tests passed 7 of 7 in the one local run of 6d-3.

## 3. Defects found and corrected in 6d

- **A tempo change moved to a beat past the nominal end of an overfull bar** was read back in the next bar. Moves now stay inside the bar (found by the round-trip property test).
- **Two tempo changes placed on one position lost one of them silently** on re-import: two empty beats at a bar start, or a change on another track's beat at the same offset. One beat now carries both and the replacement is counted (found by the coverage test and the round-trip property test).
- **TuxGuitar read an empty beat in GP3 and GP4 as a quarter rest.** The exporter writes no empty beats in those versions (found by the G9 check).
- **In the spike, chord diagrams stopped the reader.** The layout was found by byte analysis (107 bytes in GP4 and GP5) and is read in the crate.

## 4. Fitness functions and governance

- New crate `mc-guitarpro` (adapter, phase 6, ADR-036) with edges from `mc-ffi` and `mc-cli`, in `allowed-deps.toml` and the dependency-rules matrix. FF-01: 15 crates.
- No external dependency added (cargo-deny ok). No `unsafe` (FF-34: 16 crates).
- The bindings were regenerated by codegen only. A second run was identical (FF-12; 18 generated files).
- The project format is unchanged; no fixture or schema snapshot was touched.
- Three deliberate breakages of the adapter were each caught by its tests: let ring dropped on export, the GP3/GP4 repeat rule removed, the brush direction swapped.
- FF-15 gains a second target. No fitness function, lint or rule was weakened.

## 5. CI status

Every slice commit up to `6bdc990` passed GitHub Actions (run #103 for the last). The merge commit is checked again after it is pushed.

## 6. Closure

Decisions (user, 2026-11-11):
- The eight choices of ADR-036 section 7 are confirmed, and a file with only percussion tracks stays refused (ADR-036 section 9).
- Phase 6d is accepted, with gaps (a) to (e) on the record as conditions of acceptance.
- `phase-6d-guitar-pro` is merged into `main` with `--no-ff`. The merge commit carries the trailer ADR-036.

Next: Phase 6c (standard notation and PDF), entry gate S11. It is the last part of Phase 6 not yet started.

## 7. Weekly job failures found at closure (pre-existing, not caused by 6d)

While checking CI after the merge, the scheduled weekly run of 2026-11-09 (#2, on `c789862`, the Phase 6b merge) was found red in three jobs. Nobody had noticed. The weekly run is Important and does not block merges. The logs need a signed-in account, so each cause was established as follows:

| Job | Cause | Evidence | Fix (branch `weekly-ci-fixes`) |
|---|---|---|---|
| `fuzz` (FF-15) | **A real defect.** midly 0.5.3 panics (negating `i8::MIN`) on an SMPTE header whose frame byte is 0x80. `mc-midi` caught the panic with `catch_unwind`, but libfuzzer aborts on any panic, even a caught one. The fuzzer also found the same header later in a file, which midly reads while collecting the tracks. | Reproduced locally with cargo-fuzz 0.13.1: the crash came at once, both before the fix and after a first, incomplete fix. | `mc-midi` checks every header chunk midly would read, with midly's own lookup, before calling it. An SMPTE first header is refused as unsupported (as before); a later bad header is malformed. `catch_unwind` stays as a second guard. A regression test asserts that no panic happens at all; it fails without the fix. After the fix: 300 s, 338 396 inputs, 0 crashes. |
| `portability (ubuntu-24.04)` | **Inferred, not proven by a log.** cpal (`mc-audio`, since Phase 4) needs `libasound2-dev` on Linux, and the runner image lacks it. The job failed at build time (30 s). The last green weekly run (2026-11-02) predates `mc-audio`. | cpal's README; the ubuntu-24.04 image package list | The job installs `libasound2-dev` on Linux |
| `public-api` (FF-22) | **Not established.** The snapshots match the code exactly (0 differing lines for all 9 crates), both at `c789862` and on `main`, with `nightly-2026-10-31`. The job uses an unpinned nightly, which can change the rustdoc JSON under the pinned cargo-public-api 0.52.0; that is the likely cause. | Local runs | The job now stops when cargo-public-api itself fails, and it prints the full diff instead of the file names, so the next failure can be read. The nightly is **not** pinned (see below). |

No check was loosened: the same commands and thresholds run. The fixes are confirmed only by the next weekly run on Linux. It can be started by hand on the branch from the Actions tab ("weekly", workflow_dispatch).

**The nightly stays unpinned (user decision, 2026-11-11), and `weekly-ci-fixes` is merged into `main`.** The background: FF-20 (`xtask/src/fitness/inventory.rs`, a protected path) checks that the weekly workflow contains the literal commands `cargo +nightly public-api` and `cargo +nightly fuzz run import_smf`. Pinning would change those commands, so FF-20 must change with them, in the same commit, with an ADR. A first attempt pinned the nightly without that and failed FF-20. It was reverted before any merge; the check was not edited.
