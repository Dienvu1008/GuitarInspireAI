# ADR-034 Pattern engine: the Pattern IR is the source of truth (Phase 6a)

Status: Draft (2026-11-09). The acceptance criteria in section 11 are fixed before any code. Section 13 lists the decisions the user must confirm. No implementation starts until the user approves the design and the criteria.
Date: 2026-11-09
Related documents: [guitar-pattern-engine.md](../guitar-pattern-engine.md), [bounded-contexts.md](../bounded-contexts.md) section 4, [domain-invariants.md](../domain-invariants.md) (PT-01, PT-02), [ADR-035](ADR-035-playing-techniques-in-the-score.md) (techniques in the score), [ADR-011](ADR-011-rational-time-model.md), [ADR-016](ADR-016-undo-structural-sharing-snapshots.md), [ADR-030](ADR-030-editor-debt-clipboard-and-session-marker.md), [open-questions.md](../../research/open-questions.md), [pattern-corpus-proposed.md](../pattern-corpus-proposed.md)

## Context

Phase 6a builds the pattern engine: reusable right-hand patterns (picking, strumming) bound to a chord voicing to produce TAB. The user answered Q-01 to Q-06 on 2026-11-09 and changed the approach: **the text DSL is not the main way in. A structured Pattern IR is the source of truth.** A grid editor, a gesture palette, user-defined gestures, the DSL, and later tap-to-create are all ways of producing and editing that one IR.

Findings from the code and the documents (not assumptions):

- **The crate is already planned.** `allowed-deps.toml` declares `mc-pattern` (domain, phase 6; `mc-theory`, `mc-score`, `mc-guitar`; dev `proptest`). The dependency matrix already allows `project -> pattern` and `app -> pattern` from Phase 6. `bounded-contexts.md` section 4 describes the context as Proposed.
- **Time is rational everywhere** (ADR-011): `Rational`, `NoteValue` (base, dots, tuplet ratio per beat).
- **The score cannot hold every technique the user listed.**
  - A note has `tie`, `let_ring` and `dead` only.
  - Accent, palm mute, percussive hit and stroke direction do not exist in `mc-score`, the project format, playback, TAB or MusicXML.
  - Adding them is a score-model and format change, decided separately in ADR-035. This ADR depends on it.
- **A voice is a sequence of beats.** All notes of one beat share its duration, and ties need the same string and fret in the preceding beat (H-06).
  - In fingerstyle, a bass note keeps sounding while the fingers play other strings. A single voice cannot hold that.
  - The existing score can, with two voices (thumb and fingers) or with let ring.
- **PT-01 as written ("sum of step durations equals the pattern length") contradicts the user's Q-03 decision**: steps have their own onset and duration, and lanes overlap in time. It must be amended (section 4).
- **Precedents.** Atomic insert of content into a range is done by a core command (`PasteBeats`, ADR-030). Its music rules sit in `mc-score`, and Dart only shows the result.

## Decision (proposed)

### 1. Contexts and crates

- `mc-pattern` (domain, Phase 6a) owns the IR, its validation, gestures, binding to a voicing, and the text DSL as a front-end module (`mc_pattern::text`). The DSL produces and prints IR and holds no semantics of its own.
- `mc-app` owns the pattern documents in a session: commands, undo, and the `ApplyPattern` score command.
- `mc-project` stores the pattern library in the project; this needs a format minor version, section 8.
- Dart renders the grid editor and the palette from core view models and sends commands. It decides nothing about selectors, durations, binding or techniques (GR-05).

### 2. The Pattern IR (semantics; Rust names are a proposal)

