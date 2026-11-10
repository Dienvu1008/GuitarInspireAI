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
| 6a-2 | `mc-pattern`: IR, semantic validation, gestures (in memory) | PE-1 (structural round trip), PE-2, PE-9 (expansion level); decision 15.2 |
| 6a-3 | Binding, placing, `ApplyPattern` with fit and overwrite safeguard, through `mc-app` and FFI; a minimal apply dialog for one built-in sample pattern | PE-3 to PE-8, PE-15; the bind part of PE-9 (decision 15.2) |
| 6a-4 | Text DSL front end and printer; confirmed corpus as goldens | PE-10, PE-11 |
| 6a-5 | Grid editor, gesture palette, library storage (format 1.2) | PE-12 to PE-14; the byte part of PE-1, on the stored DTO (decision 15.2) |
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

## 14. Results of slice 6a-2 (2026-11-09; reviewed, see section 15)

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

**Interpretations made in 6a-2** (confirmed by the user, decision 15.1):
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

## 15. Decisions (user review of slice 6a-2, 2026-11-10)

1. **Interpretations 1 to 5 of section 14 are kept:**
   - PT-05 is checked for every voicing, with no gap and with narrowing;
   - PT-08 names are letters and digits starting with a letter, with no Unicode normalization;
   - the provisional keyword list stands;
   - structural strike checks happen in the semantic stage;
   - spans may cover rests and holds.
2. **Two criteria parts move, unchanged in substance:**
   - The **byte part of PE-1** ("round-trips through its DTO byte for byte") is checked in **6a-5**, on the stored DTO of `mc-project` (format 1.2), together with PE-13.
   - The **bind part of PE-9** ("a gesture used by several patterns binds identically in each; a rename changes no bound result") is checked in **6a-3**, on bound output.
   - Until then both stay open, and neither criterion is counted as met.

## 16. Results of slice 6a-3a, binding (2026-11-10, for user review)

**Built.** `mc_pattern::bind` (and `PatternLibrary::bind`, which resolves gestures at bind time):
- Inputs: a resolved pattern, one voicing (`mc_guitar::Voicing`), its chord and the track's `FrettedInstrument`.
- Output: a `PatternFragment` per lane, in pattern time. One `BoundBeat` per strike or hold, with its notes (string 1 first, each string once) and its stroke.
- `mc-pattern` now also uses `mc-guitar`, as the FF-01 matrix already allowed.

**Section 5 as implemented:**
- **Order.** S is ordered by string number: the bass is the highest-numbered sounded string, and `Treble(n)` takes the n lowest-numbered sounded strings above it. This is what the four Alt fixtures assume.
- **Alt.** The fifth's pitch class is root + the degree-5 interval of the chord quality, and the third's uses degree 3. Pitch classes come from tuning, capo and fret, so a capo shape is matched against the sounding chord.
- **Up-front checks.** Before any selector, the voicing must have one entry per string (`VoicingStringCount`), sound at least one string (`NoSoundedStrings`), and stay within the instrument's frets (`FretOutOfRange`).

**Interpretations made in 6a-3a, which the user should confirm or change:**
1. **Attacks.** An attack's notes get the strike's techniques plus those of every span that covers its step. The beat gets the strike's direction.
2. **Holds.**
   - A hold's notes are tied, on the same strings and frets as the beat before.
   - They keep only the let ring of the note they continue. Accent and the other flags are not repeated on a tie, and a span over a hold does not change its notes.
   - A hold has no stroke.
3. **A hold of a short note is rejected.**
   - Holding a dead, palm-muted or string-percussive note gives `HoldOfShortNote { step, string }`.
   - The score would play that tie as a new attack: short techniques end tie chains (ADR-024, ADR-035). So the hold would sound twice.
4. **A span that reaches no attack is rejected.** A span that covers only holds and rests gives `SpanReachesNoAttack { span }`, so its technique is never silently lost.
5. **Rests and gaps produce no beat.** Placement (6a-3b) writes them as rests (PT-09).

