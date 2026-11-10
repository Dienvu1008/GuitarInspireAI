# Domain Invariants

Purpose: Catalogue the rules that protect domain correctness, classify them, and state owner, enforcement point and test for each.
Status: Accepted. D-01 (underfull and overfull bars as diagnostics) confirmed by the user (Q-09), with the effective bar duration rule in section 4a ([ADR-020](decisions/ADR-020-invariants-vs-diagnostics.md)).
Related documents: [bounded-contexts.md](bounded-contexts.md), [score-model-and-undo.md](score-model-and-undo.md), [project-format.md](project-format.md), [ai-music-architecture.md](ai-music-architecture.md), [fitness-functions.md](fitness-functions.md).

## 1. Categories

| Category | Meaning | Effect on violation | Where enforced |
|---|---|---|---|
| Hard invariant (H) | Holds in every in-memory and committed state | Construction rejected; occurrence is a bug | Private-field newtypes, smart constructors, aggregate methods, DTO `TryFrom`, validating builders |
| Editing constraint (E) | Precondition of a command | Command rejected with a typed error; state unchanged | Command handlers in `mc-app` and aggregate methods |
| Configurable constraint (C) | Parameter of a check or search, with a documented default | Depends on use: rejection for inputs, filter for searches | Configuration structs passed to services |
| Diagnostic (D) | Well-formedness issue allowed in committed documents | Reported; never blocks save | `ScoreDiagnostics`, exporters, playback warnings |

Temporary editing states (typing "1" then "2" for fret 12, a drag in progress, a half-entered tuplet) live in Dart UI state. They never reach the domain. The domain only sees complete commands.

## 2. Music Theory (`mc-theory`)

| Id | Rule | Cat. | Enforcement | Test |
|---|---|---|---|---|
| T-01 | Durations and positions are positive rationals; no floats in the music model | H | `Rational` and `NoteValue` constructors | Unit, proptest arithmetic |
| T-02 | Alteration in -2..=2 (v1); microtones are expressed by bends or cents offsets, not alterations | H | `Alter` constructor | Unit |
| T-03 | `SpelledPitch` is not bounded by MIDI range | H (by design) | Type has no MIDI dependency | Unit: C-1 and C9 representable |
| T-04 | A tuning has at least one string; each open string is a valid spelled pitch | H | `Tuning::new` | Unit |
| T-05 | Tempo is greater than 0 | H | `Tempo::new` | Unit |
| T-06 | Tempo within 20 to 400 BPM for user input (default range; covers practical music while catching typos) | C | Command validation | Unit |
| T-07 | Tuning string count within 1 to 12 for user input (covers 4 to 9 string guitars, basses, 12-string as 12 courses simplification) | C | Command validation | Unit |

## 3. Composition (`mc-score`)

| Id | Rule | Cat. | Enforcement | Test |
|---|---|---|---|---|
| H-01 | Track, beat and note ids are unique within a score | H | Aggregate id allocation; builder | Proptest |
| H-02 | Every track's bar list has the same length as the master bar list (v1 model limit, see section 6) | H | Aggregate methods that insert or remove master bars update all tracks | Proptest |
| H-03 | Fretted note: string index within the track's string count | H | Aggregate method; DTO `TryFrom` | Unit, proptest |
| H-04 | Fretted note: fret within 0..=fret_count of the track | H | Same | Unit |
| H-05 | Within one beat of one voice, at most one note per string | H | `Beat` mutation methods | Unit, proptest |
| H-06 | Tie target is an existing preceding note on the same string with the same fret (fretted) or same pitch (pitched) | H | Tie command; re-checked when the preceding note changes (tie is removed and an event emitted) | Unit, proptest |
| H-07 | Beat durations are valid note values (base value, 0 to 3 dots, tuplet ratio from supported set) | H | `NoteValue` constructor | Unit |
| H-08 | Note value kind matches track instrument kind (fretted notes only in fretted tracks) | H | Aggregate method | Unit |
| H-09 | Score has at least one master bar and each bar has at least one voice | H | Builder; delete commands | Unit |
| H-10 | Capo plus fret does not exceed fret_count | H | Capo command and note commands | Unit |
| H-11 | Tempo map entries are ordered by position and unique per position | H | Tempo map methods | Unit |
| H-12 | Repeat structure is valid: no nested repeats; repeat counts 2..=16; alternate ending numbers 1..=16, strictly increasing, not above the section's repeat count, and placed inside a repeated section or directly after its end bar. A repeat end without a start repeats from the score start (or from after the previous repeat) | H | Master bar commands; full validation on load | Unit (mc-score repeats tests, h12_) |
| E-01 | `expected_revision`, if supplied, equals the current revision | E | Session | Unit |
| E-02 | Target track, bar, beat or note exists | E | Handler | Unit |
| E-03 | Cannot delete the last track or the last master bar | E | Handler | Unit |
| E-04 | Insertion position is within the voice | E | Handler | Unit |
| E-05 | Selection-based commands require a non-empty selection | E | Handler | Unit |
| C-01 | Fret count per track in 1 to 36 (covers extended-range guitars) | C | Track command | Unit |

