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
