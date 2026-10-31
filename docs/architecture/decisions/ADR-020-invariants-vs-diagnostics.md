# ADR-020 Invariants vs diagnostics model

Status: Accepted (four categories). Proposed (overfull bars as diagnostic, pending user confirmation).
Date: 2026-10-31
Related documents: [domain-invariants.md](../domain-invariants.md), [score-model-and-undo.md](../score-model-and-undo.md)

## Context
Treating every musical rule as a hard invariant would block realistic editing and imports. Treating none as hard would allow corrupt models.

## Decision
Four categories: hard invariants (always true), editing constraints (command preconditions), configurable constraints (parameters with defaults), diagnostics (reported, never block save). Temporary editing states stay in Dart. Underfull and overfull voices are diagnostics (D-01), with playback using max(nominal, longest voice). One note per string per beat per voice is hard (H-05); cross-voice same-string use is a diagnostic. MIDI range is a playback concern. Master bar alignment is a hard v1 model limit, not a universal rule.

## Alternatives
Strict bar fill with automatic rebarring (TuxGuitar style); forbid overfull but allow underfull.

## Trade-offs
Realistic editing and lossless imports; playback, layout and exporters must handle non-conforming bars.

## Consequences
Exporters to strict formats (MusicXML) report and may split or truncate with a report. Diagnostics are shown in the inspector.

## Related fitness functions
FF-13, FF-24.

## Reconsideration conditions
User prefers strict fill; exporters show unacceptable complexity handling overfull bars.
