# ADR-041 Notation engine: SMuFL metrics in Rust, bar-plan cache, vector PDF adapter, read-only hit testing (Phase 6c; amends ADR-018)

Status: Accepted (user, 2026-11-12), from the S11 proposals (spike README section 7.8, items 4 to 9). The user added the conditions in decisions 1, 3 and 5. The new crate and its dependencies (decision 5) are added to the matrix and checked by cargo-deny in the slice that creates them, under this ADR.
Date: 2026-11-12
Related documents: [ADR-018](ADR-018-notation-layout-in-rust.md) (amended), [ADR-021](ADR-021-bundled-font-ofl.md), [ADR-031](ADR-031-multi-track-tab-display.md), [ADR-037](ADR-037-bravura-smufl-font.md), [ADR-038](ADR-038-notation-staff-model.md), [ADR-039](ADR-039-pitch-capo-and-written-transposition.md), [ADR-040](ADR-040-theory-notation-rendering-boundary.md), [performance-requirements.md](../performance-requirements.md), `spikes/s11-notation/README.md` (section 7)

## Context

ADR-018 has Dart supply font metrics to the Rust layout. For TAB that is one digit width.

Standard notation needs, per glyph: bounding boxes, anchors and engraving defaults. SMuFL publishes these in the font's metadata, in staff spaces. The PDF needs the font files themselves.

S11 measured three things:
- Without a cache, layout cost grows with the score: LARGE in view p99 212 ms, against a budget of 33 ms.
- With bar plans cached, the same path is 4.9 ms.
- The SVG-to-PDF route was 70 times slower than writing PDF directly, and not deterministic. It also loaded system fonts.

`mc-notation` is a pure, WASM-compatible crate with no I/O (FF-03, FF-04).

## Decision

### 1. SMuFL metrics in Rust, pinned and generated

