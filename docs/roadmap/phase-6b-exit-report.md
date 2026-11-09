# Phase 6b Exit Report (editor depth: debt, multi-track display, mixer, MusicXML export)

Purpose: Evidence for Phase 6b (the first part of Phase 6, "multi-track polish and MusicXML export"), the evidence gaps accepted at closure, and what comes next.
Status: Final. Phase 6b accepted by the user on 2026-11-09, with the evidence gaps below as conditions of acceptance. **Read "Known evidence gaps" before relying on this report.**
Related documents: [implementation-phases.md](implementation-phases.md), [milestones.md](milestones.md), [mvp-known-limitations.md](mvp-known-limitations.md), [mvp-release-notes.md](mvp-release-notes.md), [phase-5-exit-report.md](phase-5-exit-report.md), [ADR-030](../architecture/decisions/ADR-030-editor-debt-clipboard-and-session-marker.md), [ADR-031](../architecture/decisions/ADR-031-multi-track-tab-display.md), [ADR-032](../architecture/decisions/ADR-032-track-mixer-volume-pan-solo.md), [ADR-033](../architecture/decisions/ADR-033-musicxml-export-adapter.md).

## 1. Outcome

Phase 6 was split by user decision into 6b, then 6a (pattern engine), then 6c and 6d after the spikes S11 and S10. 6b is done. Its four slices each had an ADR whose acceptance criteria were fixed before any code. Thresholds were never lowered to match a result.

| Slice | What the user gets | ADR | Result |
|---|---|---|---|
| 6b-1 | Copy, cut and paste a run of beats (in-app clipboard). A "save changes?" prompt on close, New, Open and MIDI import. Crash recovery that offers only autosaves newer than their source, with Restore, Open original and Discard. | ADR-030 | Met |
| 6b-2 | Several TAB tracks shown at once, aligned by onset; hide and show per track | ADR-031 | Met, with performance gap (a) |
| 6b-3 | Mixer panel: mute (now on screen), solo, volume and pan per track | ADR-032 | Met, with gap (b) |
| 6b-4 | File > Export MusicXML... (MusicXML 4.0, TAB staves) | ADR-033 | Met, with gaps (c) and (d) |

Details, numbers and per-criterion evidence are in each ADR's results section. This report does not repeat them.

## 2. Slices and commits

All on `phase-6-editor-depth`, 26 commits after `main` at `2fe101f`. Every commit that touches a protected path carries its `Architecture-Change` trailer (FF-19 ok).

- **6b-1** (ADR-030): `155d848` and `7689bd8` (ADR), `89b230f`, `fa59781`, `bfe2b23`, `dd807e9`, `fbc3336`.
- **6b-2** (ADR-031): `dd4c510` (ADR), `cedc3c0`, `861a185`, `a1a59ef`, `21d1877`, `12f869e`.
- **6b-3** (ADR-032): `64507de` (ADR), `b75eade`, `bc8cda4`, `d5d458a`, `9b7c337`, `71b9664`.
- **6b-4** (ADR-033): `691da3a` and `7432530` (ADR), `27fe2a3`, `c23df07`, `6a09ca1`, `9a7cdb1`, `2cfddbf`.

## Known evidence gaps

These are conditions of acceptance (user decision 2026-11-09), not debt found later. All evidence comes from one machine (ADR-025, R-21 open).

- **(a) Edit latency on large scores, ADR-031 P2 and P3.**
  - On the large test score (20 tracks of 300 bars), a note edit took about 15 to 19 ms in 18 of 23 controlled runs, against a 33 ms budget.
  - The other 5 runs took 40 to 48 ms. They happened at every track count, including 4, and also with code from before 6b-2.
  - The cause of this intermittent slow state is not found. P2 and P3 are met in the fast state only. Typical scores were within budget in every run of the last session.