**Evidence** (`crates/mc-pattern/tests/bind.rs`):
- The four Alt fixtures give the stated results (C, G and Am string 4; D `NoAlternateBass`), and the fifth-then-third-then-lowest order is tested.
- A table of 15 voicings covers standard, drop D and DADGAD tunings, with capo 0, 2 and 3, slash chords and an inversion. On each it checks Bass, Treble(1) to Treble(5), Strum, Alt, Absolute on all six strings, and that every note sits at the voicing's fret.
- StrumRange (either order) and the `+` union are tested.
- Each typed failure is tested: `StringOutOfRange`, `StringNotInVoicing`, `NoAlternateBass`, `NoTrebleStrings`, `EmptyStrike` and the three voicing checks.
- Property tests:
  - 400 random patterns on random voicings, tunings, capos and chords. On success: one beat per strike or hold, every note at the voicing's fret, attacks with exactly the strike's and spans' flags and its direction, and holds as tied continuations. On failure: only typed errors, each naming a real step.
  - A coverage test shows success, success with holds, and every error kind.
  - 300 cases where holds must bind: every string sounded, no short technique.
- The bind part of PE-9: a gesture binds to the same notes and stroke in two patterns, and renames change no bound fragment.
- Eleven mutation checks on the binder were all caught.

| Id | Result in 6a-3a |
|---|---|
| PE-3 | Met |
| PE-6 | Met at bind level (steps, spans and gestures to notes and beats). Placement (6a-3b) and apply (6a-3c) must keep it; checked again there |
| PE-9 | Bind part met; with section 14 PE-9 is met, pending review of the interpretations above |
| PT-02 | Tested (`pt02_`) |

## 17. Results of slice 6a-3b, placing (2026-11-10, for user review)

**Built.** `mc_pattern::place(fragment, bars, fit)`:
- `bars` are the master bars of the range, in order. Their durations are nominal (time-signature lengths).
- The result is, per bar, one beat list per lane, in voice order (Strum or Thumb = voice 1, Fingers = voice 2). Each list fills its bar exactly.
- A `PlaceReport` lists repetitions, dropped and shortened steps (lane, repetition, step, onset, written value, new values), attacked notes, and with `WholeRepetitionsOnly` where the trailing rests begin.
- Errors: `EmptyRange`, `PickupInRange`, `MeterMismatch`, `FitRequired`, `TupletCrossesBar`, `ShortNoteNeedsTie` (named `ShortNoteCrossesBar` in the first version; see section 20), `UnwritableLength`.

**Section 7 as implemented:**
- A whole multiple repeats with no fit. Any other length needs `Fit::RepeatAndCut` or `Fit::WholeRepetitionsOnly`; `None` gives `FitRequired`, and cancel is the caller not applying.
- `RepeatAndCut`: a step that starts at or after the range end is dropped. A step that would sound past the end is shortened to the remaining length, written as tied values, unless it is a tuplet, which is dropped. Nothing else moves.
- `WholeRepetitionsOnly`: only whole repetitions; when the pattern is longer than the range, that means zero repetitions and only rests.
- A pattern without a meter runs on across bars of different time signatures.

**Interpretations made in 6a-3b, which the user should confirm or change:**
1. **Writing computed lengths.** Gaps, bar-line pieces and cuts are written exactly, longest value first:
   - a dyadic length as whole notes, then at most one plain value per binary digit, down to 1/256;
   - a length with one factor 3 in its reduced denominator gets one triplet value for the remainder (for example 1/6 is a triplet quarter, and 5/12 a quarter plus a triplet quarter);
   - anything else is `UnwritableLength`, never rounded. Example: the gap after a lone quintuplet sixteenth.
   - Dotted values are kept only where the pattern writes them; computed pieces are not merged into dotted values.
2. **Bar-line continuations** follow the hold rule of section 16: tied, same string and fret, only let ring kept, no stroke.
3. **A dead, palm-muted or string-percussive note that would need a tied continuation is rejected** (`ShortNoteNeedsTie`): it crosses a bar line, or `RepeatAndCut` shortens it to a length that needs more than one note value. Its tied continuation would be played as a new attack, the same reason holds of short notes are rejected in section 16. The first version checked only the bar-line case; section 20 records the fix.
4. **The note count** in the report counts attacked notes only, not tied continuations.

