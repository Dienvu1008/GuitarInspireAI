# ADR-011 Rational time model

Status: Accepted
Date: 2026-10-31
Related documents: [domain-invariants.md](../domain-invariants.md), [bounded-contexts.md](../bounded-contexts.md)

## Context
Durations with dots and tuplets (for example triplets, quintuplets) are not exactly representable in floats or a fixed tick resolution.

## Decision
Musical positions and durations are rationals (fractions of a whole note) in the domain. Ticks are derived only for playback timelines, with a resolution chosen per compile. Seconds are derived through the tempo map.

## Alternatives
Floating-point beats; fixed PPQ ticks (for example 960) as domain time.

## Trade-offs
Exact arithmetic and comparisons; slightly more complex math and overflow considerations (use i64 numerator and denominator with normalization; overflow returns errors).

## Consequences
No floats in persisted music data (rationals as pairs). MIDI import quantizes to rationals and reports approximations.

## Related fitness functions
FF-13 (property tests of arithmetic), FF-32.

## Reconsideration conditions
Free-time or audio-aligned tracks require a seconds-based axis (added alongside, not replacing).
