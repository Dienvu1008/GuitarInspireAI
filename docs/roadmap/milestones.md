# Milestones

Purpose: Define verifiable milestones with demo scenarios and evidence.
Status: Proposed.
Related documents: [implementation-phases.md](implementation-phases.md), [roadmap/README.md](README.md), [dependency-map.md](dependency-map.md).

| Milestone | End of | Demo scenario | Evidence |
|---|---|---|---|
| M0 Foundation | Phase 0 | CI rejects a forbidden dependency and an untrailered protected-path change; spike demos (frb stream, glitch-free playback, 500-bar scroll, atomic save) | Spike notes S1, S2, S4, S5, S7; updated ADR statuses |
| M1 Core domain via CLI | Phase 1 | CLI builds a song, edits, undoes, saves, reloads; large reference score round trip | FF-11, FF-13, FF-14, FF-32 green; benchmark baselines |
| M2 Guitar core | Phase 2 | CLI lists voicings for chords in standard and drop D | Golden set passes |
| M3 Editable TAB desktop app | Phase 3 | Create a 2-track song in the app, save, kill the process, recover, reopen | Integration tests; acceptance checklist |
| M4 Playback | Phase 4 | Play the song with tempo change and mute; import a MIDI file with report; export MIDI | A1 to A7 in product build and FF-18, measured on the reference machine only (R-21 open, [ADR-025](../architecture/decisions/ADR-025-single-machine-evidence-accepted.md)); MIDI fixtures |
| M5 MVP | Phase 5 | Tune a guitar, find a chord, insert it, play it back, save | MVP checklist; known limitations list |
| M6 and later | Phases 6 to 11 | Defined at each phase entry | Per phase |

Recommended first implementation milestone: M0, with S1, S2, S4 and S5 first because they carry the highest architectural risk.

Status: M0 accepted on 2026-10-31. First CI run (`6dcb8bc`) succeeded; the workflow update in the baseline commit is validated on its push. Evidence: [m0-exit-report.md](m0-exit-report.md).

M1 reached and Phase 1 accepted on 2026-10-31 (Q-08 decided: `.giproj`; merged from `phase-1-core`). Evidence: [phase-1-exit-report.md](phase-1-exit-report.md).

M2 reached and Phase 2 accepted on 2026-10-31 after the closure decisions on ranking, display spelling and minimum pitch content (golden set of 235 cases passes; `mc-cli chord` in standard and drop D; merged from `phase-2-guitar`). Evidence: [phase-2-exit-report.md](phase-2-exit-report.md). Next milestone: M3 (Phase 3).

M3 demo runs end to end: the Windows integration tests build the app and, against the real core, create a 2-track song, enter notes by keyboard, undo and redo, save, reopen identically and recover after a lost session; a real process kill is in the manual script. Edit feedback is within budget on the typical score on the development machine. M3 reached and Phase 3 accepted on 2026-10-31 (open points decided as proposed; merged from `phase-3-editor`). Evidence: [phase-3-exit-report.md](phase-3-exit-report.md). Next milestone: M4 (Phase 4).

M4 demo: the app plays the song with a tempo factor and per-track mute, and the File menu imports and exports MIDI with a report. Playback (A1 to A7, A4 via the FF-18 glitch run with zero underruns in 10 minutes) and the bundled trimmed SoundFont are evidence for the reference machine only; R-21 stays open ([ADR-025](../architecture/decisions/ADR-025-single-machine-evidence-accepted.md)). MIDI round trip preserves pitch and quantized rhythm. M4 reached and Phase 4 accepted on 2026-10-31 (single-machine evidence, R-21 open; open points decided as proposed; merged from `phase-4-playback` with `--no-ff`). Evidence: [phase-4-exit-report.md](phase-4-exit-report.md). Next milestone: M5 (Phase 5).

M5 (MVP) demo: tune a guitar with the microphone tuner (referencing the active track's tuning and capo), find a chord and insert a voicing at the cursor, play it back, and save. Tuner accuracy and latency (A8 to A10) are measured on synthetic signals on the reference machine only; the real-guitar set is not yet recorded, so [ADR-029](../architecture/decisions/ADR-029-tuner-pitch-detection.md) stays Draft and the MPM choice is provisional (single-machine, R-21 open, [ADR-025](../architecture/decisions/ADR-025-single-machine-evidence-accepted.md)). M5 reached and accepted on 2026-10-31 with the evidence gaps (a) and (b) on the record as conditions of acceptance (merged from `phase-5-mvp`). Evidence: [phase-5-exit-report.md](phase-5-exit-report.md), [mvp-known-limitations.md](mvp-known-limitations.md), [mvp-release-notes.md](mvp-release-notes.md). Next milestone: M6 and later (Phase 6, editor depth).

Phase 6b (no milestone of its own; part of M6 and later) was accepted on 2026-11-09.
- It delivered the beat clipboard and close prompt, a multi-track TAB display, the Mixer panel and MusicXML export. All evidence comes from a single machine.
- Five evidence gaps are conditions of acceptance: edit latency in an intermittent slow state; the FF-18 glitch baseline no longer reproducing; one tie-into-repeat note that MuseScore plays differently; MuseScore ignoring the capo; and a possible M3 test flake.
- It was merged from `phase-6-editor-depth`.
- Evidence: [phase-6b-exit-report.md](phase-6b-exit-report.md). Next: Phase 6a (pattern engine).

Phase 6a (no milestone of its own; part of M6 and later) was accepted on 2026-11-11.
- It delivered playing techniques in the score (accent, palm mute, string-percussive, stroke direction; format 1.1) and the pattern engine: patterns bound to a chord voicing and applied over bars, the text syntax with the user-confirmed corpus, and the pattern editor with gestures, saved with the project (format 1.2). All evidence comes from a single machine.
- Five evidence gaps are conditions of acceptance: MuseScore losing three techniques on re-export (TQ-8); no dragging in the grid editor (PE-14); PE-15 measured once; the older-reader check done by hand; and the editor's command queue covered only by the Windows integration test.
- It was merged from `phase-6a-pattern-engine`.
- Evidence: [phase-6a-exit-report.md](phase-6a-exit-report.md). Next: Phase 6c or 6d, in the order the user chooses (entry gates S11 and S10).

Phase 6d (no milestone of its own; part of M6 and later) was accepted on 2026-11-11.
- It delivered Guitar Pro GP3, GP4 and GP5 (5.00) import with a report, and export to the same three versions (added by the user), with an own clean-room adapter `mc-guitarpro` (ADR-036). All evidence comes from a single machine and from files written by TuxGuitar 2.0.1.
- Five evidence gaps are conditions of acceptance: the fuzz target not yet run; no file written by Guitar Pro itself; TuxGuitar's palm mute and let ring limit; GP5 5.10, GPX and GP7+ refused; and no legal review.
- It was merged from `phase-6d-guitar-pro`.
- Evidence: [phase-6d-exit-report.md](phase-6d-exit-report.md). Next: Phase 6c (standard notation and PDF, entry gate S11).