**Evidence** (`crates/mc-pattern/tests/place.rs`, 19 tests):
- PT-09 (`pt09_`): gaps become rests, and every bar is filled exactly.
- Fit: a whole multiple needs no fit; anything else is refused without one. The `RepeatAndCut` report is checked exactly, including a shortened half, a dropped tuplet that would need cutting, and a step starting exactly at the range end. `WholeRepetitionsOnly` leaves trailing rests, including the zero-repetition case.
- Pickup and meter are checked; continuity across 4/4 and 3/4 is checked.
- Bar lines: a dotted half across a bar line becomes a half tied to a quarter. `TupletCrossesBar`, `ShortNoteCrossesBar` and `UnwritableLength` are each tested.
- Two lanes give two voices with independent durations (corpus 2).
- `values_for`: a table, plus a property that every result sums exactly.
- PE-4 property (300 cases): the placed bars are read back into events and compared with `range start + repetition offset + step onset`, its duration, its notes and its stroke. Every bar's sum is exact, and failures are only real tuplet crossings. A coverage test: 356 of 400 cases place, 91 with bar-line splits and 61 with holds.
- Thirteen mutation checks were all caught. Two survived at first (a step exactly at the range end; the note count), and tests were added for them.

| Id | Result in 6a-3b |
|---|---|
| PE-4 | Met at domain level |
| PE-8 | Met at domain level (range, fit, pickup, meter, mixed time signatures). Its "session tests" part comes with `ApplyPattern` in 6a-3c |
| PT-09 | Tested (`pt09_`) |

## 18. Results of slice 6a-3c, ApplyPattern (2026-11-10, for user review)

**Built:**
- **The `mc-app → mc-pattern` edge**, added to `allowed-deps.toml` as section 1 planned.
- **`Command::Pattern(PatternCommand::ApplyPattern(ApplyPattern { track, first, last, pattern, voicing, chord, fit, overwrite }))`**, with plain values: the voicing is a fret or `None` per string, string 1 first; the chord is a symbol.
  - It is a separate command family because it needs the session's pattern library.
  - It binds (section 5), places (section 7), applies the overwrite safeguard, and writes through the new atomic `Score::replace_voices`. That method checks everything first, then replaces whole voices of the range, adds voice 2 only where the pattern writes it, and normalizes ties once.
  - It is one revision and one undo step. `CommandOutcome.pattern` carries the report: range, voices written, overwritten bars, and the placement report.
- **Errors:** `CommandError::Pattern(ApplyError)`, with `UnknownPattern`, `Bind`, `Place` and `WouldOverwrite { bars }` (rule `PE-07`). An invalid chord symbol is a `Value` error, a voicing that cannot be built a `Guitar` error, and a bad range or track a `Domain` error. On every error nothing changes.
- **`DocumentSession::preview_pattern(args)`:** the same plan, with the bars it would write and the bars it would overwrite. It changes nothing; this is what the apply dialog shows.
- **The session pattern library:**
  - It is session state like the transport: not saved before format 1.2 (6a-5), and not part of undo.
  - It holds one built-in sample pattern, corpus 3 "Boom-chick". It is a pattern, not a gesture; no gesture is built in.
  - `patterns_mut()` gives direct access until the undoable library commands of 6a-5.

**Interpretations made in 6a-3c, which the user should confirm or change:**
1. **What counts as existing content.** A checked voice "holds a beat" when it has any beat, rests included.
2. **Order of checks.** Binding and placement errors (fit included) come before `WouldOverwrite`, so the dialog asks for a fit first and for overwrite confirmation second.
3. **The bar after the range.** A tie in the bar after the range that loses its source is cleared and reported (`TiesRemoved`, H-06), as for every other edit.
4. **The sample pattern** is corpus 3.

**Evidence** (`crates/mc-app/tests/pattern.rs`, 13 tests; `crates/mc-score/tests/replace_voices.rs`, 3 tests):
- **PE-7:**
  - The sample fills an empty range in one revision and one undo step.
  - Only voices 1 and 2 of the track in the range change. Other tracks, master bars, the tempo map, bars outside the range and voice 3 are equal in model content (derived equality, ids included; `score_json` is a pure function of it).
  - `WouldOverwrite` names exactly bars 1 and 3 and changes nothing; the preview names the same bars.
  - A Strum pattern over bars with only voice 2 content needs no confirmation and leaves voice 2 unchanged.
  - A Thumb and Fingers pattern is blocked by voice 2 content (amendment A).
  - Undo restores the `score_json` bytes, and redo reapplies them.
  - Ten refusals are each typed and change nothing; the preview refuses them too.
  - The preview equals what is applied.
