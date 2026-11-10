# ADR-034 Pattern engine: the Pattern IR is the source of truth (Phase 6a)

Status: Accepted (2026-11-09).
- The acceptance criteria in section 11 were fixed before any code.
- The user reviewed revision 1 (`8e3c8ee`) and revision 2 (`a20dbe3`). The user approved revision 2 with the four clarifications and two amendments recorded in section 13.

Date: 2026-11-09
Related documents: [guitar-pattern-engine.md](../guitar-pattern-engine.md), [bounded-contexts.md](../bounded-contexts.md) section 4, [domain-invariants.md](../domain-invariants.md) (PT-01 to PT-09), [ADR-035](ADR-035-playing-techniques-in-the-score.md), [ADR-011](ADR-011-rational-time-model.md), [ADR-016](ADR-016-undo-structural-sharing-snapshots.md), [ADR-020](ADR-020-invariants-vs-diagnostics.md), [ADR-030](ADR-030-editor-debt-clipboard-and-session-marker.md), [open-questions.md](../../research/open-questions.md), [pattern-corpus-proposed.md](../pattern-corpus-proposed.md)

## Context

Phase 6a builds the pattern engine: reusable right-hand patterns (picking and strumming) bound to a chord voicing to produce TAB. The user answered Q-01 to Q-06 on 2026-11-09 and set the approach: **the text DSL is not the main way in; a structured Pattern IR is the source of truth.** A grid editor, a gesture palette, user-defined gestures, the DSL and, later, tap-to-create all produce and edit that one IR.

Findings from the code and the documents (not assumptions):

- **The crate is planned.**
  - `allowed-deps.toml` declares `mc-pattern`: domain, phase 6, using `mc-theory`, `mc-score` and `mc-guitar`, with `proptest` as a dev dependency.
  - The dependency matrix allows `app -> pattern` and `project -> pattern` from Phase 6, but `allowed-deps.toml` does not list those two edges yet. They are added in the slices that need them (6a-3 and 6a-5).
- **Time is rational everywhere** (ADR-011): `Rational`, and `NoteValue` (base, dots and a tuplet ratio per beat).
- **The score cannot hold every technique the user listed.**
  - A note has only `tie`, `let_ring` and `dead`.
  - Accent, palm mute, string-percussive and stroke direction are added by ADR-035, which this ADR depends on.
- **A voice is a sequence of beats.** All notes of a beat share its duration. A tie needs the same string and fret in the preceding beat (H-06). A bass note that keeps sounding while the fingers play needs a second voice.
- **Bar durations.**
  - A master bar's nominal duration is its time-signature length, and a pickup bar's nominal duration is 0 (`Score::nominal_bar_duration`).
  - The effective duration depends on content (D-01). This ADR uses nominal durations only.
- **PT-01 as written** ("sum of step durations equals the pattern length") contradicts Q-03 (independent onsets, overlapping lanes). It is amended in section 4.
- **Precedent:** inserting content into a range atomically is done by a core command (`PasteBeats`, ADR-030).

## Decision (proposed)

### 1. Contexts and crates

- `mc-pattern` (domain, Phase 6a) owns the IR, its validation, gestures, binding to a voicing, and the text DSL as a front-end module (`mc_pattern::text`). The DSL produces and prints IR; it has no semantics of its own.
- `mc-app` owns pattern documents in a session (commands, undo) and the `ApplyPattern` score command. The `app -> pattern` edge is added in 6a-3.
- `mc-project` stores the pattern library. The `project -> pattern` edge and format 1.2 are added in 6a-5 (section 8).
- Dart renders the grid editor, the palette and the preview from core view models and sends commands. It decides nothing about selectors, durations, binding, fit or techniques (GR-05).

### 2. The Pattern IR (semantics; the Rust names are a proposal)