- A generator (`cargo xtask`) reads `Bravura.json` and `Bravura.otf` and writes a Rust table into `mc-notation`. The table covers the glyphs the engine uses.
  - Per glyph: SMuFL name, codepoint, glyph id, bounding box, advance width (from the font's `hmtx`), and the anchors **that glyph has**, each an `Option`.
  - Engraving defaults.
  - The font name, version and the files' SHA-256.
- `mc-notation` reads no file. Glyphs are an enum, so a glyph name cannot be missing at run time.
- **Pinned:** the table records Bravura 1.482 and its SHA-256 (ADR-037). A test regenerates the table from the bundled files and fails on any difference. A new font version needs an amendment of ADR-037.
- **Mapping checked at generation:**
  - every codepoint is in the font's cmap;
  - the outline box agrees with the metadata box within 0.05 staff spaces (the S11 check);
  - a glyph absent from the font, or a required anchor absent (for example `stemUpSE` on a notehead), **fails the generator with its name**. Nothing falls back silently.
- **Tests:** advance width, bounding box and anchors against the font for every glyph; scaling from staff spaces to layout units; no assumption that glyphs share sizes or anchors (an anchor-less glyph is handled explicitly, by its own rule).
- **Bundling moves earlier.** The Bravura files are bundled per ADR-037, with `OFL.txt`, `PROVENANCE.md` and the checksum check, in 6c-2. The generator test needs them.
- TAB digits keep ADR-018's Dart-measured width on screen. The PDF reads the same Noto Sans Mono file (ADR-021) in Rust. A test checks that the two widths agree.

### 2. Layout model

The immutable layout model of S11 (ADR-038) is the one output for screen and PDF:
- systems of staves;
- painted items (lines, glyphs, texts with their knockout, beams, ties);
- facts for checks and hits.

Renderers decide nothing, not even a knockout.

### 3. Bar-plan cache (mandatory)

A bar's plan, for all shown tracks, is cached and reused only if all of these are unchanged:

| Input | Why |
|---|---|
| The content of the bar in each shown track, by `Arc` identity (ADR-016; ADR-031 `LayoutCache`) | notes, durations, ties |
| The master bar (key, time) and the previous master bar | key and time display, accidentals by key |
| Each shown track's instrument (tuning, capo, fret count) and written transposition (ADR-039) | sounding and written pitch |
| The spellings of ties entering the bar from the previous bar (values) | tie spelling (ADR-040). On a change, re-planning continues forward until the entering spellings equal the cached ones, so a tie chain over many bars stays correct |
| The staff configuration of each shown track, and the shown tracks | staff kinds, reach |
| The metrics identity (font name, version, table SHA-256) and the TAB metrics | widths and reach |
| The layout constants version | spacing rules |

- A plan does not depend on the page or system width. Width, the score revision and the configuration key the outline (line breaking, system heights). That outline cache already exists in ADR-031's form.
- **Tests:**
  - the cached layout equals a fresh layout after every kind of change: note, duration, tie, key signature, time signature, accidental-changing edit, tuning, capo, written transposition, track added and removed, staff configuration, metrics;
  - a tie chain over 3 bars edited at its start re-plans exactly the chain;
  - one mutation per key component (that component left out) is caught, as in ADR-031.

### 4. Hit testing, read-only

- Rust emits hit regions for the notation staff: one per beat, from its noteheads and stem.
- Dart only maps a tap to a region and selects that beat through the existing selection. The TAB staff edits it.
- Dart computes no geometry.

### 5. PDF: a vector adapter `mc-pdf`

- New adapter crate `mc-pdf` (ADR-012), depending on `notation` only. Only `ffi` and `cli` use it.
- It writes the layout model to PDF with vector paths and embedded font text.
- Candidate dependencies, as used in S11: `pdf-writer`, `subsetter`, `ttf-parser`, `miniz_oxide`. S11 recorded them as MIT or Apache-2.0 (`miniz_oxide` also Zlib). This is to be verified by cargo-deny in the slice.
- **Fonts, by the user's conditions:**
  - only the bundled Bravura (subset, OFL-FAQ 1.12) and the bundled Noto Sans Mono are embedded;
  - no system font is loaded, and no standard-14 font is used;
  - a missing glyph is an error, never a substitution (no Consolas or other fallback).
  - The S11 vector prototype still uses Helvetica for TAB digits; 6c fixes that.
- **Tests:**
  - glyph positions read back from the PDF equal the layout model;
  - every font in the file is embedded, and its name is one of the two;
  - output is byte-identical across runs, processes and debug or release (no creation date);
  - fixtures and the generating command are kept with the outputs, so artefacts can be reproduced.
- **Pagination checkpoint:** before the export is committed to, a pagination checkpoint with its own criteria, fixed before code (6c-5a). It covers page size, margins, whole systems per page and multi-page output.
- **Manual acceptance in Microsoft Edge by the user**, per checklist:
  - Bravura glyphs correct, with no boxes or substitutes;
  - notation and TAB aligned;
  - vector at every zoom;
  - no dependence on installed fonts;
  - multiple pages once pagination exists.
  The agent supplies the PDF, its source fixture, and the list of embedded fonts with their subset names.

### 6. The S11 checks become tests

- N1 to N3, the bounds and page checks, and the key-signature count move into `mc-notation` as test support.
- Each S11 breakage becomes a test that corrupts a layout model and expects the check to fail. There is no fault switch in production code.
- One canonical layout per fixture is a golden (N8). Goldens are append-only.
- Per GR-14 these tests may be extended, never weakened.

### 7. Performance criteria for 6c (fixed now, before code)

- TYPICAL, notation and TAB, all tracks: edit plus layout in view p99 ≤ 16 ms.
- LARGE: p99 ≤ 33 ms.
- Measured both as the Rust bench and end to end in the app (ADR-031 protocol).
- Flutter paint cost is measured in 6c-3. If a criterion fails, the threshold is not changed. The cause is measured and the next step is decided with the user.

## Alternatives

- **Metrics measured in Dart** (ADR-018 as written). Rejected for notation: the PDF and tests also need them, and SMuFL already gives them in staff spaces.
- **Reading `Bravura.json` at run time in `mc-app`.** Possible, but it adds I/O and a parse on every start. A generated, pinned table is checked once.
- **SVG-to-PDF (`svg2pdf`).** Rejected by S11: not deterministic, 4.2 s against 60 ms on TYPICAL, and it loads system fonts.
- **No cache.** Rejected: 212 ms p99 on LARGE.

## Trade-offs

- The cache key has many parts. A missing part gives a stale layout, which is why each part has a mutation test.
- `mc-pdf` adds four external dependencies.

## Consequences

- ADR-018: metrics for standard notation come from SMuFL in Rust. TAB digit metrics on screen stay as they are.
- xtask gains the generator. The bundled Bravura files and their checksum check (ADR-037 decision 6) arrive in 6c-2.
- `architecture/allowed-deps.toml` and `deny.toml` change in 6c-5, with the trailer `Architecture-Change: ADR-041`.

## Related fitness functions

FF-01 (new row for `pdf`), FF-02 and FF-09 (licenses), FF-03, FF-04, FF-12, FF-21, FF-22, FF-34 (no `unsafe`).

## Reconsideration conditions

- A second music font.
- Paint cost in Flutter fails the budget with the model as designed.
- Pagination needs layout decisions that the system model cannot express.