Why H-05 is per beat and per voice: a beat is one attack event, so two different frets on one string at one attack are physically meaningless. Two voices may place notes on the same string at the same time (for example a held bass note in voice 2 and a melody in voice 1 on adjacent strings, or a deliberate collision). That case is a playability diagnostic (D-03), not a hard rule, so realistic polyphonic guitar writing and imported files remain representable.

## 4. Diagnostics (allowed in committed documents)

| Id | Rule | Owner | Behaviour |
|---|---|---|---|
| D-01 | Voice duration differs from the time signature (underfull or overfull) | Composition | Accepted (Q-09): both are diagnostics, like Guitar Pro red bars, and never block saving. Playback uses the effective master bar duration (section 4a), so no content is lost and all tracks stay synchronized. Layout draws as written. Exporters report. Pickup bars are explicit master bar flags and do not raise D-01 for being short |
| D-02 | Unplayable stretch in a beat (fret span beyond configured hand span) | Guitar | Reported in inspector |
| D-03 | Same string used by two voices at the same time | Guitar | Reported; playback plays both |
| D-04 | Note outside instrument practical range after retune | Guitar | Reported |
| D-05 | Sounding pitch outside MIDI 0 to 127 | Playback | Owned by the MIDI mapping, not theory. Note skipped in playback and export with a warning |
| D-06 | Tie chain broken by an import | Composition | Reported in ImportReport; tie removed |

Why D-01 is a diagnostic, not a hard invariant: editors need to insert and delete beats one at a time, which passes through underfull and overfull bars. Imports from MIDI and Guitar Pro frequently contain malformed bars. Forcing exact fill would either reject realistic files or silently rewrite user content. Alternative considered: strict fill with automatic rebarring (TuxGuitar style). Rejected as default because automatic rebarring changes content the user did not edit. It can be offered as an explicit command.

## 4a. Effective master bar duration (playback semantics of D-01)

Definitions, all in exact rationals (fractions of a whole note):

- `nominal(m)`: time signature length of master bar `m` (for example 4/4 = 1). For a master bar flagged as pickup, `nominal(m) = 0`.
- `voice_len(t, m, v)`: sum of beat durations in voice `v` of track `t` in bar `m`.
- `E(m) = max(nominal(m), max over every track t and every voice v of voice_len(t, m, v))`.
- Bar start times: `start(0) = 0`, `start(k + 1) = start(k) + E(bar at playback position k)`, computed over the repeat-expanded playback order.

Rules:

1. `E(m)` is a property of the master bar, shared by all tracks. Every track's bar `m` starts at the same `start`, so later bars stay globally synchronized.
2. All tracks count, including muted tracks and empty voices. Mute only silences output; it never changes timing.
3. An underfull voice ends early; the remainder up to `E(m)` is silence for that voice. An overfull voice plays completely; the other tracks wait (silence or held notes) until `E(m)`.
4. A pickup bar with no notes in any voice has `E(m) = 0` and is skipped in playback.
5. `E(m)` is computed by `mc-score` (`Score::effective_bar_duration`, Phase 1) and consumed by `mc-playback` (Phase 4). Layout does not use it; it draws bars as written.
6. Tempo map positions are expressed in score positions (bar index, offset within bar). The offset must be in `0 <= offset < nominal(m)` (0 only, for a pickup bar), checked by H-11. Revised in Phase 1: the earlier draft allowed offsets up to `E(m)`, but `E(m)` changes with every content edit, so a beat deletion could have invalidated the tempo map. Bounding by the nominal length keeps H-11 independent of content. A tempo change inside the overfull part of a bar is placed at the start of the next bar.

Required tests (ids used as test name prefixes):