```rust
pub struct PatternDefinition {
    id: PatternId,
    name: String,
    length: Rational,                // whole notes, > 0; may span several bars
    meter: Option<TimeSignature>,    // if set, target bars must use it (section 7)
    lanes: Lanes,                    // Strum(Lane) or ThumbFingers(Lane, Lane); at most 2 lanes
    spans: Vec<TechniqueSpan>,       // one technique over a run of steps of one lane
    meta: PatternMeta,               // description, tags, origin (grid, text, tap)
}
pub struct Lane { steps: Vec<Step> }
pub struct Step {
    onset: Rational,                 // from the pattern start, in whole notes
    duration: NoteValue,             // written value: base, dots, tuplet ratio
    action: Action,
}
pub enum Action {
    Strike(Strike),
    Gesture(GestureRef),             // a named gesture from the library (section 6)
    Hold(Option<Vec<StringSelector>>), // continue the previous strike; None = all its strings
    Rest,                            // a true rest: no attack, no motion
}
pub struct Strike {
    targets: Vec<StringSelector>,    // "+" joins targets; the union is struck together
    direction: Option<Direction>,    // Down | Up (ADR-035 stroke)
    techniques: TechniqueSet,        // accent, palm mute, dead, let ring, string-percussive (ADR-035)
}
pub enum StringSelector {
    Bass,                            // lowest sounded string of the voicing
    Alt,                             // alternate bass (section 5)
    Treble(u8),                      // up to n highest sounded strings above the Bass; Treble = Treble(3)
    Absolute(StringIndex),           // a physical string, 1 = highest (Q-02); must be sounded
    Strum,                           // every sounded string
    StrumRange(StringIndex, StringIndex), // the sounded strings in the range
}
```

- **Onset and duration are separate** (Q-03).
  - A step's duration is how long its notes sound in its lane. The distance to the next attack comes from the onsets.
  - Snap resolution is an editing aid and is not stored. No floating point anywhere.
- **Tuplets.**
  - A step carries its tuplet ratio, as in the score.
  - Steps form a tuplet group when their onsets and values fill the group exactly, the same rule as the MusicXML exporter's brackets.
- **Lanes** (at most two in 6a; no third lane):
  - `Strum` maps to voice 1.
  - `ThumbFingers` maps the thumb lane to voice 1 and the fingers lane to voice 2.
  - Steps of one lane never overlap. Steps of different lanes may overlap, which is how a bass note keeps sounding under the finger notes.
- **Hold (`_`)** creates no attack.
  - It continues the previous strike of its lane on the selected strings, which get tied notes (H-06). The other strings stop.
  - `Hold(None)` holds all of them.
- **Rest** is silence with no motion. A silent stroke motion (a "ghost" strum) is not a rest. If it is needed later, it becomes its own action `StrokeOnly(Direction)` (section 10).

### 3. Validation stages

1. **Front end** (DSL parse, grid edit, tap capture): produces IR, or a typed error with a source span or a cell address.
2. **Semantic** (pure, no voicing): PT-01 and PT-03 to PT-09 (section 4).
3. **Bind** (`Pattern x Voicing x Chord x Instrument -> PatternFragment`): selectors (PT-02, section 5), techniques (ADR-035).
4. **Place** (target range): fit, splitting at bar lines and range checks (section 7).
5. **Apply** (`mc-app`): one atomic command; every score invariant holds (H-01 to H-12).

### 4. Invariants

| Id | Rule | Kind |
|---|---|---|
| PT-01 (amended) | Every step lies inside the pattern: `0 <= onset` and `onset + length(duration) <= length` | Invariant |
| PT-02 | Every selector resolves to sounded strings of the given voicing at bind time, or binding fails with a typed error | Invariant at bind |
| PT-03 | Within one lane, steps are in onset order and do not overlap | Invariant |
| PT-04 | Durations are valid `NoteValue`s; onsets are non-negative rationals | Invariant |
| PT-05 | A `Hold` follows a strike or another hold in the same lane, and holds only strings the held strike plays | Invariant |
| PT-06 | Gesture references resolve in the library; gestures do not reference gestures in 6a, so there are no cycles | Invariant |
| PT-07 | Technique spans cover existing steps of one lane. Combinations are not rejected: the sound follows the ADR-035 precedence, and every flag is kept | Invariant |
| PT-08 | Gesture names are unique without case (Unicode lower case) and never equal a DSL keyword | Invariant |
| PT-09 | Gaps in a lane are rests. The binder fills them, and the IR needs no explicit rest steps | Convention |

Errors are typed and name the step (lane, index, onset), the source span or the bar. Nothing is skipped silently.

### 5. Binding

Inputs:
- the pattern;
- one voicing per application: a fret or "muted" per string (choice 13.7);
- the chord (root and chord tones, needed for `Alt`);
- the instrument (tuning, string count, capo).

The output is a `PatternFragment` of beats and notes per lane, in pattern time, with techniques.

**Only sounded strings are ever selected** (choice 13.4). No selector produces a note on a string the voicing mutes. A string-percussive note keeps its fretted position and pitch (ADR-035).

