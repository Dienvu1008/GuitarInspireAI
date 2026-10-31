# ADR-015 Dependency license policy

Status: Accepted
Date: 2026-10-31
Related documents: [technology-evaluation.md](../../research/technology-evaluation.md), [security-and-privacy.md](../security-and-privacy.md)

## Context
The project license is undecided (user decision). The code base must remain distributable under a permissive open-source license or commercially, including app stores.

## Decision
Allow-list: MIT, Apache-2.0, BSD-1/2/3-Clause, ISC, Zlib, Unlicense, Unicode-3.0, CC0-1.0. Any other license (GPL, LGPL, MPL, AGPL, custom, data licenses for SoundFonts and models) requires an ADR evaluating usage category (learn, implement format, link, copy), isolation options and distribution impact. Copying code from copyleft projects is prohibited. Bundled data (SoundFonts, models) follows the same policy. The project license itself is not chosen by this ADR.

## Alternatives
Allow LGPL with dynamic linking; decide project license first.

## Trade-offs
Keeps all license options open; excludes some mature libraries (FluidSynth, Verovio, JUCE) unless isolated.

## Consequences
cargo-deny license checks in CI from Phase 0; S7 audit before Phase 1 dependencies are added.

## Related fitness functions
FF-09, FF-10.

## Reconsideration conditions
The user selects a project license (the allow-list may widen, for example GPL-compatible if the project becomes GPL).
