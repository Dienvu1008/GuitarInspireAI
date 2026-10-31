# Phase 2 Exit Report

Purpose: Evidence against the Phase 2 acceptance criteria (Guitar core), the golden set, benchmark results, the Phase 2 conventions, the closure decisions on ambiguous musical cases, and architecture changes.
Status: Final. Phase 2 accepted in principle by the user on 2026-10-31, subject to the closure decisions in section 6, which are applied. CI status in section 9.
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
| Golden set of at least 100 chord and voicing cases agreed with the user passes | `crates/mc-guitar/tests/golden/chords.txt`: 235 cases, all passing (`g03_golden_set`), updated to the closure decisions (section 6); no case left flagged ambiguous | Met |
| Voicing search under 50 ms for one chord [Assumption] | Section 4: p99 3.9 ms for the heaviest chord (C9, all results kept) after the closure changes | Met |
| Property tests G-01, G-02 | `crates/mc-guitar/tests/properties.rs` (`g01_`, `g02_`, two `g03_` properties, shape round trip) | Met |
| Golden tests in standard and alternate tunings | Drop D, DADGAD, open G, open D, half-step down and capo cases in the golden set | Met |

Milestone M2 demo ("CLI lists voicings for chords in standard and drop D"): `mc-cli chord C` lists `x32010` first; `mc-cli chord D --tuning drop-d` lists `000232` first. `mc-cli identify x32210` prints C6 (preferred), then Am/C, with tones, missing tones and explanations; `mc-cli identify x355xx` reports insufficient pitch content.

## 3. Golden set composition

| Category | Cases |
|---|---|
| Symbol parsing and canonical display, aliases (`Δ7`, `ø7`, `°`, `°7`, `min`, `+`), slash chords | 32 |
| Rejected symbols (deferred qualities, power chord `C5`, unknown, malformed) | 16 |
| Chord spelling (`C E G Bb`, `C Eb Gb Bbb` ...) | 11 |
| Identification: triads, sevenths, sus, sixth, add9, 9, every quality | 54 |
| Identification: inversions and slash chords | 16 |
| Identification: omitted fifth, insufficient pitch content and invalid voicings | 16 |
| Identification: alternate tunings and capo | 12 |
| Constraint boundaries (span 4/5, fret 12/13, fingers, open strings, min fret, string count) | 20 |
| Voicing search (contains, excludes, first, count; constraint options; tunings; capo) | 44 |
| Retune planning | 9 |
| Scale and note positions | 5 |

Every candidate list is the full ranked list, not a single name. For every search result the runner also checks G-02 (constraints) and G-03 (the result is identified back as the requested chord). A negative control (one wrong expectation in each of `id` and `find`) failed the test as expected.

## 4. Benchmarks (reference machine, release)

After the closure changes (full `cargo bench --workspace` run):

| Query | p50 | p99 | Budget |
|---|---|---|---|
| `chord_voicings` C, all results | 0.51 ms | 0.62 ms | 50 ms |
| `chord_voicings` C9, all results | 3.0 ms | 3.9 ms | 50 ms |
| `chord_voicings` Cdim7, all results | 0.41 ms | 0.61 ms | 50 ms |
| `identify` x02010 (criterion mean) | 3.8 µs | | |

A separate guitar-only run on a busy machine showed p99 tails up to 9.4 ms (C) with unchanged p50; the full run above did not reproduce them. The Phase 1 command and format benchmarks were rerun in the same session and stay within their budgets (note edit p99 11.4 µs typical, 21.8 µs large; open large 280 ms against 2 s).

Measured in `crates/mc-app/benches/commands.rs` (`guitar_queries`), compiled in CI (FF-21).

## 5. Phase 2 conventions

These fill in details the approved conventions did not fix. The user kept them at closure as explicit Phase 2 domain and search conventions, not universal music-theory rules. All are configurable defaults or documented rules; none is an invariant. I-9 was changed by decisions C and D (section 6).

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
| I-9 | Display spelling without key or harmonic context: C, C#, D, Eb, E, F, F#, G, Ab, A, Bb, B for chord roots and slash basses. Chord-tone spellings from the root are kept for tone lists (Cdim7: C Eb Gb Bbb) | key-aware spelling, later |

## 6. Closure decisions on the ambiguous cases (user, 2026-10-31)

All 14 cases flagged `?` are resolved; the golden set has no remaining flags. Applying the decisions changed exactly 17 golden expectations, all listed below; no other case changed.

