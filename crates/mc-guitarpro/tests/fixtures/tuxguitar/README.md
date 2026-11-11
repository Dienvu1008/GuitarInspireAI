# Guitar Pro fixtures written by TuxGuitar 2.0.1

Purpose: GP3, GP4 and GP5 test files for `mc-guitarpro` (ADR-036, criterion G1). Each covers one feature group.
License: CC0-1.0. The files contain no third-party music. They were generated for this project on 2026-11-11.

## Provenance

- Copied unchanged from the S10 spike (`spikes/s10-guitar-pro/fixtures/tuxguitar/`). That folder's README records how they were made: TuxGuitar 2.0.1's own `GP3/GP4/GP5OutputStream` writers, called through its public API by `spikes/s10-guitar-pro/generator/MakeFixtures.java`, with the user's consent.
- 54 files: f01 to f18, each as `.gp3`, `.gp4` and `.gp5`. The content of each is listed in the spike README, section 4.
- They were written by TuxGuitar, not by Guitar Pro. Guitar Pro's own files may differ (ADR-036, trade-offs).
- Treat them as append-only. A changed file would invalidate the reviewed golden `../../golden/import.txt`.

## Known losses by TuxGuitar's writers (not importer faults)

- GP4 drops heavy accents.
- GP3 drops accent, palm mute, staccato, endings, tremolo picking, pick strokes and chord diagrams. f15.gp3 is byte-identical to f03.gp3.
- GP3 and GP4 merge the second voice into the first.
- GP3 has no lyrics section.