- **PE-7 property:** 128 cases over random scores, with beats seeded into random bars. Without confirmation, `WouldOverwrite` names exactly the occupied bars, or the command applies to an empty range. With confirmation, only voice 1 of the range changes, and undo restores the bytes. A coverage test reaches every branch: 81 applied, 116 refused and then applied after confirmation, 3 other refusals.
- **PE-8 through the session:** fit required, `RepeatAndCut` (with its report), `WholeRepetitionsOnly`, a whole multiple without fit, pickup and meter refusals, and a pattern without a meter across 4/4 and 3/4. Both bars come out exactly full, with no D-01 diagnostic.
- **PE-5:** corpus 2 (Travis) applied and compiled with `mc_playback::compile`. The thumb's C3 starts at 0 and ends at 1/4, its own duration, while a finger note attacks at 1/8.
- **Mutation checks:** seven on the apply path and `replace_voices`, all caught.

| Id | Result in 6a-3c |
|---|---|
| PE-5 | Met |
| PE-7 | Met |
| PE-8 | Met (domain level in section 17, session level here) |
| PE-15 | Met: see the measurement below |

**PE-15 measurement** (`cargo bench -p mc-app --bench commands -- pattern`, release, reference machine, 2026-11-10, one run):
- What is timed: one `execute` of `ApplyPattern` with a 4-bar Thumb and Fingers pattern (16 thumb quarters and 32 finger eighths) on bars 0 to 63, overwrite confirmed. That covers binding, placing, the overwrite check, writing both voices, ties and the undo snapshot.
- Each sample starts from a fresh session.

| Score | Samples | p50 | p99 | max | Criterion (mean) | Budget |
|---|---|---|---|---|---|---|
| Empty, 64 bars | 200 | 0.44 ms | 0.99 ms | 1.23 ms | 0.53 ms | ≤ 20 ms |
| `reference(TYPICAL)`, first track, bars 0 to 63 (existing content) | 200 | 0.74 ms | 1.52 ms | 1.64 ms | 0.57 ms | ≤ 20 ms |

## 19. Results of slice 6a-3d, bridge and minimal apply dialog (2026-11-10, for user review)

**Built:**
- **`mc-ffi`**, with no music rule:
  - `patterns`, `sample_pattern` and `preview_pattern`;
  - `CommandDto::ApplyPattern { args }`;
  - `OutcomeDto.pattern`, the report.
  - The voicing uses the chord finder's convention (`frets`, string 1 first, `-1` muted), so a chord-finder voicing goes straight into the command.
  - The preview maps "a fit is needed and none was given" to `needs_fit`. Every other refusal is an `ApiError` with the core's rule: `PE-07` overwrite, `PE-08` range and fit, `PE-04` bar lines and lengths, `PT-02` binding, `PT-06` unknown pattern, `voicing` for a bad fret code.
  - `ApplyError` has a readable message, for example "bars 2, 4 already have content in the voices the pattern writes".
  - Bindings were regenerated twice: 15 files with identical hashes (FF-12).
- **`core_bridge`:** `DocumentService.patterns`, `samplePattern` and `previewPattern`, in the real service and the fake. The fake decides no music: it writes placeholder beats, and a test hook makes it report that a fit is needed.
- **The dialog**, Tools > Apply pattern... (`editor_feature`):
  - Inputs: the pattern; a chord and one of its voicings from the chord finder; the first and last bar.
  - Preview gives a summary: repetitions, notes, bars, the voices written, the bars replaced, and the steps dropped or shortened.
  - When the core needs a fit, it asks for "Repeat and cut" or "Whole repetitions only". Any change of the range clears the fit and the preview, so the dialog asks again (decision 13.6).
  - Apply is enabled only after a preview. It asks for confirmation before replacing content, and sends one `CommandDto.applyPattern`.
  - Errors from the core are shown in the dialog. Fields and messages have semantic labels, and messages are live regions.

