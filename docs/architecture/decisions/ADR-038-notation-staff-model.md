# ADR-038 Staff model for standard notation and TAB (Phase 6c; amends ADR-031)

Status: Accepted (user, 2026-11-12), from the S11 proposal (spike README section 7.8, item 1).
Date: 2026-11-12
Related documents: [ADR-031](ADR-031-multi-track-tab-display.md) (amended), [ADR-018](ADR-018-notation-layout-in-rust.md), [ADR-039](ADR-039-pitch-capo-and-written-transposition.md), [ADR-040](ADR-040-theory-notation-rendering-boundary.md), [ADR-041](ADR-041-notation-engine-metrics-cache-pdf.md), `spikes/s11-notation/README.md` (sections 4.3 and 7)

## Context

ADR-031 gives a system one TAB stave per shown track, and all staves share bar x positions and onset columns. It names standard notation as a reason to revisit the stave model: a notation staff has another kind of content, density and vertical extent.

In S11, one bar plan per bar, for every staff of every track, gave exact alignment between notation and TAB (N2: 0.0000 difference). A notation box with fixed margins clipped content. The box had to follow the content of the system's bars.

## Decision

1. **`StaffConfiguration`, per track, is a view setting.** It is not stored in the score and not part of undo, like visibility in ADR-031 decision 2.
   - It says which staves the track shows: notation, TAB, or both.
   - In 6c the written transposition is not chosen here; it comes from the instrument (ADR-039). The only notation mode is sounding pitch (ADR-039 decision 4).
2. **`Staff`** is one laid-out staff of one track in one system. Its kind is `Notation` or `Tab`. It holds the painted items in staff-local coordinates, plus the facts the checks and hit testing need: notes, stems, beams, rests and tuplets.
3. **`StaffPosition`** places a staff in its system:
   - the track and the staff's index within the track;
   - the top and height of its box;
   - the top of its first line inside the box.
4. **One bar plan per bar, shared by every staff of every shown track.** Bar x, widths and onset columns come from it, so notation and TAB of one track, and staves of different tracks, line up exactly. This extends ADR-031 decision 1, column alignment by onset, to notation staves.
5. **Vertical extent from content.** Each bar plan records how far its notation reaches above the top line and below the bottom line, from the same rules the emission uses. A system's notation box is the reach of its bars plus a pad, with a minimum. The clef counts. The TAB box keeps its fixed margins, with 1.0 staff space above the first string.
6. **Order:** within a track, notation above TAB. Tracks are in score order (ADR-031).
7. **Editing stays on the TAB staff in 6c.** The notation staff is read-only. Its hits select a beat (ADR-041 decision 4), and the TAB staff edits it.
8. **ADR-031 compatibility.** With notation off, the layout is today's TAB layout. ADR-031 M1 and the existing goldens stay as they are.

## Alternatives

- **A separate layout per staff kind, aligned afterwards.** Rejected: alignment would be a second decision, outside the bar plan (the same reason as in ADR-031).
- **Fixed notation margins.** Rejected: S11 clipped a stem with 4.0 staff spaces.
- **Store the staff configuration in the score.** Rejected for now: it has no musical meaning and would change the format (GR-08).

## Trade-offs

- Content-dependent box heights make system heights differ, so the y of a system depends on the systems before it. Bar plans are cached (ADR-041), so computing every system's height costs no emission.
- The bar plan reserves room for every staff's content, so a column is as wide as its widest need, on any staff.

## Consequences

- `mc-notation` gains the staff types, and the score layout API takes the staff configuration of each track.
- The DTO gains the staff kind and position (FF-22: additions only).
- Dart keeps the per-staff list items of ADR-031 step A.

## Related fitness functions

FF-03, FF-04, FF-12, FF-21, FF-22. The S11 checks become `mc-notation` tests (ADR-041 decision 6).

## Reconsideration conditions

- Grand staff or other instruments (piano, bass). The model is chosen so that they add staff kinds and configurations, not a new model; their own ADRs decide that.
- Editing on the notation staff.
