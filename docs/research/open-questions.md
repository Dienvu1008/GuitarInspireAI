# Open Questions

Purpose: Track questions that need a user decision or more information, with the documents and phases they affect.
Status: Living document.
Related documents: [guitar-pattern-engine.md](../architecture/guitar-pattern-engine.md), [domain-invariants.md](../architecture/domain-invariants.md), [project-format.md](../architecture/project-format.md), [technical-spikes.md](technical-spikes.md), [architecture/README.md](../architecture/README.md).

| Id | Question | Needed by | Owner | Affects |
|---|---|---|---|---|
| Q-01 | Pattern DSL: are keywords case-sensitive (`Bass` vs `bass`)? | Phase 6a entry | User | guitar-pattern-engine |
| Q-02 | Pattern DSL: do numbers mean absolute strings or chord-relative tones, and which numbering convention? | Phase 6a entry | User | guitar-pattern-engine |
| Q-03 | Pattern DSL: default step duration (beat, fixed subdivision, bar divided equally)? | Phase 6a entry | User | guitar-pattern-engine |
| Q-04 | Pattern DSL: meaning of `Treble` (single note, pinch, strum; how many strings)? | Phase 6a entry | User | guitar-pattern-engine |
| Q-05 | Pattern DSL: default picking directions and syntax for direction | Phase 6a entry | User | guitar-pattern-engine |
| Q-06 | Pattern DSL: syntax for rests, ties, accents, simultaneous strikes; plus 5 to 10 real example patterns with expected TAB | Phase 6a entry | User | guitar-pattern-engine |
| Q-07 | Resolved: string 1 is the highest-pitched string; numbers are stable under retuning and never renumbered by pitch | Done | User | bounded-contexts, mc-theory |
| Q-08 | Native file extension (for example `.gia`, `.gisong`) and registered media type | Phase 1 exit | User | project-format |
| Q-09 | Resolved: underfull and overfull bars are diagnostics and never block saving; playback uses the effective master bar duration across all tracks and voices (domain-invariants section 4a) | Done | User | domain-invariants, audio-dsp-architecture |
| Q-10 | Project license (permissive open source, copyleft, commercial) | Before first public release; affects ADR-015 allow-list | User | ADR-015 |
| Q-11 | Default bundled SoundFont (size vs quality vs license) | Phase 0 (S7) | Developer, user approval | audio |
| Q-12 | State management library (Riverpod or flutter_bloc) | Phase 3 entry | Developer | cross-platform-ui |
| Q-13 | Minimum supported Windows version (cpal WASAPI requires Windows 10 [Verified: README]) and macOS version (cpal README lists 14.2) | Phase 3 | User | performance, distribution |
| Q-14 | Web priority: remain Phase 11 or earlier? | Phase 5 exit review | User | ADR-014 |
| Q-15 | Undo coalescing rules for typing (per digit or per note) | Phase 3 | Developer | score-model-and-undo |
| Q-16 | Large audio in projects: embedded or external referenced folder | Phase 8 entry | User | project-format |
| Q-17 | Resolved: one active TAB track at a time in MVP, with track management and switching; playback of all tracks simultaneously with per-track mute; simultaneous multi-track notation deferred to Phase 6b | Done | User | roadmap |