**Interpretations made in 6a-3d, which the user should confirm or change:**
1. **The preview is a text summary, not a TAB preview.** A TAB preview of the result needs a layout of a score that does not exist yet; it belongs to the grid editor (6a-5, PE-14).
2. **A stale preview.** Superseded by section 20: the first version sent `overwrite: true` after a confirmation without the preview's revision, so a confirmation could be applied to content that had changed in the meantime. Apply is now sent with the preview's revision and refused with `E-01` if the score changed.
3. **The library is not saved.** It holds only the built-in sample until 6a-5.

**Evidence:**
- **The bridge from Rust** (`crates/mc-ffi/tests/api.rs`):
  - `patterns` lists the sample.
  - With a 4/4 and a 3/4 bar, the preview without a fit gives `needs_fit`. With `RepeatAndCut` it reports two repetitions and the dropped last quarter, with lane, step and onset. `execute` without a fit is refused with `PE-08`, and nothing changes; with the fit, the outcome carries the same report.
  - A bad fret code gives `voicing`. Existing content gives `PE-07`, with "bars 2" in the message.
- **The contract** (fake and real core): list, preview, apply in one revision, `PE-07` without confirmation (revision unchanged), apply after confirmation, undo, `PT-06`.
- **Dialog widget tests** (8 tests):
  - preview and apply in one command;
  - Cancel at the confirmation sends nothing, and Overwrite applies;
  - the fit step; a range change clears the fit, and the next preview asks again;
  - a stale preview is refused by the core;
  - invalid inputs (no voicing, bars out of range, an empty chord);
  - Tab, Enter and Space;
  - semantic labels and live regions.
  - Five dialog mutations were all caught.
- **The Windows acceptance test** "6a-3: apply the sample pattern from the Tools menu" (real core): preview, confirmation, one revision, four beats per bar with the down-strum on beat 2, and undo with Ctrl+Z.

## 20. Review of slice 6a-3 and its corrections (2026-11-10)

The user accepted the interpretations of sections 16 to 19, except the stale preview, and asked for three corrections, made in this order.

1. **Short-technique notes that need a tie (section 17, interpretation 3).**
   - The review described `lay_out()` as already refusing a `RepeatAndCut` cut that needs more than one note value. **It did not.** The check ran only when a piece crossed a bar line, so a palm-muted note cut to 3/8 (a quarter plus an eighth) was placed with a tied continuation, which playback sounds as a second attack. A test reproduced this before the fix.
   - The check now covers any second value or piece. The error is `ShortNoteNeedsTie`. It is an exception to shortening: not every short-technique note can be shortened. A short note cut to a single value is still shortened.
   - Tests: the cut to 3/8 is refused; the same cut of a normal note is tied values; a cut to 1/8 is kept. Removing the new condition makes the test fail.
2. **Overwrite dialog wording.** The message is now "Bars … already contain beats in the voices the pattern writes, including rests. Replace them?", because a voice with only rests counts as content (decision 18.1). Tests: a rest-only voice in the core, and in the dialog with the exact wording.
3. **Optimistic concurrency for the confirmation (replaces interpretation 19.2).**
   - The core returns the document revision with the preview, read under the same lock (`PatternPreview.revision`, `PatternPreviewDto.revision`).
   - The dialog keeps it and sends it as `expected_revision` with `ApplyPattern`, through a new `DocumentController.executeAt`. The core already enforced `expected_revision` (E-01).
   - On `E-01` the dialog drops the preview and the fit, says the score changed, and asks the user to preview and confirm again. It never resends with `overwrite: true`.
   - The rule stays in the core. Any change since the preview is refused, not only a change in the range.
   - Tests:
     - In the core, the dangerous sequence (the preview finds content, another command changes the same bar, the confirmation is sent): `StaleRevision`, the score unchanged.
     - In the dialog, the same sequence with the confirmation open: the note added meanwhile survives, the dialog asks to preview again, and a fresh preview and confirmation then applies.
     - A new preview after an unrelated change uses the new revision.
     - Mutations that drop the revision, disable the `E-01` branch or keep the first revision are all caught.
   - The earlier test covered only content appearing before the first Apply; it is replaced by these.
