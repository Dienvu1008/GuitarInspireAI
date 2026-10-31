# ADR-020 Invariants vs diagnostics model

Status: Accepted (user confirmed Q-09 on 2026-10-31)
Date: 2026-10-31
Related documents: [domain-invariants.md](../domain-invariants.md), [score-model-and-undo.md](../score-model-and-undo.md), [audio-dsp-architecture.md](../audio-dsp-architecture.md)

## Context
Treating every musical rule as a hard invariant would block realistic editing and imports. Treating none as hard would allow corrupt models.

## Decision
Four categories: hard invariants (always true), editing constraints (command preconditions), configurable constraints (parameters with defaults), diagnostics (reported, never block save). Temporary editing states stay in Dart.

- Underfull and overfull voices are diagnostics (D-01) and never block saving.
- Playback uses the effective master bar duration `E(m) = max(nominal(m), longest voice across all tracks and voices)`, including muted tracks. All tracks share bar start times, so later bars stay globally synchronized (domain-invariants section 4a, tests PB-01 to PB-09).
- One note per string per beat per voice is hard (H-05); cross-voice same-string use is a diagnostic.
- MIDI range is a playback concern.
- Master bar alignment is a hard v1 model limit, not a universal rule.

## Alternatives
Strict bar fill with automatic rebarring (TuxGuitar style); forbid overfull but allow underfull; per-track bar timing (tracks drift apart after an overfull bar).

## Trade-offs
Realistic editing and lossless imports; playback, layout and exporters must handle non-conforming bars. Global synchronization means an overfull bar in one track delays all others, which matches how the score is written in a shared master bar structure.

## Consequences
Exporters to strict formats (MusicXML) report and may split or truncate with a report. Diagnostics are shown in the inspector. `Score::effective_bar_duration` is a Phase 1 `mc-score` function; `mc-playback` consumes it in Phase 4.

## Related fitness functions
FF-13, FF-24.

## Reconsideration conditions
Exporters show unacceptable complexity handling overfull bars; polymeter support (major format version).
