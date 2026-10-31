# Guitar Pattern Engine

Purpose: Propose a structured intermediate representation for reusable picking and strumming patterns, define validation stages, and document the ambiguity of the example notation. The DSL grammar is NOT finalized.
Status: Proposed (IR). Grammar: open. Implementation: Phase 6a, after open questions Q-01 to Q-06 are resolved.
Related documents: [bounded-contexts.md](bounded-contexts.md), [domain-invariants.md](domain-invariants.md), [open-questions.md](../research/open-questions.md), [implementation-phases.md](../roadmap/implementation-phases.md).

## 1. Why a structured IR rather than a string

A plain string cannot be validated without parsing, cannot be safely migrated, and ties the persisted format to one surface syntax. A structured IR:

- is the persisted form (versioned like other DTOs);
- can be produced by several front ends (text DSL, UI pattern builder, import);
- separates syntax errors from musical errors;
- can be bound to any voicing and rendered to TAB or MIDI through normal score commands.

## 2. Proposed IR (illustrative, not final)

```rust
pub struct Pattern {
    length: Rational,        // e.g. one bar of 4/4 = 1
    steps: Vec<Step>,
}
pub struct Step {
    onset: Rational,         // position within the pattern
    duration: NoteValue,
    action: Action,
}
pub enum Action {
    Strike { targets: Vec<StringSelector>, direction: Option<Direction>, articulation: Articulation },
    Rest,
}
pub enum StringSelector {      // candidates, not final
    Bass,                    // lowest string of the voicing that sounds the chord root, or lowest sounded string (to decide)
    AltBass,                 // alternate bass (e.g. string holding the fifth)
    Treble(u8),              // n highest sounded strings
    Absolute(StringIndex),   // physical string
    ChordTone(u8),           // nth sounded string from the bottom of the voicing
    StrumRange(StringIndex, StringIndex),
}
```

## 3. Validation stages

1. Parse (syntax): text to IR. Errors carry source spans.
2. Semantic: durations valid, step onsets ordered, PT-01 (sum equals length), selectors well-formed.
3. Bind: `Pattern x Voicing x Tuning -> ScoreFragment`. Resolves selectors to strings (PT-02). Fails with typed errors such as `NoAlternateBass` or `StringNotInVoicing`.

The fragment is applied through `mc-app` as an ordinary score command, so all score invariants apply (GR-04 analogue for generated content).

## 4. Ambiguity of `Bass 4 3 Treble Bass bass`

The example cannot establish case sensitivity, selector meaning, step timing or ordering semantics. Candidate interpretations (none accepted):

| Id | Interpretation | Bass | 4, 3 | Treble | Final `bass` |
|---|---|---|---|---|---|
| I1 | Case-insensitive sequence of equal-duration steps | Root bass string of the chord | Absolute strings 4 and 3 | Strum of the treble strings | Same as `Bass` (repeat) |
| I2 | Case-sensitive | Root bass | Absolute strings | Strum of treble strings | Alternate bass (for example the fifth) |
| I3 | Chord-relative numbers | Root bass | 4th and 3rd sounded chord tone from the bottom of the voicing | Highest chord tone only | Same as `Bass` or alternate, undecided |

Open questions (tracked in [open-questions.md](../research/open-questions.md)):

- Q-01 Case sensitivity of keywords.
- Q-02 String numbering convention for numbers (1 = highest or lowest; absolute or chord-relative).
- Q-03 Default step duration (one beat, a fixed subdivision, or the bar divided equally by step count).
- Q-04 Meaning of Treble: single note, pinch with bass, or strum; how many strings.
- Q-05 Direction defaults (down for bass, up for treble?) and how to write them.
- Q-06 Syntax for rests, ties, accents, simultaneous strikes (for example `Bass+Treble`).

## 5. Example to resolve the questions

Before grammar design, collect 5 to 10 real patterns the user wants to express (for example Travis picking, a bossa pattern, a folk strum) written in the user's intended notation, together with the expected TAB for one chord. The grammar is then derived from these examples and tested against them as golden fixtures.

## 6. Non-goals for 6a

No loops or conditionals in the DSL, no per-chord-type branching, no humanization. These can be added once the basic language is validated.