Regions, computed from the voicing alone and independent of which selectors a strike uses:
- **S:** the sounded strings, from lowest to highest.
- **Bass B:** the lowest string of S. This is the actual bass, so a slash chord or an inversion keeps it (choice 13.3).
- **Upper region U:** the 3 highest strings of S strictly above B, or fewer if S has fewer. U is fixed for the voicing and does not depend on whether the strike contains `Treble`.

Selectors:
- **Bass:** B.
- **Alt:**
  - The candidates are the strings of S strictly above B and not in U.
  - Choose the candidate holding the chord's fifth. Otherwise the one holding the third. Otherwise the lowest candidate.
  - No candidate gives `NoAlternateBass`.
  - Examples:

    | Voicing | Result |
    |---|---|
    | C `x32010` | candidates {4}: E, the third, so string 4 |
    | G `320003` | candidates {5, 4}: B and D; D is the fifth, so string 4 |
    | Am `x02210` | candidates {4}: E, the fifth, so string 4 |
    | D `xx0232` | U = {3, 2, 1}, no candidates: `NoAlternateBass` |
- **Treble(n):** up to n highest strings of S strictly above B, or fewer if there are fewer (Q-04). `Treble` = `Treble(3)`. None gives `NoTrebleStrings`. n may exceed 3: `Treble(4)` on C `x32010` gives {4, 3, 2, 1}.
- **Absolute(s):** `s > string_count` gives `StringOutOfRange`; a muted s gives `StringNotInVoicing`.
- **Strum:** S. **StrumRange(a, b):** the strings of S between a and b; none gives `EmptyStrike`.
- **`+`:** the union, with each string struck once. An empty result gives `EmptyStrike`.

Techniques, direction and accent from steps, spans and gestures are attached to every resulting note or beat, as defined in ADR-035. Nothing is dropped.

### 6. Gestures (user-defined names such as "Bùm", "Kịch", "XL")

- A `GestureDefinition` is a named strike template (targets, techniques, direction, accent) stored in the project library.
- No gesture is built in. "Bùm = Bass + Palm mute + Accent" is only an example of what a user may define.
- A step refers to a gesture by id, so a rename keeps every use working. Gestures are expanded at bind time.
- Multi-step figures (macros) are a later slice, as an `Action::Figure` (section 10).

### 7. Placing and applying a pattern

**Target range.**
- The range is a run of whole master bars `[first, last]` on one track.
- Its duration is the **sum of the nominal durations** of those bars (their time-signature lengths), never the content-dependent effective durations.
- **Pickup bars:** a range that contains a pickup bar is rejected with `PickupInRange`. A pickup's nominal duration is 0, and its intended length is not stored. Supporting pickups is a later change.
- **Mixed time signatures:**
  - If the pattern declares a `meter`, every bar of the range must have that time signature, or placement fails with `MeterMismatch { bar }`.
  - A pattern without a meter is placed continuously across bars of any time signatures. Its time runs on from bar to bar and is split at each bar line.

**Fit** (Q-03: never stretch, never squeeze).
- If the range duration is a whole multiple of the pattern length, the pattern repeats.
- Otherwise the command needs an explicit `fit`. There is no default and no remembered choice (choice 13.6):
  - **`RepeatAndCut`:**
    - Full repetitions, then the last one is cut at the range end.
    - A step that starts at or after the end is dropped.
    - A step that starts before the end and would sound past it is shortened. The remaining length is written as tied values, the same way bar lines split notes.
    - A tuplet step that would need shortening is dropped instead, because a tuplet cannot be split. The timing of every remaining step is kept, and no duration is redistributed.
    - The report lists every dropped and shortened step: lane, repetition, onset, written value and new value.
  - **`WholeRepetitionsOnly`:** whole repetitions only; the rest of the range is left as rests.
  - **Cancel:** no change.

**Bar lines.**
- A step that crosses a bar line inside the range is split into tied notes whose lengths sum exactly.
- A tuplet step that crosses a bar line fails with `TupletCrossesBar`.

