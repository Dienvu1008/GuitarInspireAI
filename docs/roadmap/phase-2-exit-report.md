# Phase 2 Exit Report

Purpose: Evidence against the Phase 2 acceptance criteria (Guitar core), the golden set, benchmark results, interpretations that need user confirmation, ambiguous musical cases, and architecture changes.
Status: Draft for user review. Phase 2 is not accepted yet: the interpretations in section 5 and the ambiguous cases in section 6 need a decision. CI status in section 9.
Related documents: [implementation-phases.md](implementation-phases.md), [milestones.md](milestones.md), [phase-1-exit-report.md](phase-1-exit-report.md), [domain-invariants.md](../architecture/domain-invariants.md), [bounded-contexts.md](../architecture/bounded-contexts.md), [fitness-functions.md](../architecture/fitness-functions.md).

## 1. Outcome

Phase 2 delivered `mc-guitar` and the Phase 2 roadmap scope: tuning presets, fretboard position queries, the chord finder in both directions, voicing constraints, scale positions, retune planning, application queries, the retune command and CLI commands. No UI, audio, AI or unrelated features were added. The chord vocabulary is exactly the approved list (15 qualities); deferred qualities (`7b9`, `7#9`, `7#11`, `7b13`, `maj9`, `m9`, `11`, `13`) are rejected by the parser.

| Area | Where |
|---|---|
| Chord qualities as interval structures, `NoteName`, `ScaleFormula` | `mc-theory` (`chord.rs`, `note_name.rs`, `scale.rs`): value objects only (kernel rule) |
| Chord symbol notation (canonical ASCII, aliases) | `mc-theory/src/chord_symbol.rs`, separate from the interval model; aliases map to canonical qualities |
| Presets, fretboard, voicing, constraints, search, identification, scales, retune planning | `mc-guitar` (depends on `mc-theory` only) |
| Atomic retune of a track | `Score::retune_track` in `mc-score` (validates every shifted note; all or nothing) |
| Retune command, queries | `mc-app`: `Command::Guitar(GuitarCommand::Retune { .. })`, `mc_app::guitar::*` |
| CLI | `mc-cli chord`, `identify`, `scale`, `tunings`; script verb `retune` |

## 2. Acceptance criteria

| Criterion ([implementation-phases.md](implementation-phases.md) Phase 2) | Evidence | Result |
|---|---|---|
| Golden set of at least 100 chord and voicing cases agreed with the user passes | `crates/mc-guitar/tests/golden/chords.txt`: 235 cases, all passing (`g03_golden_set`). Agreement pending: section 6 | Passes; agreement pending |
| Voicing search under 50 ms for one chord [Assumption] | Section 4: p99 5.0 ms for the heaviest chord (C9, all results kept) | Met |
| Property tests G-01, G-02 | `crates/mc-guitar/tests/properties.rs` (`g01_`, `g02_`, two `g03_` properties, shape round trip) | Met |
| Golden tests in standard and alternate tunings | Drop D, DADGAD, open G, open D, half-step down and capo cases in the golden set | Met |

Milestone M2 demo ("CLI lists voicings for chords in standard and drop D"): `mc-cli chord C` lists `x32010` first; `mc-cli chord D --tuning drop-d` lists `000232` first. `mc-cli identify x32210` prints the ranked candidates with tones, missing tones and explanations.

## 3. Golden set composition

| Category | Cases |
|---|---|
| Symbol parsing and canonical display, aliases (`Δ7`, `ø7`, `°`, `°7`, `min`, `+`), slash chords | 32 |
| Rejected symbols (deferred qualities, power chord `C5`, unknown, malformed) | 16 |
| Chord spelling (`C E G Bb`, `C Eb Gb Bbb` ...) | 11 |
| Identification: triads, sevenths, sus, sixth, add9, 9, every quality | 54 |
| Identification: inversions and slash chords | 16 |
| Identification: omitted fifth, incomplete and invalid voicings | 16 |
| Identification: alternate tunings and capo | 12 |
| Constraint boundaries (span 4/5, fret 12/13, fingers, open strings, min fret, string count) | 20 |
| Voicing search (contains, excludes, first, count; constraint options; tunings; capo) | 44 |
| Retune planning | 9 |
| Scale and note positions | 5 |