- **(b) Glitch-free playback not re-confirmed, ADR-032 X12 (FF-18).**
  - The 10-minute device run now has a few late callbacks (5 on a quiet machine after a restart).
  - In an interleaved A/B, the build from before the mixer had 6 late callbacks in 6 minutes; the mixer build had 2.
  - No regression from the mixer is seen, but the Phase 4 baseline (0 late, maximum callback 9.69 ms in debug) no longer reproduces, and the cause is unknown. Record: `benchmarks/ff18-glitch.md`.
- **(c) MusicXML playback in MuseScore differs for one note, ADR-033 E8.**
  - Every note matches MuseScore's MIDI in pitch and onset, except a tie from a pickup into a repeated section: our playback plays that note again on the second pass, and MuseScore does not. MusicXML cannot say "tied on the first pass only".
  - Lengths differ for let-ring and dead notes, because each program renders them differently in MIDI. MuseScore also releases every note one tick early.
  - None of these is content lost in the file.
- **(d) MuseScore ignores the capo, ADR-033 E9.**
  - String and fret survive MuseScore's re-export for tracks without a capo (9 600 of 9 600 notes in the typical score). A track with a capo is re-fingered by MuseScore, which ignores `<capo>` (an open MuseScore issue).
  - Our encoding follows the MusicXML 4.0 definition.
  - Only MuseScore 4.7.5 was checked. Finale, Dorico and Guitar Pro were not, and layout quality was not judged.
- **(e) A possible flake in the M3 integration test.** In one of three local runs of the Windows integration tests, the first test (M3) timed out waiting for its first key press. That was the first run after a rebuild; the other runs passed 4 of 4. It was not investigated further.

Carried over from Phase 5 and unchanged: the tuner has not been measured on a real guitar, and ADR-029 stays Draft (`spikes/s3-tuner/results/README.md`).

## 3. Defects found and corrected in 6b

- **Per-track mute was never on screen in the MVP.** The core, the controller and the `MuteButton` widget existed, but no screen used them. The Phase 4 exit report and the MVP release notes said it was met. Both now carry a correction, and mute is in the Mixer panel (6b-3).
- **Two vacuous property tests were caught and fixed.**
  - An ADR-030 coverage check (6b-1a).
  - The ADR-033 random scores never produced a tie across a bar line until a coverage test showed it (6b-4a).
- **The 16-track cap** applied in 6b-2d was removed after a controlled re-measurement showed it was not needed (6b-2e).
- **The draft of ADR-033 missed the MusicXML let-ring form** (`<tied type="let-ring"/>`). The user pointed it out before any code was written.

## 4. Fitness functions and governance

- New crate `mc-musicxml` (adapter, phase 6). `allowed-deps.toml` and the dependency-rules matrix gained its row and the edges from `mc-ffi` and `mc-cli` (ADR-033).
- New dev dependency `roxmltree =0.21.2` (MIT OR Apache-2.0, test-only).
- No new runtime dependency outside the allow-list, and no new `unsafe` (FF-34: 14 crates).
- New CI job `musicxml-schema`. It validates exports against the MusicXML 4.0 XSD, fetched at a pinned commit with SHA-256 checks and not stored in the repository, and runs a negative control.
- Public API snapshots changed by additions only (FF-22; lines added / removed since `2fe101f`): `mc-app` 108/0, `mc-notation` 120/0, `mc-playback` 85/0, `mc-score` 61/0, `mc-theory` 2/0.
- Bindings were regenerated by codegen only, and a second run is identical (FF-12).
- No fitness function, lint or rule was weakened.

## 5. CI status

Every slice commit passed GitHub Actions: governance, cargo-deny, bridge, and from `c23df07` also musicxml-schema. The merge commit is checked again after it is pushed.

## 6. Closure

Decisions (user, 2026-11-09):
- 6b-3 and 6b-4 are closed with gaps (b), (c) and (d) on the record.
- Phase 6b is accepted.
- `phase-6-editor-depth` is merged into `main` with `--no-ff`. The merge commit carries the trailers ADR-030, ADR-031, ADR-032 and ADR-033.

Next: Phase 6a, the pattern engine. Its entry needs the user's answers to the open questions Q-01 to Q-06.