**ApplyPattern** `{ track, first, last, pattern, voicing, chord, fit, overwrite: bool }`:
- It **replaces voices 1 and 2** of the track over the range. Other tracks, voices 3 and above, master bars, tempo and every other bar are kept. A voice 2 is created where the pattern uses the fingers lane, and left untouched where it does not (choice 13.5).
- **Overwrite safeguard.**
  - The core checks only the voices the pattern writes: voice 1 for a Strum pattern, voices 1 and 2 for a Thumb and Fingers pattern.
  - If a checked voice of any bar in the range holds a beat and `overwrite` is false, the core returns `WouldOverwrite { bars }` and changes nothing.
  - Existing voice 2 content never triggers the prompt for a Strum pattern, because that voice is not touched.
  - The UI shows a preview of the result and the bars to be overwritten, and asks for explicit confirmation. Only then does it resend with `overwrite: true`.
  - The rule is in the core, not in Dart.
- **No insert semantics** in 6a.
- It is one atomic command, one undo step and one revision. On any error nothing changes.
- The result reports notes written, repetitions, dropped and shortened steps, and bars overwritten.

### 8. Storage

- The pattern library (patterns and gestures) is a versioned DTO in the project package, validated through `mc-pattern` on load (GR-09 analogue).
- ADR-035 (6a-1) moves the format to 1.1. The library is the next additive change and is expected to use **format 1.2**, with its own fixture, schema snapshot and U1/U2 behaviour for older readers, unless another version change comes in between.
- A user-level library shared across projects is out of scope.

### 9. Input methods

- **Grid editor** (6a-5):
  - Rows are targets (Bass, Alt, Treble, strings 1 to n, Strum, user gestures). Columns are time.
  - The snap is set for the pattern or per beat (2, 3, 4, 6 or 8 parts; 3 creates triplets).
  - Click places a step; drag sets its duration.
  - Direction, accent and techniques apply to a step or a selected run (a span). Lanes are colour bands.
  - Every edit is a core command, and the view model comes from the core.
- **Gesture palette:** the same tools, plus the user's gestures, which can be created from a selected strike.
- **Text DSL** (advanced, 6a-4):
  - It is a front end and a printer: `parse(print(ir)) == ir` for every valid IR.
  - Keywords are case-insensitive (Q-01), and numbers are absolute strings (Q-02).
  - The symbols are `-` rest, `_` hold, `>` accent and `+` together (Q-06).
  - **No implicit duration** (Q-03): every step has a value or a declared `unit` in force; otherwise it is an error with a span.
  - `|` is a bar check against the declared meter.
  - Note names are reserved for a later prefixed pitch selector (`@A3`).
  - The grammar is derived from the confirmed corpus.
- **Tap-to-create** (later): taps become onsets on a user-chosen grid, then IR steps. Nothing in the IR depends on it.

### 10. Extension points kept open (no IR rewrite later)

- `Action::Figure` (multi-step macros).
- `Action::StrokeOnly(Direction)` (a silent stroke motion).
- More lanes.
- A pitch selector (`@A3`).
- A chord track (a voicing per bar).
- Pickup support.
- Muted-string or body percussion (a separate event model and ADR, see ADR-035).
- Strum spread.
- Swing and humanisation, as render settings and never as stored onsets.

## 11. Acceptance criteria (fixed before implementation; revision 2)

Measured on the reference machine (ADR-025). Thresholds are not lowered to match a result. Criteria about how a technique sounds or shows are in ADR-035. No criterion depends on a feature deferred in section 10.