4. **Checklist for 6a-5.**
   - Changing the pattern clears the fit. This is done already, in the same `_changed(range: true)` call as a range change; a test needs more than one pattern and comes with the library.
   - The grid and apply dialogs must also keep the revision rule above for every command that follows a confirmation.

## 21. Results of slice 6a-4, text syntax and corpus goldens (2026-11-10, for user review)

**Built:**
- **`mc_pattern::text`**: `parse(text, &library) -> Result<PatternParts, ParseError>` and `print(&parts, &library) -> Result<String, PrintError>`. The grammar is in [pattern-text-syntax.md](../pattern-text-syntax.md).
  - The parser is hand-written and scannerless. It stops at the first error, which has a byte span, a line, a column and a typed kind.
  - Parsing is syntactic only. `PatternLibrary::define_pattern_text` parses, then validates with the existing semantic stage: `TextError::Parse` or `TextError::Invalid(Vec<PatternError>)`.
  - No new dependency, no I/O, no `unsafe`. The DSL is a library API only: there is no FFI or UI for it yet.
- **The confirmed corpus as text**: `crates/mc-pattern/tests/corpus/`, one file per pattern 1, 2, 3, 4, 5, 6, 8, 9, 10. Pattern 7 has no file.
- **The goldens** (`crates/mc-app/tests/corpus.rs`): each corpus file is defined in a fresh session, applied to its voicing (pattern 6 on a 3/4 bar, pattern 10 on two 2/4 bars), and checked against a TAB table and a timeline golden in `crates/mc-app/tests/golden/`. The golden files are rewritten with `cargo test -p mc-app --test corpus -- --ignored update_goldens`. The tests live in `mc-app` because `mc-pattern` may not depend on `mc-playback` (FF-01).

**Interpretations made in 6a-4, which the user should confirm or change:**
1. **Header keys are quoted strings** (`"name": "…"`, `"length": 1`). Decision 15.1 allows only its reserved words in the grammar; words such as `name` or `length` would take names away from gestures. Only `meter` and `unit`, which are on the list, are bare.
2. **Pattern 1 is a single `strum:` lane.** The corpus writes it as `fingers:`, but its IR has one lane, written to voice 1, which is the strum lane; `fingers:` alone needs `thumb:` (decision 5).
3. **The printer is canonical.** It always writes an explicit value, writes `@onset` only where there is a gap, writes spans in the index form (`strum pm#0..7`) after the lanes in IR order, and writes `|` where a bar starts and the step follows the previous one. It never writes `unit` or the bracket form. So crossing or overlapping spans, gaps and explicit rests all round-trip exactly, and `origin` is printed and parsed as it is, not normalised to `text`.
4. **A gap and `-` are different IR** (PT-09): `@3/8 1:8` leaves a gap; `-:8 1:8` writes a rest step. Both are rests in the score.
5. **The bar check `|`** checks the position after the previous step against the declared meter. It is an error without a meter (`BarCheckWithoutMeter`) and off a bar line (`BarCheckOffBarLine`), checked when the text ends. It changes no IR.
6. **Gesture names are bare identifiers**, resolved against the library at parse time, without case. A reserved word in place of a gesture name is `KeywordNotAllowed`. Text with a gesture therefore parses only against a library that has it.
7. **The PE-11 TAB check reads the score**, not a rendered TAB: for each eighth of the bar, the string, the fret (`x` for string-percussive) and the marks (stroke, `PM`, `>`, `S`) of every note the pattern wrote, in all voices. The tables were transcribed by hand from the corpus document. The rendered TAB (P.M. line, `x` glyphs) is covered by the 6a-1 notation tests, not again here.
8. **The timeline goldens were generated by the code**, then reviewed by hand against the corpus and ADR-035 for patterns 2 (the thumb quarters sound under the finger eighths), 8 (accent 104, palm mute 80, notes 1/16 long), 9 (string-percussive with accent 94, 1/64 long; other strums 96) and 10 (two bars, bar 2 starts at 1/2). The other goldens are locked as generated; their TAB tables were checked independently.

