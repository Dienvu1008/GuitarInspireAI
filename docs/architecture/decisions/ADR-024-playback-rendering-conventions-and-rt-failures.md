# ADR-024 Playback rendering conventions and real-time failure handling
Status: Accepted (2026-10-31, user decisions Q4, Q5, Q8 and Q9 at Phase 4 entry)
Date: 2026-10-31
Related documents: [ADR-007](ADR-007-audio-thread-isolation.md), [ADR-011](ADR-011-rational-time-model.md), [audio-dsp-architecture.md](../audio-dsp-architecture.md) sections 2, 3 and 3a, [domain-invariants.md](../domain-invariants.md) section 4a

## Context
ADR-007 decides the thread isolation of the audio engine and records the S2 amendments. It does not fix the values the timeline compiler uses for notes the score does not describe (velocity, dead notes, channels), what happens to sounding notes when a timeline is swapped during playback, or what the callback does on a panic. Golden timeline tests need these values to be fixed. This ADR adds them without changing ADR-007.

## Decision

### Timeline values (`mc-playback`, deterministic, used by golden tests)
1. Velocity: every note on uses velocity 96 (`Velocity::NORMAL`). The score has no dynamics in format 1.0.
2. Dead notes (`NoteEffects::dead`): note on at the beat onset with velocity 40 (`Velocity::DEAD`), note off after exactly 1/64 of a whole note, or at the end of the beat if the beat is shorter. A dead note does not take part in tie chains (it neither extends nor is extended) and ignores let ring. Its pitch is the sounding pitch of its fret.
3. Let ring: as in [audio-dsp-architecture.md](../audio-dsp-architecture.md) section 3a, rule 4: the note off moves to the next onset on the same string in the same track, or to the end of the played bar, whichever comes first. For a tie chain, the let ring flag of the first note of the chain applies, measured from the end of the chain.
4. Retrigger: if a note on hits a key that is already sounding on the same channel (unison on two strings, or let ring), the earlier note gets a note off at that tick. With the same-tick order of section 3a rule 5 (note off before note on), every note on has exactly one later or equal note off on the same channel and key.
5. Channels and program (Q4): tracks get MIDI channels in score order from the table 0 to 8, 10 to 15 (channel index 9, MIDI channel 10, is never used). Tracks beyond the 15th reuse the table cyclically, with a `PlaybackWarning`. Mute does not change the assignment. Every channel gets one program change at tick 0 to General MIDI program 25 (zero-based; "Acoustic Guitar (steel)", GM number 26). The program is not stored in the project.

### Real-time behaviour (`mc-audio`)
6. Timeline swap (Q8): when the callback swaps in a new timeline during playback, it first sends all-notes-off to the synth, then relocates the cursor in the new timeline by binary search on the current tick. Notes that started before the swap are not carried over. This is the S2 behaviour; carrying held notes is reconsidered only if the cut is audible in practice.
7. Panics (Q9): the callback body runs inside `std::panic::catch_unwind`. On a panic the callback fills the output with silence for that and every later callback, sets an error flag (atomic) and stops reading the timeline; the control thread reports the error and can rebuild the engine. The workspace keeps `panic = "unwind"` (no profile may set `panic = "abort"` for crates linked into the app, or this protection is lost). A test injects a panic into the callback body and checks the silence and the flag.

## Alternatives
- Velocity from note position or accents: no data for it in the model.
- Dead notes omitted: loses the rhythmic click that players expect.
- Carrying held notes across a swap: needs voice tracking across timelines; S2 showed the simple cut is clean.
- Abort on panic: one bug in a rarely used path would end the app with no recovery.

## Trade-offs
Fixed velocity and program sound uniform; that is acceptable for an MVP practice player. A swap during a sustained note cuts it.

## Consequences
`audio-dsp-architecture.md` section 3a references these values. Changing any of them changes golden timelines and needs a new decision.

## Related fitness functions
FF-16, FF-17 (callback body, including the panic path), FF-18; PB-09 and the note-pairing property test in `mc-playback`.

## Reconsideration conditions
The format gains dynamics or per-track instruments; users report audible cuts on edits during playback.
