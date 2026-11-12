# ADR-037 Bravura as the SMuFL music font, bundled unmodified under the SIL OFL 1.1 (Phase 6c)

Status: Accepted (user, 2026-11-11). The user chose Bravura, asked for the license audit and this exception, and approved this ADR and ADR-015 amendment 1 on 2026-11-11. The font is added to the repository in the 6c slice that bundles it, together with its checksum check (decision 6). Until then the S11 spike reads a local copy.
Date: 2026-11-11
Related documents: [ADR-015](ADR-015-dependency-license-policy.md) (amendment 1), [ADR-018](ADR-018-notation-layout-in-rust.md), [ADR-021](ADR-021-bundled-font-ofl.md), [technical-spikes.md](../../research/technical-spikes.md) (S11)

## Context

Phase 6c adds standard notation above the TAB, in the app and in PDF export (user decision 2026-11-11). Layout is computed in Rust from font metrics, and Flutter paints it (ADR-018). Standard notation needs a music font with the SMuFL glyph set and its metadata: glyph bounding boxes, stem and anchor points, and engraving defaults such as staff-line and stem thickness. Without the metadata the layout has to guess. The user chose Bravura, the SMuFL reference font.

ADR-021 already allows one font, Noto Sans Mono, under the SIL Open Font License 1.1, as data and not as code. Its reconsideration conditions name this case: "a different font is needed for standard notation (Phase 6c)". OFL-1.1 is not on the ADR-015 allow-list.

### License audit (2026-11-11, technical, not legal advice)

Release `bravura-1.482` of `github.com/steinbergmedia/bravura`, published 2026-08-24, the latest release on the audit date. The files were downloaded from the release assets; their sizes equal the sizes the release lists.

| File | Size (bytes) | SHA-256 |
|---|---|---|
| `Bravura.otf` | 889 228 | `CDF0F893EE1FDB64B7F6713D71EE0DCFC349C0AC01429A8E451B01A9E79F5F3B` |
| `Bravura.json` (SMuFL metadata) | 1 256 995 | `489FB4D734FC0B72A3CC601B50EBDF0BF8C57802AB9893E52CE030B86BF2CE11` |
| `OFL.txt` | 4 420 | `C24929BE7028026A65EE8894DA1E3C36D2A4CCCE0548D9BA8BA64509F46319EE` |
| `FONTLOG.txt` | 49 571 | `EAD28A670D1429AEBC3350C52BC13A479B5912726CD42BD1E9EDD367076CCD22` |
| `OFL-FAQ.txt` | 59 071 | `8ACD621722E71DC89DE0087A264497B8964D240EEDC53677F6344AB20CE36FD7` |

Findings:
- **License:** SIL Open Font License, Version 1.1 (26 February 2007), as stated in `OFL.txt`.
- **Copyright:** © 2015, Steinberg Media Technologies GmbH.
- **Reserved Font Name "Bravura".** A Modified Version may not use the name. The OFL FAQ (2.6) counts subsetting as modification. The file bundled with the app must therefore be the original, and it is not subset.
- **Embedding in documents (OFL FAQ 1.12 and 1.13):** allowed, in full or as a subset. The modification and redistribution restrictions do not apply to a font embedded in a document, and embedding does not change the license of the document. A PDF we export may therefore embed a subset of Bravura.
- **The metadata version matches:** `Bravura.json` says `fontName` "Bravura" and `fontVersion` 1.482.
- **Usage categories:**
  - (c) distribution of an unmodified data file with the application (the app bundle);
  - embedding in documents the user creates (PDF export).

## Decision

1. **Bravura 1.482 is the music font** for standard notation in the app and in PDF export, with its SMuFL metadata `Bravura.json`.
2. **Bundled unmodified.** `Bravura.otf` and `Bravura.json` are added with the original `OFL.txt` next to them, and a `PROVENANCE.md` that records the source URL, release, file names, sizes and SHA-256 above. The files keep their upstream names. They live with the app's bundled fonts, as ADR-021's font does.
3. **The release third-party notices** include the Bravura copyright and the OFL text.
4. **PDF export may embed a subset** of Bravura (OFL FAQ 1.12). The bundled file itself is never subset or altered.
5. **The font is not sold by itself** (OFL condition); bundling it with the application is permitted.
6. **A fitness check** compares the bundled files with the SHA-256 recorded in `PROVENANCE.md`, so a changed file fails CI. Added with the font, in the slice that bundles it.
7. **No automatic widening.** This exception covers these files only. Another font, or a new Bravura version, needs its own ADR or an amendment to this one, with a new audit (ADR-015 amendment 1).

## Alternatives

- **Leland (MuseScore's SMuFL font, OFL 1.1):** the same license, also a complete SMuFL font. Bravura is the reference font of the SMuFL specification, and the user chose it.
- **A system or text font with music glyphs:** no SMuFL metadata, and metrics that vary by machine (the problem ADR-021 solved for TAB).
- **Drawing the glyphs ourselves:** a large effort with lower quality, and it would still need the metadata the font gives.

## Trade-offs

- About 0.9 MB for the font and 1.3 MB for the metadata added to the app bundle, for exact SMuFL metrics. The metadata can be reduced to the glyphs the layout uses: that is our own derived data, not a modified font.
- One more license in the notices.

## Consequences

- ADR-015 gains amendment 1. The allow-list for code is unchanged.
- FF-09 (Dart part) treats the bundled font directory as data with recorded provenance, as for ADR-021.
- The S11 spike reads the font and metadata from a local copy outside the repository until this ADR is accepted.

## Related fitness functions

FF-09 (license policy), FF-29 (bundle size), and the checksum check of decision 6.

## Reconsideration conditions

- The project license is chosen (Q-10) and conflicts with OFL notices.
- A Bravura release fixes a glyph or metadata defect that the layout needs.
- The bundle size of the metadata becomes a problem (FF-29).
