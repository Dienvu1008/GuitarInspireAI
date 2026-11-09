# Open Questions

Purpose: Track questions that need a user decision or more information, with the documents and phases they affect.
Status: Living document.
Related documents: [guitar-pattern-engine.md](../architecture/guitar-pattern-engine.md), [domain-invariants.md](../architecture/domain-invariants.md), [project-format.md](../architecture/project-format.md), [technical-spikes.md](technical-spikes.md), [architecture/README.md](../architecture/README.md).

| Id | Question | Needed by | Owner | Affects |
|---|---|---|---|---|
| Q-01 | Decided 2026-11-09: keywords are case-insensitive; the alternate bass is its own keyword (`Alt`); note names will need an explicit syntax (ADR-034) | Done | User | guitar-pattern-engine |
| Q-02 | Decided 2026-11-09: numbers are absolute strings, 1 = highest; a string above the instrument's count is an error, never skipped; chord-relative selection would be a separate selector (ADR-034) | Done | User | guitar-pattern-engine |
| Q-03 | Decided 2026-11-09 (changed from the proposal): no default duration. Each step has its own onset and duration in rationals; nothing is inferred from the step count, divided equally or stretched; patterns may span several bars; a length that does not fit the target is reported or asked about (ADR-034) | Done | User | guitar-pattern-engine |
| Q-04 | Decided 2026-11-09: `Treble` = the 3 highest sounded strings, struck together; `Treble(n)` for n; fewer if the voicing has fewer; no notes are invented (ADR-034) | Done | User | guitar-pattern-engine |
| Q-05 | Decided 2026-11-09: fingerstyle has no direction by default; strums default to down; `D` and `U`; the direction is stored in the IR and shown on TAB (ADR-034, ADR-035) | Done | User | guitar-pattern-engine |
| Q-06 | Decided 2026-11-09: `-` rest, `_` hold (no new attack, optionally only some strings), `>` accent (playback and TAB), `+` simultaneous; techniques are model techniques, and player names are user gestures. The example corpus is proposed by the agent and awaits the user's review ([pattern-corpus-proposed.md](../architecture/pattern-corpus-proposed.md)) | Corpus open | User | guitar-pattern-engine |
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