| Group | Decision | Implementation | Golden result after closure |
|---|---|---|---|
| A. Root in bass vs complete slash chord | Without harmonic context, prefer the interpretation whose root is the bass, then the more direct name; keep all valid candidates | Ranking key order is now: root in bass, fewer missing tones, fewer chord tones, quality order | `x32210`: C6 (no 5), Am/C; `322000`: G6 (no 5), Em/G; `1x0231`: F6 (no 5), Dm/F; `x-3-x-2-4-x`: Cm6 (no 5), Adim/C |
| B. Same notes, both complete | Kept (consistent with A) | Unchanged | `x02010`: Am7, C6/A; `x32253`: C6, Am7/C; `3x2210`: Am7/G, C6/G |
| C. Theoretical bass spelling | No theoretical enharmonics without context; slash basses use the canonical sounding-pitch display spelling | `Chord::bass` in a candidate is the display spelling of the bass pitch class; `CandidateTone::name` keeps the interval spelling (B#), so pitch representation and display stay separate | `x3211x`: Caug, Eaug/C, Abaug/C; `x-3-4-2-4-x`: Cdim7, Ebdim7/C, F#dim7/C, Adim7/C; half-step-down `022000`: Ebm, F#6/Eb (no 5) |
| D. No-key root spelling | Deterministic, guitarist-friendly canonical spelling; C#m over Dbm. Display convention only | `NoteName::default_for` table: C, C#, D, Eb, E, F, F#, G, Ab, A, Bb, B | `x46654`: C#m, E6/C# (no 5). Consequence of one deterministic table: `x46664` now reads C# (was Db) |
| E. Insufficient pitch content | At least three distinct pitch classes for a chord name; otherwise an explicit insufficient result | `IdentifyStatus::{Identified, NoMatch, Insufficient { distinct }}` and `MIN_DISTINCT_PITCH_CLASSES = 3`; the CLI prints "insufficient pitch content". Voicing search applies the same minimum so every result stays nameable (G-03) | `x32x1x`, `x355xx`, `x-3-4-5-x-x`, `xxxxx0`, `xxxxxx`, drop-d `000xxx`: insufficient. Search for C no longer returns `x32x1x` |

Note on group A: the closure text names "Am7/C" as the alternative for `x32210`. That shape (C E A C E) has no G, so the valid alternative is Am/C; Am7/C is the alternative for `x32253` (C E A C G), which already ranked C6 first.

Consequence of E for search: a triad voicing that omits its fifth has only two pitch classes, so search no longer returns triads without a fifth; four- and five-note chords without a fifth (for example `x3x45x` Cmaj7, `x3233x` C9) are still returned and named. Dyad, power chord and interval identification are deferred to a later phase.

## 7. Fitness functions

23 of 33 active; `current_phase = 2`. New: FF-25 and FF-26 (both advisory reports, `cargo xtask fitness coupling` and `cargo xtask fitness module-cycles`, wired into CI).

| Id | Phase 2 evidence |
|---|---|
| FF-01 | 6 crates checked; `mc-guitar` depends on `mc-theory` only; `mc-app` gained `mc-guitar` (already allowed by the matrix in dependency-rules section 1) |
| FF-03, FF-04 | `mc-guitar` has the domain `clippy.toml` and compiles for wasm32; the golden set is embedded with `include_str!` (no file I/O in domain tests) |
| FF-13, FF-14 | The random command generator gained a retune operation (all presets, capo 0 to 2, both modes) |
| FF-22 | Snapshots regenerated for mc-theory, mc-score, mc-app; `mc-guitar.txt` added; weekly job covers mc-guitar. Closure added `IdentifyStatus`, `MIN_DISTINCT_PITCH_CLASSES` and `Identification::status` (snapshots regenerated) |
| FF-24 | 41 of 55 rule ids have tests (was 38); G-01, G-02, G-03 covered |
| FF-25 | Crate fan-in/fan-out: mc-theory 4/0, mc-score 2/1, mc-guitar 1/1, mc-project 2/2, mc-app 1/4, mc-cli 0/2 |
| FF-26 | One existing cycle reported: `mc-score` modules `score` and `validate` (Phase 1 code; advisory). Not changed in Phase 2 |
| FF-28 | 0 cognitive-complexity warnings; 7 functions over 100 lines (advisory; new: the golden runner's case dispatcher) |

Workspace tests: 165 passing after closure (including xtask self-tests), 0 failing. Closure added unit tests for insufficient pitch content and slash-bass display spelling, and the G-03 property now checks the status.

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

Local runs of every CI step passed after the closure changes: fmt, clippy `-D warnings`, workspace tests, benchmark build and full benchmark run, `xtask fitness all`, FF-24/25/26 reports, Mermaid, cargo-deny. GitHub Actions passed on `4c3780d` and `93e5f48` (branch `phase-2-guitar`); the closure commit and the merge into `main` are verified on their pushes.

## 10. Not done in Phase 2

`CagedShape` and the fingering optimizer (later phases), diagnostics D-02 (unplayable stretch in a beat) and D-04 (note outside practical range after retune), which need the editor (Phase 3). A retune that would move a note outside the fret range is rejected as a whole (H-04), not reported per note.

## 11. Closure

Decisions (user, 2026-10-31): Phase 2 accepted in principle; conventions I-1 to I-8 kept as Phase 2 conventions (span 4 as an inclusive four-fret window, barres where the search model permits them, the sus2/sus4 fifth requirement, rootless voicings not named); groups A to E decided as in section 6. No feature, UI, audio, AI, tuner work or chord vocabulary was added at closure. Phase 2 closes when `phase-2-guitar` is merged into `main` with CI passing. Next phase per the roadmap: Phase 3 (Flutter shell, bridge and desktop TAB editor), on a new branch.
