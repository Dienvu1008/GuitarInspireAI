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
| M4 Playback | Phase 4 | Play the song with tempo change and mute; import a MIDI file with report; export MIDI | A1 to A7 in product build; MIDI fixtures |
| M5 MVP | Phase 5 | Tune a guitar, find a chord, insert it, play it back, save | MVP checklist; known limitations list |
| M6 and later | Phases 6 to 11 | Defined at each phase entry | Per phase |

Recommended first implementation milestone: M0, with S1, S2, S4 and S5 first because they carry the highest architectural risk.

Status: M0 reached on 2026-10-31 on the reference machine, with GitHub Actions not yet run. Evidence and recommendation: [m0-exit-report.md](m0-exit-report.md). Next milestone: M1.