**Evidence:**
- **PE-10** (`crates/mc-pattern/tests/text.rs`, 26 tests):
  - Property: `parse(print(ir)) == ir` and `print(parse(print(ir))) == print(ir)` for random valid IR (500 cases): both lane shapes, gestures, holds of all or some strings, rests, gaps, dots up to three, triplets, quintuplets, septuplets, 1/64 and 1/256, crossing spans, meters, any Unicode in name, description and tags, every origin. A coverage test asserts that the generated inputs contain each of these features.
  - Property: patterns over several bars of 4/4 or 3/4 print bar checks and parse back to the same IR.
  - Every confirmed corpus file parses to an IR written by hand from the corpus document, validates, and prints canonically.
  - A step without a value and without a `unit` is `NoValue`, with the span of the step. Keywords and header keys are case-insensitive (`"LENGTH"`, `METER`, `Unit`, `STRUM:`, `BASS`, `bass`, `TREBLE(4)`, `PM(SP(ALT))`, `d`, `u`). A bar check off a bar line is `BarCheckOffBarLine` with the position and the bar length.
  - The kind and span of each syntax error; syntax-valid but musically invalid text is refused by the library, not the parser.
  - Property: the parser never panics, and its spans are inside the text and on character boundaries, for cut-off text, text with random Unicode inserted, and random strings. This found a real bug: the "step outside a lane" error took one byte as the span of the current character, which split a multi-byte character; fixed.
  - Eight mutations of the parser and printer (dead and let-ring marks, up-strokes, palm-mute wrapper, gap onsets, bar checks, the off-bar-line error) were all caught.
- **PE-11** (`crates/mc-app/tests/corpus.rs`): for each of the 9 confirmed patterns, the TAB equals the corpus table and the timeline equals its golden; applying twice gives the same score bytes. Pattern 7 is not a golden.
- **Workspace:** 468 tests pass; clippy with `-D warnings`, FF-01 to FF-34 (`fitness all`), `cargo deny` and the mermaid check pass. The `mc-pattern` public API snapshot has additions only (the `text` module and `define_pattern_text`).

## 22. Slice 6a-5: plan, and results of 6a-5a, library storage (2026-11-10, for user review)

The user accepted 6a-4 and started 6a-5. Like 6a-3, it is built in sub-slices, each committed with its evidence; the user reviews the whole slice.

| Sub-slice | Content | Criteria |
|---|---|---|
| 6a-5a | Library storage, format 1.2, through `mc-project` and the session | PE-13; the byte part of PE-1 |
| 6a-5b | Undoable library and pattern-editing commands in `mc-app`, with the grid view model from the core | PE-12 (core side), part of PE-14 |
| 6a-5c | FFI and the grid editor | PE-12, PE-14 (place, move, resize, delete, snap, span, keyboard) |
| 6a-5d | Gesture palette (a gesture from a strike), the apply dialog on the stored library, and the Windows integration test | PE-14 (rest); the 6a-3 checklist of section 20 |

**Built in 6a-5a:**
- **Format 1.2** (`mc-project`): the library is one entry, `patterns/library.json` ([project-format.md](../project-format.md) section 2b). New DTOs (`mc_project::library`), `library_json`, `save_with_library`, and `LoadedProject.library`. `save` keeps its signature and writes no library.
  - On load, the shape is checked first. Every problem is a P-05 violation with its JSON path under `library` (`LoadError::Invalid`). The library is then rebuilt through `PatternLibrary::from_parts`, so every library rule is checked (GR-09 analogue); a broken rule is the new `LoadError::InvalidLibrary`.
  - A 1.2 reader knows the entry; any other listed entry is still unknown content.
- **The dependency edge `project -> pattern`**, planned in the dependency-rules matrix ("Phase 6") and in section 1, is now in `architecture/allowed-deps.toml`.
- **The session** (`mc-app`):
  - It opens the stored library. A file without one gets the built-in sample library, as before.
  - It saves the library, also in autosave snapshots.
  - The library is part of the dirty check: a library change makes the document dirty, and saving makes it clean.
  - `DocumentSession::sample_pattern()` now returns `Option`: the sample can be removed from a stored library. The FFI function keeps its signature and returns `-1` then. The dialog handles that in 6a-5d.