```rust
pub struct PatternDefinition {
    id: PatternId,
    name: String,
    length: Rational,                // whole notes, > 0; may span several bars
    meter: Option<TimeSignature>,    // for bar lines in the editor and the DSL only
    lanes: Vec<Lane>,                // 1..=2 in 6a (choice 13.2)
    spans: Vec<TechniqueSpan>,       // a technique over a range of steps
    meta: PatternMeta,               // description, tags, origin (grid, text, tap)
}
pub struct Lane { kind: LaneKind, steps: Vec<Step> } // LaneKind::Thumb | Fingers | Strum
pub struct Step {
    onset: Rational,                 // from the pattern start, in whole notes
    duration: NoteValue,             // written value: base, dots, tuplet ratio
    action: Action,
}
pub enum Action {
    Strike(Strike),
    Gesture(GestureRef),             // a named gesture from the library (section 6)
    Hold(Option<Vec<StringSelector>>), // continue the previous strike; None = all its strings
    Rest,
}
pub struct Strike {
    targets: Vec<StringSelector>,    // "+" joins targets; the union is struck together
    direction: Option<Direction>,    // Down | Up (ADR-035)
    techniques: TechniqueSet,        // accent, palm mute, dead, let ring, percussive (ADR-035)
}
pub enum StringSelector {
    Bass,                            // the voicing's lowest sounded string
    Alt,                             // alternate bass (rule in section 5)
    Treble(u8),                      // up to n highest sounded strings above the bass; Treble = Treble(3)
    Absolute(StringIndex),           // a physical string, 1 = highest (Q-02)
    Strum,                           // every sounded string of the voicing
    StrumRange(StringIndex, StringIndex),
}
```

- **Onset and duration are separate** (Q-03). A step's duration is how long its notes sound in its lane. The distance to the next attack is a property of the onsets.
- Snap resolution in an editor is an input aid only and is not stored.
- No floating point anywhere in the IR.
- Tuplets are written per step, as in the score (`NoteValue` tuplet ratio). A group of steps forms a tuplet when their onsets and values fill the group exactly, the same rule the MusicXML exporter uses for brackets.
- **Lanes.**
  - A lane maps to one voice of the score. In 6a, Thumb and Fingers map to voices 1 and 2, and a single Strum lane maps to voice 1.
  - Steps of one lane must not overlap. Steps of different lanes may.
  - This is how a bass note keeps sounding under finger notes (Travis) without overlap rules inside one voice.
- **Hold (`_`)** creates no attack. It continues the previous strike of its lane on the selected strings: those strings get tied notes (H-06) in the next beat, and the other strings stop. `Hold(None)` holds all of them. Holding a string that the previous strike did not play is a validation error.

### 3. Validation stages

1. **Front end** (DSL parse, grid edit, tap capture): produces IR or a typed front-end error with a source span or a cell address.
2. **Semantic** (pure, no voicing): PT-01 (amended), PT-03 to PT-09 (section 4).
3. **Bind** (`Pattern x Voicing x Chord x Instrument -> PatternFragment`): resolves selectors (PT-02), splits steps at bar lines into tied notes, and applies techniques.
4. **Apply** (`mc-app`): the fragment enters the score through one atomic command, and every score invariant holds (H-01 to H-12).

### 4. Invariants

| Id | Rule | Kind |
|---|---|---|
| PT-01 (amended) | Every step lies inside the pattern: `0 <= onset` and `onset + length(duration) <= length` | Invariant |
| PT-02 | Every selector resolves for the given voicing at bind time, or binding fails with a typed error | Invariant at bind |
| PT-03 | Within one lane, steps are in onset order and do not overlap | Invariant |
| PT-04 | Durations are positive and valid `NoteValue`s; onsets are rationals with denominators the score can hold | Invariant |
| PT-05 | A `Hold` follows a strike or another hold in the same lane, and holds only strings that the held strike plays | Invariant |
| PT-06 | Gesture references resolve within the library; gestures do not reference other gestures in 6a, so there are no cycles | Invariant |
| PT-07 | Technique spans cover existing steps of one lane. Combinations are not rejected: the sound follows the precedence of ADR-035 section 1 | Invariant |
| PT-08 | Gesture names are unique, compared without case (Unicode lower case), and never equal a DSL keyword | Invariant |
| PT-09 | Gaps in a lane are rests. The IR does not need explicit rest steps; the binder fills gaps with rests | Convention |

