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
  - Note (2026-11-16): `ttf-parser` is unmaintained (RUSTSEC-2026-0192), so it is replaced. Whether `subsetter` depends on it is checked in 6c-5 (section 8 follow-up).
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

## 8. Slice 6c-2 acceptance criteria (fixed 2026-11-12, before any code)

**User decision (2026-11-12).**
- Approved: the scope; TAB backward compatibility as a mandatory invariant; L1 to L6; L9; the L11 thresholds; the L12 method (SVGs to be supplied for review).
- Approved: the generator as an ignored test plus a regular consistency test; FF-35 with `sha2`; `ttf-parser` as a dev-dependency.
- Approved: `staff_space = string_spacing / 1.5`, on condition that the coordinate system is documented and no magic number is used. See "Coordinate system" below.
- L7 and L8 are the generator and FF-35 decisions in criterion form.
- L10 (goldens and determinism) and L13 (non-functional) were confirmed by the user on 2026-11-16.

**Follow-up decision (user, 2026-11-16): `skrifa` replaces `ttf-parser`.**
- Reason: cargo-deny failed on RUSTSEC-2026-0192, which marks `ttf-parser` as unmaintained. The user chose to replace it rather than ignore the advisory.
- `skrifa` 0.50.0 (Google Fonts fontations, MIT OR Apache-2.0) is the replacement the advisory names. It is a dev-dependency of `mc-notation` only, and brings 11 crates into the lockfile.
- It reads CFF outline boxes from the outline. The table it generates is byte-identical to the one `ttf-parser` generated.
- Where this section says `ttf-parser`, read `skrifa`. Decision 5's candidate list for `mc-pdf` is to be re-evaluated in 6c-5, because `ttf-parser` is no longer acceptable there either.

