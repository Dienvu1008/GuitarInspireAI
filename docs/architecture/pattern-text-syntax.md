# Pattern text syntax (Phase 6a-4)

Purpose: The reference for the text front end of the pattern engine (`mc_pattern::text`, [ADR-034](decisions/ADR-034-pattern-engine-ir-first.md) section 9). The text is one way to write the Pattern IR; the IR stays the source of truth. `parse(print(ir)) == ir` for every valid IR.
Status: Implemented in slice 6a-4; the grammar below is derived from the confirmed corpus ([pattern-corpus-proposed.md](pattern-corpus-proposed.md)) and is reviewed with ADR-034 section 21.
Related documents: [pattern-corpus-proposed.md](pattern-corpus-proposed.md), [ADR-034](decisions/ADR-034-pattern-engine-ir-first.md), [ADR-035](decisions/ADR-035-playing-techniques-in-the-score.md).

## 1. Principles

- **Only the reserved words of ADR-034 decision 15.1** are words in the grammar: `bass alt treble strum thumb fingers d u pm sp unit meter`. Everything else is a quoted string or a symbol, so no gesture name is ever taken away. Keywords are case-insensitive (Q-01).
- **No implicit duration** (Q-03). A step has a value (`:4`) or a `unit` is in force. Otherwise it is an error with a source span.
- **Strings are absolute numbers** (Q-02): `3` is string 3, string 1 is the highest.
- **The printer is canonical.** It always writes an explicit value, writes an onset `@` only where a step does not follow the previous one, writes spans in index form and bar checks where a bar starts. `unit` and brackets are only conveniences for hand-written text; the printer never writes them.
- **Parsing is syntactic.** The parser builds the IR and reports syntax errors with a span. Musical validity (overlaps, holds, spans in range, gesture rules) is the semantic stage (`PatternLibrary`), with its own typed errors.

## 2. Layout

```
"name": "Boom-chick"
"length": 1
meter 4/4
"description": "Alternating bass"
"tags": "sample", "fingerstyle"
"origin": "grid"
strum: Bass:4 D Treble:4 Alt:4 D Treble:4
```

- Whitespace and line breaks separate items; `// ...` is a comment to the end of the line.
- A **header** is a quoted key, a colon and a value. Keys (case-insensitive): `"name"` (string), `"length"` (rational, whole notes, required), `"description"` (string), `"tags"` (comma-separated strings), `"origin"` (`"grid"`, `"text"` or `"tap"`). A key may appear once. A missing `"name"` or `"description"` is empty; a missing `"origin"` is none.
- `meter 3/4` declares the meter once. `unit 8` sets the duration of steps written without a value; it stays in force, across lanes, until the next `unit`.
- **Lanes.** A lane starts with a header at the start of a line: `strum:` (one lane, voice 1), or both `thumb:` and `fingers:` (voices 1 and 2, in either order). The steps follow, over any number of lines. A lane header is a lane word, a colon, and then white space or the end; `Strum:4` is a step.
- Strings are written `"..."` with the escapes `\"` `\\` `\n` `\r` `\t` and `\u{hex}`. The printer escapes every control character.
- **Gesture names** are written bare, as identifiers (a letter, then letters and digits, as the library requires). They are resolved against the library when the text is parsed, without case.

## 3. Steps

```
step     = [ "@" rational ] [ "D" | "U" ] { ">" | "!" | "~" } action [ ":" duration ]
action   = "-"                          a rest (no attack, no motion)
         | "_" [ "(" selectors ")" ]    hold the previous strike (all its strings, or these)
         | strike
         | gesture-name                 a user gesture, by name
strike   = selectors | "pm" "(" strike ")" | "sp" "(" strike ")"
selectors = selector { "+" selector }
selector = "Bass" | "Alt" | "Treble" | "Treble" "(" n ")" | "Strum" | "Strum" "(" a ".." b ")" | string-number
duration = base { "." } [ "t" | "(" actual ":" normal ")" ]       base: 1 2 4 8 16 32 64 128 256
```

- **Position.** A step starts where the previous step of its lane ended (0 for the first). `@3/8` starts it at 3/8 of a whole note instead; the gap before it is a rest (PT-09). A `-` is an explicit rest step, which is a different IR from a gap.
- **Direction** `D` or `U` is the stroke of the beat (ADR-035). It is only valid on a strike.
- **Marks** on a strike: `>` accent, `!` dead, `~` let ring. **Wrappers** `pm(...)` palm mute and `sp(...)` string-percussive wrap the whole strike (`pm(Bass+Treble)`); they may nest. These are the five techniques of ADR-035.
- **Durations.** `4` is a quarter, `8.` a dotted eighth, `8t` a triplet eighth, `16(5:4)` a quintuplet sixteenth. The tuplet ratio is the one of the score (H-07).
- **Marks, direction and wrappers are not allowed** on a rest, a hold or a gesture; the gesture carries its own technique and direction.

## 4. Spans and bar checks

- **Bracket form**, as in the corpus: `[pm >Bass:8 Bass:8 Bass:8]` is a palm mute over the three steps inside. The first token is `>`, `!`, `~`, `pm` or `sp`; brackets may nest and must contain at least one step.
- **Index form**, which the printer uses: a line `thumb pm#0..3` (or `fingers sp#2`) is a technique over the steps `0` to `3` of that lane, inclusive, counting from 0. It is written after the lanes, in the order of the IR, so spans that overlap or cross are kept exactly.
- **Bar check** `|` asserts that the position (the end of the previous step) is a whole number of bars of the declared meter. It is an error without a meter, or off a bar line. The printer writes it where a bar starts.

## 5. Errors

Every syntax error has a byte span, a line and a column. They include: an unexpected character or end, an unterminated string, a bad escape, a bad number, rational, duration, tuplet or meter, an unknown key or origin, a duplicated header, a missing `"length"`, a step outside a lane or without a value, marks on a non-strike, a string that is not 1 to 255, an unknown gesture name, a reserved word used as a gesture name, a bar check without a meter or off a bar line, mixed or incomplete lanes, and an empty or unclosed bracket.