Errors are typed and name the step (lane, index, onset) or the source span. None is silently skipped (Q-02: an absolute string above the instrument's string count is an error).

### 5. Binding

Inputs:
- the pattern
- a voicing: one fret or muted per string
- the chord, needed for `Alt`: the root and the chord tones
- the instrument: tuning, string count, capo

The output is a `PatternFragment`: per lane, beats with notes and techniques, already split at the bar lines of the target range.

- **Bass:** the lowest sounded string of the voicing. This is the root for a plain chord and the slash bass for a slash chord (choice 13.3).
- **Alt:**
  - Among the sounded strings above the Bass and below the Treble group, take the one holding the chord's fifth.
  - Otherwise take the one holding the third.
  - Otherwise take the nearest sounded string above the Bass.
  - If there is none: `NoAlternateBass`.
  - For C `x32010` this gives string 4 (E); for G `320003` it gives string 4 (D).
- **Treble(n):** up to n highest sounded strings strictly above the Bass. If fewer exist, the ones that exist are used (Q-04). If none: `NoTrebleStrings`.
- **Absolute(s):**
  - `s > string_count` gives `StringOutOfRange`.
  - A muted string in the voicing gives `StringNotInVoicing`, unless the strike is dead or percussive, which play muted strings as dead notes (choice 13.4).
- **Strum and StrumRange:** every sounded string of the voicing, or of the range.
- **`+`:** the union of the resolved strings. A string named twice is struck once. An empty union gives `EmptyStrike`.
- **Bar lines:** a step that crosses a bar line of the target is split into tied notes. A tuplet step cannot be split, so it gives `TupletCrossesBar`.
- **Techniques:** passed to the notes and beats as defined in ADR-035. Spans are expanded to their steps. Nothing is dropped.

### 6. Gestures (user-defined names such as "Bùm", "Kịch", "XL")

- A `GestureDefinition` is a named, reusable strike template: targets, techniques, direction and accent. It is stored in the project library.
- No gesture is built in: "Bùm = Bass + Palm mute + Accent" is only an example. The palette offers the built-in selectors and techniques, and the user's own gestures.
- A step refers to a gesture by id. Renaming a gesture keeps the patterns that use it working.
- A gesture is expanded at bind time. Its content is copied into the step when the pattern is exported as text with `--expand`.
- Multi-step figures ("macros" that place several strikes in time) are a later slice. The IR keeps room for them as a `Figure` action (section 10), without changing `Step`.

### 7. Applying a pattern to the score

- `ApplyPattern { track, range: bars, pattern, voicing, chord, fit }` is one atomic core command: one undo step, one revision.
- It replaces the content of voices 1 and 2 of the bars in range (choice 13.5). Other tracks and other voices are untouched.
- **Fit policy** (Q-03: never stretch, never squeeze):
  - If the range length is a whole multiple of the pattern length, the pattern repeats.
  - Otherwise the command needs an explicit `fit`. There is no default, so the UI asks:
    - `RepeatAndCut`: the last repetition is cut at the range end, and the report lists the cut notes;
    - `WholeRepetitionsOnly`: the rest of the range is left as rests;
    - cancel.
  - A wrong length is never applied silently.
- The result reports notes written, repetitions, steps cut, and any techniques the target cannot show.

### 8. Storage

- The pattern library (patterns and gestures) is stored in the project package as a versioned DTO, validated through `mc-pattern` on load (GR-09 analogue). This is a **format minor version** change, additive and optional (project-format sections 3 and 5). It lands in slice 6a-5 with its own fixture, schema snapshot and U1/U2 behaviour for older readers.
- A user-level library across projects is out of scope.

### 9. Input methods

- **Grid editor** (first UI, 6a-5):
  - Rows are targets: Bass, Alt, Treble, strings 1 to n, Strum, and the user's gestures.
  - Columns are time.
  - The snap is chosen for the whole pattern or per beat (2, 3, 4, 6 or 8 parts per beat), and a beat divided into 3 creates triplet values.
  - A cell is placed by click; drag sets the duration.
  - Direction, accent and techniques come from a toolbar, applied to one step or to a selected run (a span).
  - Lanes are shown as colour bands.
  - Every edit is a core command, and the view model comes from the core.
- **Gesture palette:** the same toolbar plus the user's gestures, which can be created from a selected strike.
- **Text DSL (advanced, 6a-4):** a front end and printer.
  - `parse(print(ir)) == ir` for every valid IR, as a property test.
  - Keywords are case-insensitive (Q-01).
  - Numbers are absolute strings (Q-02).
  - The symbols are `-` rest, `_` hold, `>` accent and `+` together (Q-06).
  - **No implicit duration** (Q-03): every step carries a value, or a declared `unit` in force. A pattern without either is an error, never a guess.
  - `|` is a bar check and must fall on a bar line of the declared meter.
  - Note names are reserved for a later pitch selector with a prefix (`@A3`), so they cannot clash with keywords.
  - The sketch below is a proposal, not the final grammar. The final grammar is derived from the confirmed corpus.

  ```
  pattern "Alternating bass" length 1 meter 4/4
  strum: Bass:4 D Treble:4 Alt:4 D Treble:4
  ```

- **Tap-to-create (later):** taps against the metronome become onsets snapped to a user-chosen grid, then IR steps. Nothing in the IR depends on it.

### 10. Extension points kept open (no IR rewrite later)

- `Action::Figure`: multi-step macros.
- More lanes.
- A pitch selector (`@A3`).
- Per-chord-type variants.
- Strum spread timing.
- Swing and humanisation: these are a render setting, never stored as different onsets.

## 11. Acceptance criteria (fixed before implementation)

Measured on the reference machine (ADR-025). Thresholds are not lowered to match a result. A criterion for TAB, playback or MusicXML output of a technique is in ADR-035.

| Id | Criterion | How it is checked |
|---|---|---|
| PE-1 | The IR holds onset and duration per step in rationals; any valid IR round-trips through its DTO byte for byte | Unit and property tests |
| PE-2 | Semantic validation accepts every valid IR and rejects each violation of PT-01 and PT-03 to PT-08 with its typed error naming the step | One test per rule plus a property test over random IR |
| PE-3 | Binding resolves Bass, Alt, Treble(n), Absolute, Strum and StrumRange by section 5, for standard, drop D and DADGAD tunings, with and without a capo, on at least 12 voicings; every PT-02 failure (`StringOutOfRange`, `StringNotInVoicing`, `NoAlternateBass`, `NoTrebleStrings`, `EmptyStrike`, `TupletCrossesBar`) is a typed error | Table tests plus a property test: every bound note is on a string of the instrument and in the voicing, or is dead or percussive |
| PE-4 | Timing: every bound note starts at `range start + repetition offset + step onset` and lasts its step duration, split at bar lines into tied notes that sum exactly; a hold produces ties only on the held strings | Property test comparing the bound fragment with the IR |
| PE-5 | Lanes: a bass note keeps sounding for its full duration while the other lane attacks (two voices); playback has the bass note-off at the bass duration, not at the next finger attack | Bind test plus a playback timeline test |
| PE-6 | Techniques, direction and accent from steps, spans and gestures reach the score exactly once per note or beat, as ADR-035 defines; nothing is dropped | Property test |
| PE-7 | `ApplyPattern` is atomic: on any error nothing changes; on success there is one revision and one undo step; undo restores the score byte for byte | Session tests |
| PE-8 | Fit: a whole multiple repeats with no prompt; any other length is rejected without an explicit fit; `RepeatAndCut` reports exactly the notes it cut; nothing is stretched or squeezed | Session tests |
| PE-9 | Gestures: a gesture used by several patterns binds identically in each; renaming it changes no bound result; unknown and duplicate names are typed errors; a user name equal to a keyword is rejected | Unit tests |
| PE-10 | DSL: `parse(print(ir)) == ir` for random valid IR; every corpus pattern parses to its IR; a step without a value and without a `unit` is an error with a span; keywords are case-insensitive; a bar check off a bar line is an error | Property and golden tests |
| PE-11 | **Corpus:** every pattern of the corpus **confirmed by the user** binds to its expected TAB on its stated voicings, and its playback timeline matches its golden; proposed but unconfirmed patterns are never used as goldens | Golden tests |
| PE-12 | No music rule in Dart: the grid editor and palette send commands and render core view models; FF-05 and FF-01 pass with only the planned edges | Review, fitness functions |
| PE-13 | Library storage: patterns and gestures survive save and reopen byte for byte; an older reader sees the library as unknown content (U1) and protects the file (U2); a new fixture and schema snapshot are added; released fixtures are untouched | `mc-project` round-trip and fixture tests |
| PE-14 | Grid editor: placing, moving, resizing and deleting steps, choosing the snap per beat (including 3), applying a technique to a selected run, and creating a gesture from a strike, all work by mouse and keyboard, with semantic labels | Widget tests, Windows integration test |
| PE-15 | Binding and applying a 4-bar pattern to 64 bars takes at most 20 ms in release on the reference machine | Benchmark |

## 12. Implementation slices (proposed order)

| Slice | Content | Criteria |
|---|---|---|
| 6a-1 | Techniques in the score: model, commands, format minor version, playback, TAB, MusicXML (ADR-035) | ADR-035 criteria |
| 6a-2 | `mc-pattern`: IR, semantic validation, gestures (no storage yet) | PE-1, PE-2, PE-9 |
| 6a-3 | Binding and `ApplyPattern` with fit policy, through `mc-app` and FFI; a minimal "apply pattern" dialog for one built-in sample | PE-3 to PE-8, PE-15 |
| 6a-4 | Text DSL front end and printer; confirmed corpus as goldens | PE-10, PE-11 |
| 6a-5 | Grid editor, gesture palette, pattern library storage in the project | PE-12 to PE-14 |
| later | Figures (macros), tap-to-create, more lanes, strum spread | Own criteria |

The corpus must be confirmed by the user before 6a-4 makes it golden. 6a-1 to 6a-3 use their own synthetic tests.

## 13. Decisions for the user (before any code)

1. **Two ADRs.** The pattern engine (this ADR) and the new score techniques (ADR-035) are separate, because ADR-035 changes the score model, the format, playback, TAB and MusicXML for every user, patterns or not. Proposed: yes, and 6a-1 implements ADR-035 first.
2. **Lanes.**
   - Proposed: at most 2 lanes in 6a (Thumb and Fingers, or a single Strum lane), mapped to voices 1 and 2.
   - Alternative: one lane only, with let ring for sustained bass. This is simpler, but a bass note would then ring to the end of the bar instead of for its written duration.
3. **Bass:**
   - Proposed: the lowest sounded string of the voicing.
   - Alternative: the lowest string holding the chord root, which differs only for slash chords and inversions.
4. **Muted strings in a dead or percussive strike:** proposed, they may be struck (a muted string makes a dead sound). Alternative: only sounded strings.
5. **What `ApplyPattern` replaces:**
   - Proposed: voices 1 and 2 of the bars in range.
   - Alternative: all voices of those bars.
   - Alternative: insert instead of replace.
6. **Fit:** proposed, no default for a non-multiple length; the UI asks every time (`RepeatAndCut` or `WholeRepetitionsOnly`). Alternative: remember the last choice per session.
7. **Source of the voicing and chord in 6a:**
   - Proposed: one voicing per application, chosen in the chord finder or taken from the beat under the cursor.
   - A chord track (a different chord per bar) is a later slice.
8. **Corpus:** the user reviews [pattern-corpus-proposed.md](../pattern-corpus-proposed.md). Each pattern is confirmed, corrected or dropped before it becomes a golden (PE-11).

## Alternatives

- **DSL as the stored form.** Rejected by the user: one syntax would own the data, and the grid editor would have to round-trip through text.
- **Equal division of the bar by step count.** Rejected by the user (Q-03): durations would be guessed.
- **One lane with per-note durations inside a beat.** Rejected: the score's voice model cannot hold it, and it would need a score change bigger than lanes.
- **Built-in onomatopoeic keywords (Bùm, Kịch).** Rejected by the user: names belong to each player, so they are gestures.
- **Patterns as plain beat lists in the score** (copy and paste). Rejected: the pattern would not be bound to a voicing and could not be reused.

## Trade-offs

- Two lanes cover thumb-and-fingers styles. Patterns with three independent rhythmic layers need more lanes later.
- No default fit means one extra question when the length is not a multiple. This is deliberate, so no notes are lost silently.
- A gesture copied into text with `--expand` loses its link to the library. This is the user's choice per export.

## Consequences

- New crate `mc-pattern` (edges already declared).
- `mc-app`: pattern session commands, `ApplyPattern`, fit policy, report.
- `mc-project`: library DTO and a minor format version (6a-5).
- `mc-ffi` and Dart: pattern view models, grid editor, palette, apply dialog.
- `domain-invariants.md`: PT-01 amended, PT-03 to PT-09 added. `bounded-contexts.md` section 4 and `guitar-pattern-engine.md` are updated when this ADR is accepted.
- `open-questions.md`: Q-01 to Q-06 are recorded as decided, with the user's changes.

## Related fitness functions

FF-01, FF-03, FF-04 (`mc-pattern` builds for wasm32), FF-05, FF-11 and FF-32 (format), FF-12, FF-19, FF-22, FF-34.

## Reconsideration conditions

- Patterns that need more than two independent lanes.
- A chord track.
- A user request for stored swing or humanised timing.
- An IR need that would change `Step`, which would need a new ADR.
