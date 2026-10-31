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
| Q-08 | Resolved: `.giproj` (GuitarInspire Project) is the product extension, defined in adapters only; file identity comes from the manifest format and version (project-format section 2a). Registered media type: still open, not needed before distribution | Done | User | project-format |
| Q-09 | Resolved: underfull and overfull bars are diagnostics and never block saving; playback uses the effective master bar duration across all tracks and voices (domain-invariants section 4a) | Done | User | domain-invariants, audio-dsp-architecture |
| Q-10 | Project license (permissive open source, copyleft, commercial) | Before first public release; affects ADR-015 allow-list | User | ADR-015 |
| Q-11 | Resolved: FluidR3 GM (MIT) is the default bundled SoundFont, with provenance and license recorded when bundled (ADR-015); user-supplied SoundFonts are a future capability | Done | User | audio |
| Q-12 | Resolved (Phase 3 entry, confirmed by the user 2026-10-31): Riverpod (`flutter_riverpod`) without code generation; fakes are injected with provider overrides | Done | Developer | cross-platform-ui |
| Q-13 | Resolved for Windows (user, 2026-10-31): Windows 10 and 11, x64 only; ARM64 after the MVP. macOS minimum (cpal README lists 14.2) is decided when macOS builds start | Done (Windows) | User | performance, distribution |
| Q-14 | Web priority: remain Phase 11 or earlier? | Phase 5 exit review | User | ADR-014 |
| Q-15 | Resolved (Phase 3 entry, confirmed by the user 2026-10-31): per note. Digits typed into the same note within the multi-digit timeout (600 ms) form one undo step; moving the cursor or the timeout ends the step | Done | Developer | score-model-and-undo |
| Q-16 | Large audio in projects: embedded or external referenced folder | Phase 8 entry | User | project-format |
| Q-17 | Resolved: one active TAB track at a time in MVP, with track management and switching; playback of all tracks simultaneously with per-track mute; simultaneous multi-track notation deferred to Phase 6b | Done | User | roadmap |