- **Fixtures:** `mc_pattern::fixtures::library()` (feature `fixtures`) is a reference library that uses every stored feature. `mc-cli format-fixtures` writes `v1.2/patterns.{package,score.json,library.json}`, the 1.2 fixtures of the existing scores, and the 1.2 schema snapshots (score, manifest and the new library schema). Released 1.0 and 1.1 fixtures and schemas are untouched; the 1.2 `score.json` files and the score schema are byte-identical to 1.1.

**Interpretations made in 6a-5a, which the user should confirm or change:**
1. **One library entry, not one JSON per pattern.** project-format section 2 first sketched `patterns/` with one file per pattern. One entry is simpler and safer: patterns refer to gestures by id, `next_id` must be stored once, and the library is validated as a whole. Section 2 now says so.
2. **No library entry while the library is still the built-in sample library.** Otherwise every file saved by this build would carry the entry, and a 1.1 build would refuse to save over it, even for users who never touched patterns. A 1.1 build opens and saves such a 1.2 file normally (checked, see below). Reopening such a file gives the sample library again, so nothing is lost.
3. **Library edits are not undoable yet.** `patterns_mut` still changes the library directly; undoable commands come in 6a-5b. Until then, undo does not touch the library, and a library change counts for the dirty check only.
4. **Enums with data are stored as structs with a `kind`** (for example `{"kind":"strum_range","from":6,"to":4}`), so every level keeps an `extra` map for U1. A field that does not belong to the kind is a P-05 error with its path. A new kind is a JSON error (refused), as project-format section 9 says for enums that are not designated extensible.
5. **`sample_pattern` is found by content**: the first pattern equal to the built-in sample, not a stored flag.

**Evidence:**
- **PE-13** (`crates/mc-project/tests/roundtrip.rs`, module `library`, and `crates/mc-app/tests/library.rs`):
  - The 1.2 fixture loads with no warning, its library equals the reference library, and `library.json` and the whole package are written again byte for byte. Current output equals the golden `library.json`.
  - The fixture uses every stored feature (a test lists them, including the non-ASCII gesture names and a gap in the ids).
  - Every 1.0 and 1.1 fixture loads with no library. The 1.1 fixtures load to the same scores as the 1.2 ones, with identical `score.json`.
  - U1: an unknown key at every level of the library DTO is reported with its exact path, and the known content still loads. A library entry missing from the manifest is refused (`UnlistedEntry`).
  - A misshapen library is refused with every problem and its path (missing or extra selector fields, string 0, both lane shapes, a field of another action kind, a bad onset or value). An unknown enum value is a JSON error of the entry.
  - Library rules on load: duplicate gesture names, an id at or above the counter, an overlap, and a missing gesture are each `InvalidLibrary`.
  - Session: no entry for the sample library; save, reopen and save again keep `library.json` and `score.json` byte for byte; library changes make the document dirty and saving cleans it; removing the sample is kept; autosave snapshots carry the library; unknown library content blocks Save over the original (U2), which stays byte-identical, and Save As works.
  - **The older reader** (done by hand, as in ADR-035 TQ-3): `mc-cli` built at `1290325` (6a-1, format 1.1) opens `v1.2/patterns.package`, reports one item of unknown content, `package:patterns/library.json`, and passes `validate`. Saving over the file fails ("cannot be saved back; use Save As") and leaves it byte-identical; Save As to a new file works. The same build opens `v1.2/techniques.package` (no library) without unknown content and saves over it.
- **PE-1 (byte part):** a property test (200 cases) builds random valid libraries from the 6a-2 generator (the three test gestures, up to four random patterns with any Unicode names and tags, some removed), saves them in a package and loads them back: the same library, the same `library.json` bytes, and the same package bytes. A second property mutates `library.json` bytes; loading never panics.
- **Mutations:** 10 mutations of the loader, the DTO walk, the shape checks and the session (the known entry, the U1 walk of the library, the strike walk, selector fields, action fields, the dirty check, the "no entry for the sample" rule, autosave, unsaved work). 9 were caught. The tenth (returning early after a lane-shape error) gives the same result, because nothing checked after the lanes can fail on a 64-bit target.
- **Public API:** additions in `mc-project`. Two signatures changed on purpose: `DocumentSession::sample_pattern() -> Option<PatternId>`, and `mc_project::schemas()` now returns three schemas.

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