**Coordinate system (the user's condition).** All notation geometry is computed in staff spaces, then multiplied by one factor.
- One named constant, `TAB_STRING_GAP_SP = 1.5`: the TAB string gap in staff spaces. This is the only link between TAB metrics and notation.
- `staff_space = LayoutMetrics::string_spacing / TAB_STRING_GAP_SP` is computed in one function, `LayoutMetrics::staff_space()`.
- SMuFL: 1 em = 4 staff spaces. Glyph metrics in the generated table are in staff spaces.
- Layout units are those of `LayoutMetrics`: logical pixels at the current zoom, with y growing downward.
- A staff's coordinates start at its box top (ADR-038 decision 3).
- A staff position `p` counts half staff spaces above the bottom line (E4 written = 0 on the treble staff, top line = 8): `y = lines_top + (8 - p) / 2 * staff_space`.
- Engraving distances (stem length, gaps, pads) are named constants in staff spaces in one module, each with its source: the SMuFL engraving defaults, or a stated rule.

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

### Results of 6c-2 (2026-11-16; closed)

| Id | Result |
|---|---|
| L1 | **Pass.** 20 fingerprints (SMALL, TYPICAL, LARGE, `feature_rich`, `techniques`; two widths; `layout_tab` and `layout_score`) recorded from the code before notation (commit 958bf6a) equal the new output in release and debug builds. The existing goldens and ADR-031 tests are unchanged. The TAB-only configuration of the new API equals the old API. |
| L2, L3, L4 | **Pass.** 0 violations on the six S11 inputs, `feature_rich`, `techniques` and SMALL at two widths, and on 300 seeded random scores. A coverage test shows the random scores contain accidentals, ties, seconds, tuplets, second-voice stems, ledger lines and key signs (at least 20 of each). |
| L5 | **Pass.** 12 tests each corrupt a layout model and get the expected rule. |
| L6 | **Pass.** A source scan finds no spelling or accidental rule in `mc-notation`; only `semantic.rs` calls `mc-theory`'s rules. The checks take spelling and accidentals from `mc-theory` directly. |
| L7, L8 | **Pass** (step 1, 958bf6a). |
| L9 | **Pass.** Cached equals fresh over 60 scores × 25 steps, with all 12 change kinds seen. A tie chain over 3 bars edited at its start re-plans exactly those 3 bars. A note edit re-plans at most 2. Leaving out each of the 6 key components (master, previous, contents, instruments, staves, entering ties) is caught. A change of written transposition alone cannot happen yet: it is derived from the instrument kind, and only fretted instruments exist. |
| L10 | **Pass.** Goldens for the six S11 inputs and `feature_rich`. Two runs are identical. Each system laid out alone equals the full layout. |
| L11 | **Pass** (Rust bench, reference machine, notation and TAB on every shown track, edit plus layout in view): TYPICAL 4 tracks p99 0.63 ms (budget 16); LARGE 4 tracks p99 1.05 ms (33); LARGE 20 tracks p99 3.03 ms (33). The worst single samples were 5.3, 21.6 and 101.6 ms. These are probably the first edit after open, which fills the cache; not separated in this run. Payload of one system with notation, LARGE 20 tracks: 991 lines, 1 407 glyphs, 160 beams on the notation staves. |
| L12 | **Accepted by the user (2026-11-16)** on the 2x review images of the six S11 inputs, `feature_rich` and `techniques`. 6c-2 is closed. |
| L13 | **Pass** locally: fmt, clippy `-D warnings`, 73 test suites, cargo-deny, `xtask fitness all`; FF-22 snapshots of `mc-notation` and `mc-app` with 583 lines added and 0 removed. CI pending the push. |

Found while testing, and fixed:
- **Rests with two voices.** A rest could overlap a head of the other voice at the same onset (random seed 4). Rests now move away from the other voice a staff space at a time.
- **Tuplet numbers.** A number could come too close to another voice's note in its span (seed 251). Its placement now clears every beat under it.
- **The ledger-neighbour check.** It wrongly counted two voices at one onset as neighbouring columns (seed 8). It now compares columns, as the S11 rule says.

Known limitation, not in the MVP rules: **when two voices cross**, their heads at one position and onset overlap. Multi-voice collision handling was outside S11 and the MVP. It is proposed for 6c-6 or later, with its own rule.

## 9. Slice 6c-3 acceptance criteria (fixed 2026-11-16, before any code)

**User decision (2026-11-16).** F1 to F9 approved. All three choices as proposed:
1. notation on for every track when a document opens, not remembered;
2. a per-track toggle in the track list;
3. the TAB rhythm is kept in 6c-3 and revisited in 6c-6.

**Scope.**
- Notation staves cross the FFI.
- Flutter paints them above TAB with the bundled Bravura.
- A per-track view setting switches notation on and off.
- The paint and payload cost is measured end to end in the app.

**Not in 6c-3:**
- hit testing on the notation stave (6c-4);
- PDF (6c-5);
- engraving polish (6c-6).

**Design points the criteria rely on:**
- **FFI, additions only.** New functions take the staff configuration of each shown track (`ShownTrackDto { track, notation, tab }`): `score_outline_with` and `score_systems_with`. The existing `score_outline` and `score_systems` stay TAB-only and unchanged. `StaveDto` gains `kind` and `notation: NotationStaffDto`, which is empty on TAB staves. It carries `staff_space`, `lines_top` and packed arrays:
  - lines: x1, y1, x2, y2, thickness, plus a kind code;
  - glyphs: SMuFL codepoint plus x, y (the SMuFL origin);
  - beams;
  - ties: x1, x2, y, height, end thickness, middle thickness, plus up flags.

  Facts are not sent until 6c-4 needs them. Bindings are regenerated by codegen (FF-12).
- **Dart paints only.**
  - Glyphs are drawn by codepoint with the font family `Bravura` from `ui_kit`, at 4 staff spaces per em, on the alphabetic baseline at the given origin.
  - Lines, beams and ties are drawn as given.
  - Dart computes no position or size and holds no musical rule (ADR-040 decision 4).
- **Font.** `ui_kit` declares `Bravura.otf`: the same file FF-35 checks, unmodified (ADR-037).
- **Editing stays on TAB** (ADR-038 decision 7). A tap on a notation stave makes its track active and moves nothing else; hits come in 6c-4. The selection and the cursor are painted on the TAB stave only. The playback cursor spans every stave of the system.

| Id | Criterion | Checked by |
|---|---|---|
| F1 | **TAB unchanged.** With notation off, the existing FFI, Dart tests and contract tests pass unchanged. The new FFI functions with TAB only return the same DTOs as the old ones. | Rust and Dart tests |
| F2 | **DTO faithful.** For the S11 inputs and `feature_rich`, every line, glyph, beam and tie of every notation stave unpacks to the core layout's values exactly. Staves come in the order notation then TAB per track, in score order. | `mc-ffi` test |
| F3 | **Contract.** A new shared contract test, for the fake and the real service, covers:<br>– staves in order notation then TAB;<br>– bar x and width equal on every stave of a system;<br>– outline height equals the sum of stave heights plus gaps;<br>– a config with neither staff is an error. | `core_bridge` contract |
| F4 | **Painting.** Widget tests check:<br>– a notation stave item per shown track with notation, above its TAB item;<br>– the toggle shows and hides it;<br>– a tap on a notation stave selects its track and does not move the cursor;<br>– the painter reports the codepoints it drew, equal to the DTO's;<br>– the font family is `Bravura` from `ui_kit`. | `editor_feature` tests |
| F5 | **Font bundled.** `ui_kit` declares `fonts/bravura/Bravura.otf`. The app build contains it byte-identical to the file FF-35 checks. | Test on the built asset |
| F6 | **Boundary.** No musical rule in Dart: no pitch, spelling, accidental, staff position or spacing code in `editor_feature`. Review, plus a source scan for a list of forbidden names. FF-05 and FF-08 are unchanged. | Test and review |
| F7 | **Performance, end to end**, reference machine, profile build, ADR-031 protocol (one instance at a time, several runs, median and range), notation and TAB on every shown track:<br>– TYPICAL 4 tracks: edit compute bound p99 ≤ 16 ms; scroll frame p99 ≤ 16 ms and no frame over 33 ms;<br>– LARGE 4 tracks: edit p99 ≤ 33 ms;<br>– LARGE 20 tracks: measured and reported against 33 ms (the ADR-031 evidence gap still applies).<br>Payload per system in bytes is reported. A failure is measured and taken to the user; the threshold is not changed. | `app/tool/edit_latency.dart` with a notation option |
| F8 | **Review by eye** in the running app at 100 % and 200 % zoom (screenshots) of `keys`, `feature_rich` and TYPICAL. Accepted by the user. | The user |
| F9 | **Non-functional.**<br>– fmt, clippy, `dart analyze --fatal-infos`, all tests and CI green;<br>– FF-12 bindings regenerated by codegen;<br>– FF-22 additions only;<br>– no new Rust or pub dependency;<br>– no `unsafe`. | CI and the fitness functions |

**Choices for the user before code:**
1. **Notation on open.** Proposed: **on for every track** (the product aim: notation above TAB). Alternative: off, switched on per track. Like visibility, the setting is not remembered (ADR-031 5.4).
2. **Where the switch is.** Proposed: a per-track toggle in the track list, next to the eye button. Alternative: one global toggle in the toolbar.
3. **TAB rhythm when notation is shown.**
   - Proposed: **keep the TAB rhythm** (stems and beams below TAB) in 6c-3, and revisit it in 6c-6. This keeps L1 and needs no layout change.
   - Alternative: hide the TAB rhythm when notation is on, as many editors do. That is a Rust layout change with its own tests.

### Results of 6c-3 (2026-11-16; closed)

**User decision (2026-11-16).** F8 accepted. For F7, LARGE is accepted as an evidence gap, as in ADR-031: the two-state machine behaviour, with the cause not found. The thresholds are unchanged. 6c-3 is closed.

| Id | Result |
|---|---|
| F1 | **Pass.** The new FFI functions with TAB only return DTOs equal to the old functions on `feature_rich`, `techniques` and SMALL. The existing Rust, Dart and contract tests pass. Two Dart widget tests changed their stave lookup, because notation is now on by default: the tap test finds the TAB stave by key, and the M5 test counts notation staves too. Their assertions are unchanged. |
| F2 | **Pass.** Every line, glyph, beam and tie of every notation stave unpacks to the core layout's values exactly. Staves come notation then TAB per track, in score order. |
| F3 | **Pass**, fake and real service: order, bar alignment, stave tops and heights, outline height, TAB-only equality, and a track with no staff rejected. |
| F4 | **Pass.** Six widget tests: notation above TAB on open, the toggle, a tap on notation selects the track only, the painter draws exactly the DTO's codepoints in `packages/ui_kit/Bravura`, and another document at the same revision is laid out again. |
| F5 | **Pass.** `ui_kit` declares `fonts/bravura/Bravura.otf`. In the profile build it is `data/flutter_assets/packages/ui_kit/fonts/bravura/Bravura.otf`, 889 228 bytes, SHA-256 `CDF0F893…5F3B`: equal to PROVENANCE.md. |
| F6 | **Pass.** A source scan finds no spelling, accidental, semitone, staff-position, transposition or key-signature code in `editor_feature`. FF-05 and FF-08 pass. |
| F7 | **Partly met.** See below. |
| F8 | **Accepted by the user (2026-11-16)** on app screenshots of `keys`, `feature_rich` and TYPICAL at 100 % and 200 %. |
| F9 | **Pass** locally: fmt, clippy `-D warnings`, 73 Rust test suites, `dart analyze --fatal-infos` and the tests of every Dart package, cargo-deny, `xtask fitness all`. FF-12: bindings regenerated by codegen and stable on regeneration. FF-22: one line added to `mc-app`. No new dependency; `mc-ffi` gains `mc-app` with fixtures as a dev-dependency (an existing edge). CI pending the push. |

**F7, end to end** (reference machine, profile build, `app/tool/edit_latency.dart`, one instance at a time; ms; "compute" is the ADR-031 bound: command plus frame build plus raster, each p99):

| Case | Runs | Compute p99 | Edit to frame p99 | Scroll frame p99, frames over 33 ms | Payload of system 0 |
|---|---|---|---|---|---|
| TYPICAL 4, notation (quiet machine) | 4 | 9.8, 10.6, 10.1, 12.7 | 16.1 to 17.4 | 6.5 to 13.8, 0 | 37 327 B |
| TYPICAL 4, TAB only, interleaved | 4 | 12.5, 13.2, 12.8, 12.7 | 16.2 to 16.5 | 7.4 to 10.5, 0 | 48 160 B |
| TYPICAL 4, notation (agent busy, 1.1 to 1.6 other cores) | 3 | 10.1, 22.9, 27.4 | 16.9 to 36.5 | 7.7 to 17.8, 0 | |
| LARGE 4, notation | 5 | 24.8, 32.6, 31.1, 14.2, 32.6 | 17.4 to 41.2 | 7.0 to 20.7, 0 | 29 332 B |
| LARGE 20, notation | 4 | 40.8, 30.5, 23.2, 16.1 | 21.8 to 42.7 | 6.6 to 22.9, 0 | 146 660 B |

Reading:
- **TYPICAL meets the budget** on a quiet machine: compute p99 9.8 to 12.7 ms (budget 16), and scroll p99 at most 13.8 ms with no frame over 33 ms. With notation it is not slower than TAB only. The TAB-only payload per system is larger only because its first system holds more bars.
- **LARGE is in the two-state pattern of ADR-031** (fast and slow states of the machine, cause not found). LARGE 4: compute within 33 ms in 5 of 5 runs, edit to frame p99 within 33 ms in 1 of 5. LARGE 20: compute within 33 ms in 3 of 4, edit to frame in 1 of 4. No scroll frame went over 33 ms in any run.
- The runs with high other-process CPU came while the agent itself was working; the agent cannot avoid that. The thresholds are not changed. LARGE stays an evidence gap, as in ADR-031, for the user.

**Found and fixed** (by the F8 screenshot tool):
- The view's cache key had no document identity. A newly opened document with the same revision and track ids kept the previous document's layout.
- A test reproduced it (fails before the fix, passes after). The key now includes the service identity.
- The bug existed before 6c-3; it showed now because the fake and the tool open documents in sequence.

## 10. Slice 6c-4 acceptance criteria (Proposed 2026-11-16; partly approved, see below; fixed before any code)

**User decision (2026-11-16).**
- Approved: the notation stave reuses `SystemDto`'s hit arrays, with no new DTO field.
- Approved: the two kinds of region (head and beat); head before beat in `hit_test`; no append regions on notation.
- Approved: all three behaviour choices as proposed (string from the head; cursor and selection on both staves; a tap on another track's notation selects it and places the cursor).
- H1 to H9: approved. The user confirmed that there are no added conditions, and coding may start (2026-11-16).

**Precision added while designing the code (no change of scope).** "Head before beat, then the cursor's voice" becomes this preference order in `hit_test`, the same in Rust and in Dart:
1. a head region of the given voice;
2. a head region of any voice;
3. a beat region of the given voice;
4. any region.

Every TAB region has a string of 1 or more, so on TAB this order is exactly the old one (the given voice first, then any). H4 checks that.

**Scope:** read-only hit testing on the notation stave (decision 4). A tap on notation selects a beat (and a string where the tap identifies one). Editing stays on TAB. No editing command changes.

**Design points the criteria rely on:**
- **No new DTO field.** The notation stave's `SystemLayout.hits` (today empty) carries the regions, in the same packed arrays TAB uses: `hit_rects`, `hit_meta` (bar, string, voice, slot) and `beats`. Dart's existing `SystemHits` helpers then work on both staves.
- Rust computes every region from the facts it already has. There are two kinds per beat:
  - **Head region** (one per note): the notehead box grown by a pad, carrying the note's string.
  - **Beat region:** the beat's column (TAB's x range) over the staff box, with string 0, meaning "keep the cursor's string". A rest has only the beat region.

  `hit_test` prefers a head region over the beat region, then the cursor's voice.
- **No append regions on notation.** The empty-bar or insertion slot is a TAB editing action.

| Id | Criterion | Checked by |
|---|---|---|
| H1 | **Coverage.** On the S11 inputs, the fixtures and 300 random scores:<br>– every notated beat (notes or rest) has exactly one beat region and one head region per note;<br>– head regions contain their head's box;<br>– every region lies in its stave box;<br>– beat regions of one voice do not overlap. | Rust tests |
| H2 | **Addressing.** Every region's (bar, voice, beat index, beat id) equals the TAB hit of the same beat; a head region's string is its note's string. | Rust tests |
| H3 | **Hit test.**<br>– At each head's centre, `hit_test` returns that head.<br>– At a column point away from heads, it returns the beat region.<br>– With two voices, it prefers the given voice.<br>– Outside every region it returns none. | Rust tests |
| H4 | **TAB unchanged.** The L1 fingerprints and the F1 tests pass unchanged (TAB staves carry exactly the same hits). | Existing tests |
| H5 | **Dart behaviour**, widget tests:<br>– a tap on a notehead moves the cursor to that bar, voice, beat and string;<br>– a tap on a beat region keeps the cursor's string;<br>– Shift+tap extends the selection as on TAB;<br>– a tap on another track's notation makes it active and places the cursor (one click, as ADR-031 M4);<br>– a tap outside every region changes nothing. | `editor_feature` tests |
| H6 | **Boundary.** Dart computes no hit geometry; it only reads the arrays. F6 scan unchanged. | Test and review |
| H7 | **Performance.**<br>– The L11 bench stays within its thresholds.<br>– TYPICAL end to end stays within 16 ms compute p99 (3 runs).<br>– The payload growth per system is reported. | Bench and tool |
| H8 | **Manual check by the user** in the app (a short script: tap heads, rests, two voices, another track). | The user |
| H9 | **Non-functional.**<br>– fmt, clippy, analyze, all tests and CI green;<br>– FF-12 unchanged (no DTO change);<br>– FF-22 additions only;<br>– no new dependency. | CI |

**Choices for the user before code:**
1. **String from a notehead.** Proposed: a tap on a notehead sets the cursor's string to that note's string; a tap elsewhere on the beat keeps the string. Alternative: always keep the string.
2. **Cursor and selection on notation.** Proposed: **painted on both staves.** On notation the cursor is a column tint over the beat region, with the same colours as TAB, so the user sees which beat is selected. Alternative: TAB only, as in 6c-3.
3. **A tap on another track's notation.** Proposed: select the track **and** place the cursor, consistent with TAB (ADR-031 M4). This changes 6c-3, where it only selected the track. Alternative: keep selecting the track only.

### Results of 6c-4 (2026-11-21; closed)

**User results (2026-11-21).**
- **H7 end to end: pass.** The user ran the tool three times on a quiet machine (no agent running), TYPICAL with 4 tracks and notation. `p99_compute_bound_us` stayed within 16 000 µs in all three runs (9 071 µs reported). This settles the unclear agent runs below.
- **H8: pass.** The user's manual check in the release app covered all seven steps of the script. Step 3, where a click in the beat column away from the head keeps the string, was explained and then confirmed.

6c-4 is closed.

| Id | Result |
|---|---|
| H1, H2 | **Pass** on the S11 inputs, the fixtures and 300 random scores:<br>– one beat region per notated beat and one head region per note;<br>– head regions contain their heads;<br>– every region lies in its stave box;<br>– beat regions of one voice do not overlap;<br>– every region is addressed as the TAB hit of the same beat.<br>The shared contract also checks the addressing on the real and the fake service. |
| H3 | **Pass.** At a head's centre a head wins. At a column point away from heads the beat region of the given voice wins (seen with two voices more than 20 times). Outside every region there is none. |
| H4 | **Pass.** L1 fingerprints, F1 and the ADR-031 tests unchanged. The new preference order is the old one on TAB, since every TAB region has a string. |
| H5 | **Pass**, 5 widget tests: a head sets beat and string; a beat region keeps the string, and the cursor is painted on notation; Shift+tap extends the selection; a tap on another track's notation selects it and places the cursor; a tap outside changes nothing. The M4 test now expects the cursor on both staves of the active track (choice 2): the assertion was changed to the approved behaviour, and it still requires the cursor on the active track only. |
| H6 | **Pass.** Dart reads the arrays and chooses among regions in the core's order; it computes no geometry. The F6 scan is unchanged. |
| H7 | **Rust: pass.** L11 p99 is 0.74, 2.29 and 7.40 ms (budgets 16, 33, 33). **End to end: unclear (machine state).** See below. |
| H8 | Pending the user (manual script below). |
| H9 | **Pass** locally: fmt, clippy, 73 Rust suites, analyze and tests of every Dart package, cargo-deny, `fitness all`. FF-12: codegen gives no change. FF-22: no change (only the body of `hit_test` changed). CI pending the push. |

**H7, end to end** (TYPICAL 4 tracks, profile build, one instance at a time; ms):

| Runs | Notation: compute p99 | TAB only, interleaved: compute p99 | Edit to frame p99, both |
|---|---|---|---|
| First 3 | 9.6, 18.2, 21.6 | | 16.1 to 31.8 |
| Interleaved 6 | 11.9, 12.0, 40.4 | 33.3, 13.9, 18.0 | 29.6 to 38.6 |

- In the interleaved runs, TAB only (no notation, no notation hits) was as slow as with notation. Edit to frame was about 30 ms for both. This is the machine's slow state of ADR-031, not a cost of 6c-4.
- In the 40.4 ms run, frame build p99 was 7.2 and raster 5.5; the command round trip spiked.
- The thresholds are not changed. The proposal is that the user runs the tool on a quiet machine (no agent running).
- Payload of system 0: 48 335 B with hits, against 37 327 B before (+11 kB).

**H8 manual script (for the user):**
1. Open a score with notes.
2. Click several noteheads: the cursor moves to that beat and string on TAB, and a column tint appears on notation.
3. Click between heads in a beat: the beat changes and the string stays.
4. Shift+click another beat: the selection spans both staves.
5. With two voices, click each voice's notehead.
6. Click another track's notation: that track becomes active with the cursor on the beat.
7. Click the clef area: nothing changes.

## 11. Slice 6c-5a pagination checkpoint: acceptance criteria (proposed 2026-11-21, fixed 2026-11-26 before any code)

**User decision (2026-11-26).** PG1 to PG9 approved, with all four choices as proposed:

1. A4 by default, with Letter as an option;
2. the title on page 1 and "page n of N" on every page;
3. a system taller than a page body breaks between tracks, keeping each track's notation and TAB together;
4. the export contains what the view shows.

**Purpose (decision 5).**
- Settle how a layout becomes pages, before the PDF export (6c-5b) is committed to.
- The checkpoint ends with a Go or Revise for 6c-5b, decided by the user.
- No PDF bytes are written in 6c-5a.

**Dependency re-check (follow-up of section 8, 2026-11-21, crates.io):**
- `pdf-writer` 0.15.0: bitflags, itoa, memchr, ryu.
- `subsetter` 0.2.6: rustc-hash, plus optional skrifa, kurbo and write-fonts. It **no longer uses `ttf-parser`**.
- `miniz_oxide` 0.9.1: adler2.
- All are MIT OR Apache-2.0 (`miniz_oxide` also Zlib).
- The list for 6c-5b is therefore `pdf-writer`, `subsetter` and `miniz_oxide`, with `skrifa` (already a dev-dependency) for font reading. cargo-deny runs in 6c-5b, when they are added.
- A larger alternative, `krilla` (by the same authors, with about 15 dependencies), is noted and not proposed.

**Design points the criteria rely on:**
- **Pagination is a layout decision, so it lives in `mc-notation`** as a pure function. No new crate in 6c-5a.
  - Input: the score, the shown staves and a page description (size and margins in points).
  - Output: pages, each a list of system indices (or stave ranges, see choice 3) with their y on the page.
- **Units.** One layout unit is one PDF point (1/72 inch). The layout width is the page width minus the margins. The staff size comes from `LayoutMetrics::string_spacing` (default 12 pt). It is fixed per export and is not the screen zoom.
- **Page geometry, proposed defaults:**
  - A4, 595.28 × 841.89 pt; margins 36 pt (0.5 in);
  - systems placed from the top with the existing system gap;
  - a page holds whole systems;
  - no system is stretched vertically to fill a page.
- **Header and footer, proposed:** the title (document name) on page 1, and "page n of N" at the foot of every page. Their space comes out of the page body.
- **TAB text font in the PDF.** It is Noto Sans Mono (ADR-021), which is a variable font. Embedding a variable-font subset is a known risk. 6c-5a checks with `subsetter` that the TAB digits can be subset and embedded at the default instance. That is a probe, not the export.

| Id | Criterion | Checked by |
|---|---|---|
| PG1 | **Whole systems, in order.** Every system appears exactly once, in order. No page body is overfilled. A page break happens only between systems, or, for a system taller than a page body, as choice 3 decides. | Rust property tests over the S11 inputs, the fixtures and 300 random scores, at two page sizes |
| PG2 | **Greedy and stable.** Each page takes as many systems as fit. An edit that changes only bars on page k does not change pages before k. | Tests |
| PG3 | **Same systems as the screen.** At the page's content width, system breaks and system content equal `outline_score_with` and `layout_score_systems` at that width. Pagination only stacks systems; it re-lays nothing out. | Equality test |
| PG4 | **Header and footer.** Page 1 reserves the title; every page reserves the footer. Systems never overlap them. | Tests |
| PG5 | **Page geometry values.** A4 and Letter sizes in points, margins and content width are tested as numbers. Every distance is a named constant with its source. | Unit tests |
| PG6 | **Font probe.** `subsetter` subsets Bravura (glyphs used by `feature_rich`) and Noto Sans Mono (digits, parentheses, "TAB"). Each subset parses back with `skrifa`, and each glyph maps to the same outline as in the full font. The result for the variable font (works, or the limits found) is reported for the 6c-5b decision. | A test with dev-dependencies only |
| PG7 | **Performance.** Paginating TYPICAL and LARGE (4 and 20 tracks with notation) is measured, and reported against the 2 s "open large project" budget as the reference for a one-off export. | Bench |
| PG8 | **Non-functional.**<br>– No runtime dependency added in 6c-5a; `subsetter` is a dev-dependency for PG6, checked by cargo-deny.<br>– FF-03, FF-04 and FF-22 additions only.<br>– CI green. | CI |
| PG9 | **Checkpoint report** in this section: page counts for the fixtures, the oversize-system cases found, the PG6 font result and a page preview (SVG from the test renderer) for the user. Then **Go or Revise for 6c-5b**, decided by the user. | The user |

**Choices for the user before code:**
1. **Page size.** Proposed: A4 by default, Letter as an option (both supported in 6c-5b). Alternative: A4 only.
2. **Header and footer.** Proposed: title on page 1, and "page n of N" on every page. Alternative: page numbers only.
3. **A system taller than a page body.** This happens with many tracks: LARGE with 20 tracks gives a system of about 2 500 units, against a body of about 770 pt.
   - Proposed: **break between tracks**, keeping each track's notation and TAB together. The next page continues the same bars, with the clef and the key repeated. Only if one track alone is taller than a page (not seen in practice), break between its notation and TAB.
   - Alternative: export only what fits, and refuse with a message otherwise.
4. **Which staves are exported.** Proposed: **what the view shows**, meaning the shown tracks and their notation and TAB setting. Alternative: always every track with notation and TAB.

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