Every candidate list is the full ranked list, not a single name. For every search result the runner also checks G-02 (constraints) and G-03 (the result is identified back as the requested chord). A negative control (one wrong expectation in each of `id` and `find`) failed the test as expected.

## 4. Benchmarks (reference machine, release)

| Query | p50 | p99 | Budget |
|---|---|---|---|
| `chord_voicings` C, all results | 0.40 ms | 1.1 ms | 50 ms |
| `chord_voicings` C9, all results | 2.7 ms | 5.0 ms | 50 ms |
| `chord_voicings` Cdim7, all results | 0.50 ms | 3.1 ms | 50 ms |
| `identify` x02010 (criterion mean) | 4.8 µs | | |

Measured in `crates/mc-app/benches/commands.rs` (`guitar_queries`), compiled in CI (FF-21).

## 5. Interpretations that need confirmation

These follow the approved conventions but fill in details the conventions did not fix. All are configurable defaults or documented rules; none is an invariant.

| # | Interpretation | Alternative |
|---|---|---|
| I-1 | Span = highest minus lowest fretted fret plus one (frets 1 to 4 = span 4, passes; 1 to 5 fails) | highest minus lowest (would allow five frets) |
| I-2 | Fingers: one barre at most, at any fret; it covers adjacent strings fretted at that fret or higher; an open, muted or lower-fretted string breaks it. Example: `x32333` (C9) needs 3 fingers; `x-1-2-3-4-2` needs 4 (barre at fret 2) | barre only at the lowest fret (C9 `x32333` would need 5 and be excluded) |
| I-3 | `min_fret` applies to fretted notes; open strings depend only on "allow open strings" | `min_fret > 0` also excludes open strings |
| I-4 | The fifth of `sus2` and `sus4` is not omissible. Without it Gsus4 (G C) equals an inverted C power chord and Csus2 (C D) is a bare second, so the quality is not identifiable. This narrows "omitted fifth allowed by default" for sus chords only | allow it (then `x355xx`, a C power chord, is named `Gsus4/C`) |
| I-5 | Rootless voicings are not named; the root must sound | name rootless voicings (later, with jazz vocabulary) |
| I-6 | Search needs at least 3 sounding strings; the bass is the root (or the slash note); `--inversions` allows any chord tone in the bass | other minimums |
| I-7 | Search order: lowest fretted fret, then more sounding strings, then fewer muted strings inside the shape, then per-string frets; default limit 50 | a playability score (Phase 6 fingering work) |
| I-8 | A slash bass that is not a chord tone (for example `C/D`) is supported by the search, but identification only produces slash chords whose bass is a chord tone | name non-chord-tone basses |
| I-9 | Default root spelling for pitch classes without context: C, Db, D, Eb, E, F, F#, G, Ab, A, Bb, B. Chord tones and slash basses are spelled from the root | key-aware spelling (section 6, group D) |

## 6. Ambiguous cases (flagged with `?` in the golden set)

The expected results are what the agreed rules produce. They were not changed to match any preference; each needs a decision to keep or change the rule.

| Group | Cases (shape: ranked result) | Question |
|---|---|---|
| A. Complete slash chord ranks above a root-position chord with no fifth | `x32210`: Am/C, C6 (no 5); `322000`: Em/G, G6 (no 5); `1x0231`: Dm/F, F6 (no 5); `x-3-x-2-4-x`: Adim/C, Cm6 (no 5) | Chord charts usually call `x32210` "C6". Keep "fewer missing tones" before "root in bass", or swap the two keys? Swapping makes C6 preferred here; it also prefers e.g. "G6 (no 5)" over "Em/G" |
| B. Same notes, both complete | `x02010`: Am7, C6/A; `x2323x`: Bm7b5, Dm6/B; `x32253`: C6, Am7/C; `3x2210`: Am7/G, C6/G | Inherent; root in bass decides, then quality order. Keep? |
| C. Symmetric chords | `x3211x`: Caug, Eaug/B#, Abaug/C; `xx1212`: Ebdim7, F#dim7/Eb, Adim7/Eb, Cdim7/Eb; `x-3-4-2-4-x`: Cdim7, Ebdim7/Dbb, F#dim7/C, Adim7/C | Slash basses are spelled as chord tones (B#, Dbb). Keep, or spell the bass with the default spelling (Eaug/C, Ebdim7/C)? |
| D. Root spelling without key context | `x46654`: Dbm, E6/C# (most charts say C#m); half-step-down `022000`: Ebm, F#6/D# | Keep the default table until key-aware spelling exists (when chords are inserted into a score with a key signature), or prefer C# and G# for minor roots now? |
| E. Two pitch classes | `x32x1x` (C E C): C (no 5) | Name dyads as triads without fifth, or require at least 3 distinct pitch classes for a name? |