| Id | Criterion | How it is checked |
|---|---|---|
| PE-1 | The IR holds onset and duration per step in rationals; every valid IR round-trips through its DTO byte for byte | Unit and property tests |
| PE-2 | Semantic validation accepts every valid IR and rejects each violation of PT-01 and PT-03 to PT-08 with its typed error naming the step | One test per rule plus a property test over random IR |
| PE-3 | Binding follows section 5 exactly: the four Alt examples (C, G, Am, D) give the stated results; Bass, Treble(n) (n = 1 to 5), Absolute, Strum and StrumRange resolve as defined on at least 12 voicings in standard, drop D and DADGAD tunings, with and without a capo; every bound note is on a sounded string of the voicing; each failure (`StringOutOfRange`, `StringNotInVoicing`, `NoAlternateBass`, `NoTrebleStrings`, `EmptyStrike`) is a typed error | Table tests plus a property test over random voicings and patterns |
| PE-4 | Timing: in a range of whole repetitions, every note starts at `range start + repetition offset + step onset` and lasts its step duration; notes crossing bar lines are split into tied notes that sum exactly; a hold ties only the held strings; a tuplet step crossing a bar line is rejected | Property test comparing the placed fragment with the IR |
| PE-5 | Two lanes: a thumb note sounds for its written duration while the fingers lane attacks; in the compiled playback timeline its note-off is at its own duration, not at the next finger attack | Bind test plus a timeline test |
| PE-6 | Every technique, direction and accent from steps, spans and gestures reaches each resulting note or beat exactly once, with every flag kept | Property test |
| PE-7 | `ApplyPattern` replaces only voices 1 and 2 of the target track in the range; other tracks, voices 3 and above, master bars and tempo are byte-identical before and after; with existing content in a voice the pattern writes and `overwrite: false` it returns `WouldOverwrite` with the exact bars and changes nothing; a Strum pattern applied over bars whose only content is in voice 2 applies without `WouldOverwrite` and leaves voice 2 byte-identical; on success there is one revision and one undo step, and undo restores the score byte for byte | Session tests and a property test |
| PE-8 | Range and fit: the range duration is the sum of nominal bar durations; a whole multiple repeats with no prompt; any other length is refused without an explicit fit; `RepeatAndCut` reports exactly the dropped and shortened steps (and shortens with tied values, dropping tuplets); `WholeRepetitionsOnly` leaves the remainder as rests; a pickup in range gives `PickupInRange`; a declared meter that differs from a bar gives `MeterMismatch`; a pattern without a meter crosses mixed time signatures continuously | Session tests |
| PE-9 | Gestures: a gesture used by several patterns binds identically in each; a rename changes no bound result; unknown and duplicate names are typed errors; a name equal to a keyword is rejected | Unit tests |
| PE-10 | DSL: `parse(print(ir)) == ir` for random valid IR; every confirmed corpus pattern parses to its IR; a step without a value and without a `unit` is an error with a span; keywords are case-insensitive; a bar check off a bar line is an error | Property and golden tests |
| PE-11 | **Corpus:** each pattern the user has confirmed (currently 1, 2, 3, 4, 5, 6, 8, 9, 10 of the corpus document, with the user's corrections) binds to its expected TAB on its stated voicing and matches its playback timeline golden; unconfirmed patterns (currently 7) are not goldens | Golden tests |
| PE-12 | No music rule in Dart: editor, palette, preview and dialogs send commands and render core view models; FF-01 (with only the planned new edges) and FF-05 pass | Review, fitness functions |
| PE-13 | Library storage (format 1.2): patterns and gestures survive save and reopen byte for byte; the `mc-cli` built from the 6a-1 commit (format 1.1) opens the 1.2 fixture, reports the library as unknown content (U1) and refuses to save over it (U2); new fixture and schema snapshot; released fixtures untouched | `mc-project` round-trip and fixture tests |
| PE-14 | Grid editor: place, move, resize and delete steps; set the snap per beat (including 3); apply a technique to a selected run; create a gesture from a strike; the apply dialog shows the preview, asks before overwriting, and asks for fit when needed; everything works by mouse and keyboard with semantic labels | Widget tests, Windows integration test |
| PE-15 | Binding, placing and applying a 4-bar pattern to 64 bars takes at most 20 ms in release on the reference machine | Benchmark |

## 12. Implementation slices (proposed order)

| Slice | Content | Criteria |
|---|---|---|
| 6a-1 | Techniques in the score (ADR-035), format 1.1 | ADR-035 TQ-1 to TQ-11 |
| 6a-2 | `mc-pattern`: IR, semantic validation, gestures (in memory) | PE-1, PE-2, PE-9 |
| 6a-3 | Binding, placing, `ApplyPattern` with fit and overwrite safeguard, through `mc-app` and FFI; a minimal apply dialog for one built-in sample pattern | PE-3 to PE-8, PE-15 |
| 6a-4 | Text DSL front end and printer; confirmed corpus as goldens | PE-10, PE-11 |
| 6a-5 | Grid editor, gesture palette, library storage (format 1.2) | PE-12 to PE-14 |
| later | Section 10 items | Own ADRs and criteria |

## 13. Decisions (user review of revision 1, 2026-11-09)

1. **Two ADRs:** confirmed. ADR-035 is implemented first (6a-1), because the score techniques must work without the pattern engine.
2. **Lanes:** confirmed. At most two: Thumb plus Fingers (voices 1 and 2), or a single Strum lane (voice 1). No third lane in 6a.
3. **Bass:** confirmed. The lowest sounded string of the voicing, not necessarily the root; slash chords and inversions keep their bass.
4. **Muted strings:** not supported. Absolute, Strum and every other selector resolve only to sounded strings. A string-percussive note keeps a fretted position and pitch. Muted-string or body percussion needs its own event model and ADR.
5. **ApplyPattern:** confirmed with safeguards. It replaces voices 1 and 2 of the selected track over the range and keeps other tracks and voices 3 and above. There is no insert. A preview and an explicit confirmation are required before overwriting existing content. It is one atomic command, one undo step and one revision.
6. **Fit:** always ask when the range is not a whole multiple, with no remembered choice. `RepeatAndCut` reports exactly what was cut or shortened; `WholeRepetitionsOnly` leaves rests; cancel changes nothing. The range duration uses nominal bar durations, and pickup and mixed time signatures follow section 7.
7. **Voicing:** confirmed. One voicing per application; a chord track is deferred.
8. **Alt:** the chord-tone preference is kept. The upper region is defined from the voicing, independent of the selectors in a strike (section 5). The C, G and Am results, and the D failure, are binding fixtures.
9. **Corpus:** reviewed pattern by pattern; the corpus document records each decision. Pattern 7 stays unconfirmed.

Final review of revision 2 (user, 2026-11-09). The user approved these clarifications:
- Ranges containing a pickup bar are rejected with `PickupInRange`.
- Every target bar must match a declared pattern meter, otherwise `MeterMismatch`.
- `RepeatAndCut` drops tuplet steps that would need shortening, keeps the timing of all remaining steps, redistributes nothing, and reports the dropped steps.
- Voice 2 is left untouched when the pattern has no Fingers lane.

The user made two amendments:
- **A.** The overwrite check covers only the voices the pattern writes (section 7, PE-7).
- **B.** Corpus pattern 9 uses `U Treble(4)` for the up-stroke on 3&.

Pattern 7 stays unconfirmed. Everything else is approved, and 6a-1 (ADR-035) begins.

## 14. Results of slice 6a-2 (2026-11-09, for user review)

**Built.** A new domain crate, `mc-pattern`.
- Dependencies: `mc-theory` and `mc-score` only; `mc-guitar` comes with binding in 6a-3. No external crate, as `allowed-deps.toml` requires.
- It holds the IR of section 2, as plain data (`PatternParts`):
  - `TechniqueSet` and `Direction` are the score's `NoteEffects` and `Stroke` (ADR-035), so binding copies them unchanged.
  - Technique spans address steps by index (`first..=last`) in one lane.
- Semantic validation covers PT-01 and PT-03 to PT-07. It returns every violation, not only the first, each with a typed error naming the lane, step index and onset, or the span.
- The in-memory `PatternLibrary` is the aggregate of patterns and gestures:
  - one id counter for both kinds; ids are never reused;
  - define, rename, change and remove operations, all atomic;
  - `resolve`, which expands gestures (the input of binding);
  - `to_parts` and `from_parts`, which revalidate everything.
- Domain events (bounded-contexts section 4) are not emitted yet: no session consumes them before 6a-3.

**Interpretations made in 6a-2, which the user should confirm or change:**
1. **PT-05 is checked for every voicing.**
   - A hold may select only strings the held strike certainly plays on any voicing. Certain containment means one of:
     - the same selector;
     - `Strum` contains everything;
     - `StrumRange` contains an `Absolute` string or a `StrumRange` inside it;
     - `Treble(n)` contains `Treble(m)` for m ≤ n.
   - Anything that depends on the voicing is rejected (`HoldNotInStrike`), for example a hold of string 1 after `Treble`. So PT-05 never fails at bind time.
   - A hold must start exactly where the previous strike or hold ends, because a gap is a rest (PT-09).
   - A partial hold narrows what the next hold can continue.
   - A hold after a gesture uses the gesture's targets, so changing a gesture revalidates every pattern that uses it.
2. **PT-08 name rules beyond "unique and not a keyword":**
   - A name is letters and digits only and starts with a letter, so the 6a-4 DSL can always print and re-read it. "Bùm", "Kịch", "XL" and "Đàn" are accepted; "Kịch-2", "a b" and "4x" are not.
   - The comparison is Unicode lower case, as written in PT-08.
   - There is no Unicode normalization, because no allowed dependency provides it. A decomposed spelling (a letter followed by a combining accent) is rejected, since a combining mark is not a letter, rather than silently differing from the composed spelling.
3. **The reserved keyword list is provisional.** It is `bass alt treble strum thumb fingers d u pm sp unit meter`: the words of the confirmed corpus sketch and the declarations of section 9. The 6a-4 grammar must use only these words, so an accepted gesture name is never taken away later.
4. **Structural strike checks happen in the semantic stage.** An empty target list and `Treble(0)` are rejected there (`EmptyTargets`, `InvalidSelector`), because they would fail at bind for every voicing.
5. **Spans may cover rests and holds.** How a span applies to them is a bind rule (6a-3, PE-6).

**Evidence** (`crates/mc-pattern/tests`):
- One test per rule: `pt01_` to `pt08_`.
- The nine confirmed corpus patterns, written by hand as IR, are valid.
- Property tests:
  - 400 random valid patterns are accepted. The generator covers one and two lanes, tuplets, gaps, meters, spans, strikes, gestures, both kinds of hold, and rests, with counts checked.
  - 400 cases inject one of seven violations and check that the error names the injected step or span.
  - 300 random libraries round-trip through their plain data.
- Thirteen mutation checks, one per rule or guard, were all caught.
- FF-24 lists only PT-02 and PT-09 without a test; both are bind rules (6a-3).

| Id | Result | Notes |
|---|---|---|
| PE-1 | **Partly met** | Onsets and durations are exact rationals and written values. Parts → library → parts, and library → parts → library, are identical for every generated library. **"Byte for byte through its DTO" is not checked:** the serialized DTO belongs to `mc-project` with format 1.2 (sections 1 and 8, slice 6a-5), and `mc-pattern` may not depend on serde. The byte check is proposed to be done with PE-13 in 6a-5, on the stored DTO; it is not counted as met here. |
| PE-2 | Met | As above. |
| PE-9 | **Partly met** | Met at the expansion level: a gesture used by two patterns expands to the same strike in each; renames change no resolved result and no pattern; unknown ids, duplicate names (without case, Vietnamese included) and keywords are typed errors; a gesture in use cannot be removed; a change that would break a pattern changes nothing. **"Binds identically" needs the binder of 6a-3**, and is checked there on bound output. |

Fitness: FF-01 (14 crates), FF-03 (the domain `clippy.toml`), FF-04 (`mc-pattern` builds for wasm32), FF-07, FF-34. For FF-22, the snapshot `architecture/public-api/mc-pattern.txt` is new and the weekly job now lists the crate.

## Alternatives

- **DSL as the stored form.** Rejected by the user: one syntax would own the data.
- **Equal division by step count.** Rejected by the user (Q-03).
- **One lane with per-note durations inside a beat.** Rejected: the score's voice model cannot hold it.
- **Built-in onomatopoeic keywords.** Rejected by the user: player names are gestures.
- **Effective (content-dependent) bar durations for the range.** Rejected: the result would depend on what is being overwritten.
- **Selecting muted strings for percussive hits.** Rejected for 6a by the user: the score cannot represent or play that event correctly.

## Trade-offs

- Two lanes cover thumb-and-fingers and strum styles. Three independent rhythmic layers need a later change.
- Asking for fit and for overwrite adds a dialog step. This is deliberate, so no notes are lost silently.
- Rejecting pickups in range means a pattern is applied from bar 1 on, until pickup support exists.
- A voicing such as D `xx0232` has no alternate bass under this rule, so a pattern using `Alt` fails on it with a typed error rather than guessing.

## Consequences

- New crate `mc-pattern`.
- Edges added in 6a-3 (`mc-app -> mc-pattern`) and 6a-5 (`mc-project -> mc-pattern`).
- `mc-app`: pattern commands, `ApplyPattern`, fit, overwrite safeguard, report.
- `mc-project`: library DTO (format 1.2).
- `mc-ffi` and Dart: view models, grid editor, palette, preview and dialogs.
- Documents: `domain-invariants.md` (PT-01 amended, PT-03 to PT-09), `bounded-contexts.md` section 4, `guitar-pattern-engine.md`, `open-questions.md` (Q-01 to Q-06) and the roadmap. Each is marked as following this Draft.

## Related fitness functions

FF-01, FF-03, FF-04 (`mc-pattern` builds for wasm32), FF-05, FF-11 and FF-32 (format 1.2), FF-12, FF-19, FF-22, FF-34.

## Reconsideration conditions

- More than two lanes.
- A chord track.
- Pickup support.
- Muted-string or body percussion.
- Stored swing.
- An IR need that changes `Step`.
