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

## 8. Slice 6c-2 acceptance criteria (Proposed 2026-11-12, pending the user; fixed before any code)

**Scope.** The notation staff inside `mc-notation`:
- the semantic layer (ADR-040 decision 2);
- layout of the MVP elements:
  - treble 8vb clef, key and time signatures;
  - whole, half and black noteheads; stems, flags, flat beams with levels, dots;
  - rests, including the whole-bar rest; accidentals, ledger lines, ties, tuplet numbers;
  - bar lines and repeat dots aligned with TAB;
- the bundled Bravura and the generated metrics table;
- the bar-plan cache;
- the S11 checks as tests, goldens, and the Rust bench.

**Not in 6c-2:**
- FFI, DTO and Dart (6c-3);
- notation hit regions (6c-4);
- PDF (6c-5);
- the polish items of S11 7.5 (6c-6).

Voltas, markers and tempo stay on the TAB staff only.

**Design points the criteria rely on:**
- **Unit.** The staff space is derived from the existing metrics: `staff_space = string_spacing / 1.5`, as in S11 (TAB string gap 1.5 staff spaces). No new metric and no DTO change in 6c-2.
- **API.** The existing functions keep their signatures and meaning: TAB only. New variants take a per-track `StaffConfiguration` (ADR-038).
  - New display types are additions: a stave kind; music glyphs at their SMuFL origin; lines with a thickness; ties; knockouts.
  - TAB staves leave them empty.
- **Generator: amended from decision 1.** `xtask` deliberately has no dependencies, so the generator does not go there. Instead:
  - an ignored test in `mc-notation` regenerates the table;
  - a normal test regenerates it in memory and compares it with the committed file;
  - both use dev-dependencies: `serde_json` (already in the workspace), and `ttf-parser`, which is new (MIT OR Apache-2.0; checked by cargo-deny in the slice).
- **Checksum check: amended from ADR-037 decision 6.** It is a test with the `sha2` dev-dependency (already in the workspace), registered as FF-35. `xtask` stays dependency-free.
- **Location.** The fonts live with ADR-021's font: `packages/ui_kit/fonts/bravura/` holds `Bravura.otf`, `Bravura.json`, `OFL.txt` and `PROVENANCE.md`.

| Id | Criterion | Checked by |
|---|---|---|
| L1 | **TAB unchanged.** With notation off on every track, the output equals today's on SMALL, TYPICAL, LARGE (track 1), `feature_rich` and `techniques` (fingerprints before and after). The existing goldens and the ADR-031 tests pass unchanged. | Equivalence and existing tests |
| L2 | **N1 correctness**, from score facts (not engine internals), on the six S11 inputs, `feature_rich`, `techniques`, SMALL and 300 seeded random scores. Covered:<br>– written pitch = `spell` plus `written_transposition`;<br>– staff position and ledger-line count;<br>– accidentals equal `bar_accidentals`;<br>– stem direction by the middle-line rule (voice 1 up and voice 2 down with two voices);<br>– beam groups (beat window, a tuplet as its own unit);<br>– tuplet numbers;<br>– whole-bar rests;<br>– key-signature signs per system. | Tests |
| L3 | **N2 alignment.** Bar lines and beat anchors of notation and TAB are equal to within 0.01 units, on the same inputs. | Tests |
| L4 | **N3 rules** (a) to (j) of S11, and **bounds**: every painted item inside its staff box, boxes inside their system, systems inside the outline. 0 violations on the same inputs. | Tests |
| L5 | **The checks catch faults.** Each of the 12 S11 breakages is a test that corrupts a layout model and expects its check to fail. There is no fault switch in production code. | Tests |
| L6 | **Boundary (ADR-040).**<br>– A source-scan test fails if `mc-notation` spells from semitones, keeps accidental state or reads alterations itself (`from_semitone_*`, `natural_semitone`, per-letter alteration tables).<br>– A test shows that the semantic output equals `mc-theory` applied directly. | Tests |
| L7 | **Metrics table.**<br>– The committed table equals a fresh generation.<br>– Codepoints are in the cmap; boxes agree within 0.05 staff spaces; advance widths come from `hmtx`; anchors appear only where the metadata has them.<br>– A glyph absent from the font, or a missing required anchor, makes the generator fail with its name (tested on a modified copy in memory).<br>– Scaling from staff spaces to layout units is tested. | Tests |
| L8 | **Bundled Bravura** with `OFL.txt` and `PROVENANCE.md` (source, release, sizes, SHA-256 as in ADR-037).<br>– FF-35 passes, and fails when a byte of either file changes (tested in memory).<br>– The fitness register and inventory (FF-20) list FF-35. | FF-35, FF-20 |
| L9 | **Cache** (decision 3).<br>– Cached equals fresh after every change kind listed there, over 60 seeded scores × 25 steps.<br>– A tie chain over 3 bars, edited at its start, re-plans exactly the chain.<br>– A note edit with no tie chain re-plans at most 2 bars.<br>– One mutation per key component is caught. | Tests |
| L10 | **Determinism.**<br>– One canonical dump golden per S11 input and `feature_rich` (append-only), compared as the existing goldens are.<br>– Two runs give identical output.<br>– A windowed layout equals the full layout. | Golden tests |
| L11 | **Performance**, Rust bench on the reference machine, every track with notation and TAB, edit plus layout in view (bar-plan cache):<br>– TYPICAL 4 tracks p99 ≤ 16 ms;<br>– LARGE 4 tracks p99 ≤ 33 ms;<br>– LARGE 20 tracks p99 ≤ 33 ms.<br>Rust only; end to end is measured in 6c-3. Payload per system is reported. | `mc-app` bench |
| L12 | **Review by eye.** SVG renders at 2x (a test-support renderer, not shipped) of the six S11 inputs and `feature_rich`, next to the S11 images. The user accepts them. | The user |
| L13 | **Non-functional.**<br>– No new runtime dependency; `ttf-parser` is a dev-dependency only.<br>– No `unsafe`.<br>– FF-03 and FF-04 for `mc-notation`; FF-22 additions only.<br>– fmt, clippy `-D warnings`, the workspace tests and CI are green. | CI and the fitness functions |

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