| Id | Case | Expected |
|---|---|---|
| PB-01 | All voices exactly full | `E(m) = nominal(m)` |
| PB-02 | Track 2 voice overfull by 1/8 in bar 3 of 4/4 | `E(3) = 9/8`; bar 4 starts at `3 + 9/8` for every track |
| PB-03 | One track underfull in bar 2 | `E(2) = nominal`; that track is silent for the remainder; bar 3 start unchanged |
| PB-04 | Overfull voice in a muted track | Same `E(m)` and start times as unmuted |
| PB-05 | Two voices in one track, voice 2 longer | `E(m)` uses voice 2 |
| PB-06 | Empty pickup bar | `E = 0`, skipped; first real bar starts at 0 |
| PB-07 | Overfull bar inside a repeated section played twice | Each pass uses the same `E(m)`; start times additive |
| PB-08 | Property: for random scores, start times are non-decreasing, identical across tracks, and `E(m) >= nominal(m)` | Holds |
| PB-09 | Determinism: compiling the same snapshot twice | Identical timelines (byte-equal serialized events) |

PB-01 to PB-08 are `mc-score` tests (Phase 1). PB-09 and timeline-level variants of PB-02, PB-04 and PB-07 are `mc-playback` tests (Phase 4). Timeline compilation rules are in [audio-dsp-architecture.md](audio-dsp-architecture.md) section 3a.

## 5. Other contexts

| Id | Rule | Owner | Cat. | Enforcement | Test |
|---|---|---|---|---|---|
| G-01 | A voicing has at most one position per string | Guitar | H | `Voicing` constructor (one slot per string; `from_positions` rejects duplicates) | Unit, proptest (`g01_`) |
| G-02 | Voicing search results satisfy the supplied `VoicingConstraints` (span, fingers, open strings, fret range, sounding strings) | Guitar | C | `VoicingConstraints::check` in the search | Proptest, golden cases (`g02_`) |
| G-03 | Chord finder names only chords whose pitch classes cover every sounding note, whose root sounds, and whose missing tones are omissible perfect fifths (when allowed) | Guitar | C | `identify` | Golden set, proptest (`g03_`) |

Guitar constraint definitions (Phase 2 decisions; configurable defaults, not invariants):

- Span: highest minus lowest fretted fret plus one, so frets 1 to 4 have span 4. Open strings do not count. Default maximum 4.
- Fingers: one per fretted note, except that notes at the same fret may share one barre finger when they lie on a run of adjacent strings all fretted at that fret or higher (an open, muted or lower-fretted string breaks the run). At most one barre. Open strings take no finger. Default maximum 4.
- Fret range: fretted notes within `min_fret..=max_fret` (default 0 to 12, fret 12 included). Open strings are governed only by "allow open strings" (default yes).
- At least 3 sounding strings. Bass: the root, or the slash bass for a slash chord (inversions on request).
- Omitted fifth: only a perfect fifth, and not in sus chords (without it the quality is no longer identifiable). Allowed by default. Thirds, sevenths, sixths and ninths are never omitted; rootless voicings are not named in Phase 2.
- Minimum pitch content: a voicing is named only if it sounds at least three distinct pitch classes. Fewer (single notes, dyads, power chords) gives the explicit result "insufficient pitch content", never a chord name; voicing search returns only voicings that meet this minimum. Dyad, power chord and interval naming are deferred.
- Identification ranks all valid candidates (no harmonic context): root in the bass first, then fewer missing tones, then fewer chord tones, then quality order, then root distance above the bass. The first is the preferred name, not the only valid one (`x32210`: C6 without fifth, then Am/C).
- Display spelling without key or harmonic context: C, C#, D, Eb, E, F, F#, G, Ab, A, Bb, B for chord roots and slash basses (C#m, Eaug/C). It is a display convention; pitch identity is the pitch class, and chord-tone spellings from the root (Cdim7: C Eb Gb Bbb) are unchanged. Contextual spelling can replace it later.

