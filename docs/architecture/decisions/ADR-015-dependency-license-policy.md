# ADR-015 Dependency license policy

Status: Accepted
Date: 2026-10-31
Related documents: [technology-evaluation.md](../../research/technology-evaluation.md), [security-and-privacy.md](../security-and-privacy.md)

## Context
The project license is undecided (user decision). The code base must remain distributable under a permissive open-source license or commercially, including app stores.

## Decision
Allow-list: MIT, Apache-2.0, BSD-1/2/3-Clause, ISC, Zlib, Unlicense, Unicode-3.0, CC0-1.0. Any other license (GPL, LGPL, MPL, AGPL, custom, data licenses for SoundFonts and models) requires an ADR evaluating usage category (learn, implement format, link, copy), isolation options and distribution impact. Copying code from copyleft projects is prohibited. Bundled data (SoundFonts, models) follows the same policy. The project license itself is not chosen by this ADR.

## S7 audit result (2026-10-31)

- The allow-list is unchanged. Every recommended Phase 1 to 5 dependency tree measured in M0 passes (see [technical-spikes.md](../../research/technical-spikes.md) S7). Non-listed licenses appear only as `OR` alternatives that resolve to listed ones.
- `deny.toml` now sets `allow-wildcard-paths = true` so internal path dependencies of unpublished crates pass the wildcard ban.
- Decided (user, 2026-10-31): MPL-2.0 is not added to the allow-list. cpal's `realtime` feature stays disabled, and thread priority uses the own `windows-sys` call (ADR-007). The license check in CI rejects the MPL crate if the feature is enabled. Evaluation that informed the decision, which is technical and not legal advice:
  - Usage category: (c) linking an unmodified library.
  - MPL-2.0 is file-level copyleft. Distributing a binary requires making the source of the MPL-covered files available (for an unmodified crate this can point to the published source) and keeping its notices. It does not extend to our own files, and it allows combination with proprietary or permissively licensed code.
  - Alternative measured as technically equivalent in S2: calling `AvSetMmThreadCharacteristicsW` directly through `windows-sys` (MIT OR Apache-2.0), about 20 lines on Windows. Other platforms would need their own small implementations later.
- Decided (user, 2026-10-31): the default bundled SoundFont is FluidR3 GM (MIT), chosen for clear distribution rights over small sound-quality differences. Conditions, applied when it is first bundled in Phase 4:
  - Read and store the original license text (COPYING) from the source distribution; M0 verified the MIT statement only through secondary sources.
  - Record provenance in the repository: source URL, version or date, file name, size and SHA-256.
  - Include its license and attribution in the release third-party notices.
  - User-supplied SoundFonts remain a future capability; they are not bundled and carry no distribution obligation for the project.

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