Groups A, D and E affect which name is shown first; none affects which voicings the search returns.

## 7. Fitness functions

23 of 33 active; `current_phase = 2`. New: FF-25 and FF-26 (both advisory reports, `cargo xtask fitness coupling` and `cargo xtask fitness module-cycles`, wired into CI).

| Id | Phase 2 evidence |
|---|---|
| FF-01 | 6 crates checked; `mc-guitar` depends on `mc-theory` only; `mc-app` gained `mc-guitar` (already allowed by the matrix in dependency-rules section 1) |
| FF-03, FF-04 | `mc-guitar` has the domain `clippy.toml` and compiles for wasm32; the golden set is embedded with `include_str!` (no file I/O in domain tests) |
| FF-13, FF-14 | The random command generator gained a retune operation (all presets, capo 0 to 2, both modes) |
| FF-22 | Snapshots regenerated for mc-theory, mc-score, mc-app; `mc-guitar.txt` added; weekly job covers mc-guitar |
| FF-24 | 41 of 55 rule ids have tests (was 38); G-01, G-02, G-03 covered |
| FF-25 | Crate fan-in/fan-out: mc-theory 4/0, mc-score 2/1, mc-guitar 1/1, mc-project 2/2, mc-app 1/4, mc-cli 0/2 |
| FF-26 | One existing cycle reported: `mc-score` modules `score` and `validate` (Phase 1 code; advisory). Not changed in Phase 2 |
| FF-28 | 0 cognitive-complexity warnings; 7 functions over 100 lines (advisory; new: the golden runner's case dispatcher) |

Workspace tests: 164 passing (including xtask self-tests), 0 failing.

## 8. Architecture changes supported by implementation evidence

| Change | Evidence | Where recorded |
|---|---|---|
| Retune is one aggregate operation, `Score::retune_track(track, instrument, fret_deltas)` | Applying per-note edits one by one would clear valid ties mid-way (H-06 maintenance); per-string deltas keep tied notes tied | mc-score tests `h04_h06_...`, `h04_retune_is_all_or_nothing` |
| Retune planning returns per-string fret deltas including the capo difference | Keep-pitches = (old open + old capo) - (new open + new capo) per string | `mc-guitar` retune tests and golden cases |
| `allowed-deps.toml`: `mc-app` may depend on `mc-guitar` | Needed for the retune command and queries; the documented matrix already allowed it | `architecture/allowed-deps.toml` (ADR-003, ADR-019) |
| `CommandError::Guitar` | Retune planning errors (string count change with keep-pitches) are Guitar-context errors | `mc-app` |
| Sus chords keep their fifth (I-4) | Golden case `x355xx` | `mc-theory` chord intervals; domain-invariants G-03 notes |

No ADR was reversed. No fitness function, lint or dependency rule was weakened.

## 9. CI status

Local runs of every CI step passed: fmt, clippy `-D warnings`, workspace tests, benchmark build, `xtask fitness all`, FF-24/25/26 reports, Mermaid, cargo-deny. GitHub Actions result: recorded after the push of branch `phase-2-guitar`.

## 10. Not done in Phase 2

`CagedShape` and the fingering optimizer (later phases), diagnostics D-02 (unplayable stretch in a beat) and D-04 (note outside practical range after retune), which need the editor (Phase 3). A retune that would move a note outside the fret range is rejected as a whole (H-04), not reported per note.

## 11. Decisions needed

1. Section 5: confirm I-1 to I-9, or name the ones to change.
2. Section 6: keep or change each of groups A to E.
3. Then: accept Phase 2 and merge `phase-2-guitar` into `main`.