These are Phase 2 domain and search conventions, not universal music-theory rules.
| PT-01 | Every step lies inside the pattern: `0 <= onset` and `onset + length(duration) <= length` (amended by ADR-034; previously "sum of step durations equals the declared pattern length") | Pattern | H | Semantic validation stage | Unit, proptest |
| PT-02 | Every selector resolves to sounded strings of the given voicing at bind time, or binding fails with a typed error | Pattern | H at bind | Bind stage | Proptest |
| PT-03 | Within one lane, steps are in onset order and do not overlap (ADR-034) | Pattern | H | Semantic validation stage | Unit, proptest |
| PT-04 | Durations are valid `NoteValue`s; onsets are non-negative rationals (ADR-034) | Pattern | H | Semantic validation stage | Unit |
| PT-05 | A `Hold` follows a strike or hold in the same lane and holds only strings the held strike plays (ADR-034). 6a-2 checks this for every voicing and without a gap; see ADR-034 section 14, pending user review | Pattern | H | Semantic validation stage | Unit |
| PT-06 | Gesture references resolve in the library; no gesture references a gesture in 6a (ADR-034) | Pattern | H | Semantic validation stage | Unit |
| PT-07 | Technique spans cover existing steps of one lane; combinations are kept and resolved by the ADR-035 precedence (ADR-034) | Pattern | H | Semantic validation stage | Unit |
| PT-08 | Gesture names are unique without case and never equal a DSL keyword (ADR-034). 6a-2 also requires letters and digits, starting with a letter; see ADR-034 section 14, pending user review | Pattern | H | Library validation | Unit |
| PT-09 | Gaps in a lane are rests, filled by the binder (ADR-034) | Pattern | Convention | Bind stage | Proptest |
| P-01 | Every asset reference resolves to an entry in the package | Project | H | Loader | Fixture tests |
| P-02 | Entry hashes match the manifest | Project | H | Loader (mismatch: typed error, offer recovery) | Corruption tests |
| P-03 | Format version is supported, and all `required_features` are known | Project | H | Loader | Version tests |
| P-04 | Revision counter is monotonic within a session; saved revision is recorded in the manifest | Project and session | H | Session | Unit |
| P-05 | Loaded content passes the same validation as commands (via `ScoreBuilder`) | Project | H | `TryFrom` DTO to domain | Round-trip suite |
| P-06 | ZIP limits and path rules hold (see project-format) | Project | H | Reader | Fuzz, malicious fixtures |
| AU-01 | Audio asset metadata: sample rate > 0, channels >= 1, duration consistent with frame count | Audio adapter | H | Asset import | Unit |
| AI-01 | Every candidate carries provenance (source hash, algorithm or model id, version, parameters, confidence if available) | Analysis | H | `Candidate::new` | Unit |
| AI-02 | Only a user `AcceptCandidate` command converts a candidate into score commands; those commands pass normal validation | Application | H | Handler; GR-04 | Integration test |
| IM-01 | All imports build the score through validating builders and produce an ImportReport listing lossy or dropped features | Import adapters | H | Adapter structure; GR-09 | Fuzz, fixtures |

## 6. Master bar alignment scope

H-02 is a hard invariant of the v1 score model, matching Guitar Pro and TuxGuitar master bar models and MusicXML measure alignment. It is not a universal musical truth: polymeter, independent per-track meters and free-time tracks violate it. The universal axis is the timeline (ticks and seconds). Future audio tracks are positioned on the time axis, not on bars. Supporting polymeter would be a major format version and an ADR.

## 7. MIDI and theory separation

`SpelledPitch` lives in `mc-theory` with no MIDI bound. `MidiNote(u8 in 0..=127)` lives in `mc-playback` with a fallible conversion `TryFrom<SpelledPitch>`. Rationale: notation and theory regularly represent pitches outside the General MIDI range (for example in transposition intermediate results), and MIDI is one output format among several.

## 8. Testing approach

- Unit tests per rule id; test names include the id (for example `h05_one_note_per_string_per_beat`).
- Proptest command sequences assert all H rules after each command (FF-13).
- Golden files for chord finder results and diagnostics.
- cargo-fuzz targets for project loading and each importer (FF-15).
- A rule-id coverage check script lists rule ids in this document that have no test with that id prefix (Advisory, FF-24).

## 9. Example trace: insert a note on string 6 fret 3

1. Dart sends `InsertNote { track, bar: 4, voice: 0, beat_id, string: 6, fret: 3 }` with `expected_revision`.
2. Session checks E-01; handler checks E-02.
3. `StringIndex::new(6)` and `Fret::new(3)` construct value objects (T-level checks).
4. `Score::insert_note` checks H-08 (fretted track), H-03 (6 strings), H-04 and H-10 (capo + 3 within fret count), H-05 (string 6 unused in this beat).
5. On success: returns `BarContentChanged { track, bar_range: 4..=4 }`. Session pushes an undo snapshot and increments revision.
6. Diagnostics re-evaluated for bar 4: D-01 if the voice is now overfull (only when a new beat was created), D-03 if voice 1 also uses string 6 at the same time.
7. Dart re-queries `tab_layout` for bar 4 and diagnostics.
