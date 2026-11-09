# Pattern corpus: proposal for review (Phase 6a)

Purpose: A first set of patterns for the pattern engine ([ADR-034](decisions/ADR-034-pattern-engine-ir-first.md)), each with its intended meaning and the TAB it should produce on one voicing. The user reviews it, and confirmed patterns become the golden fixtures of criterion PE-11.
Status: **Proposed by the agent, NOT confirmed by the user.** Nothing here is a golden fixture until the user confirms it pattern by pattern. Pattern 10 is the user's own example, with the durations the user described; the other nine are the agent's proposals.
Related documents: [ADR-034](decisions/ADR-034-pattern-engine-ir-first.md), [ADR-035](decisions/ADR-035-playing-techniques-in-the-score.md), [guitar-pattern-engine.md](guitar-pattern-engine.md).

## How to read this

- **Voicings** (string 1 is the highest, Q-02; `x` = muted):

  | Chord | Voicing (strings 6 to 1) | Bass | Alt | Treble (3) |
  |---|---|---|---|---|
  | C | `x 3 2 0 1 0` | string 5, fret 3 (C) | string 4, fret 2 (E, the third; the fifth G is on string 3, which is in the treble group) | strings 3, 2, 1 |
  | G | `3 2 0 0 0 3` | string 6, fret 3 (G) | string 4, open (D, the fifth) | strings 3, 2, 1 |
  | Am | `x 0 2 2 1 0` | string 5, open (A) | string 4, fret 2 (E, the fifth) | strings 3, 2, 1 |

- **IR lines** use the DSL sketch of ADR-034 section 9, only to state the meaning. The grammar is not final.
  - Each step is `target:value`: `4` quarter, `8` eighth, `4.` dotted quarter.
  - `-` is a rest, `+` strikes together, `>` accents, and `D` and `U` are strokes.
  - `[pm ...]` is a palm-mute span, and `x(...)` is a percussive hit.
  - `thumb:` and `fingers:` are the two lanes; `strum:` is a single strum lane.
- **TAB tables:** the columns are the eighth-note positions of the bar, and the rows are strings 1 (e) to 6 (E). A number is the fret struck at that position, and an empty cell means nothing new is struck.
  - The "Marks" row shows ↓ ↑ (stroke), `>` (accent), `PM` (palm mute) and `S` (percussive).
  - The "Rings" row gives the written duration of long notes.

## 1. Basic fingerpicking (Am, 4/4)

IR: `fingers: Bass:8 3:8 2:8 3:8 1:8 3:8 2:8 3:8` (one lane, every note one eighth)

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | | | | | 0 | | | |
| B | | | 1 | | | | 1 | |
| G | | 2 | | 2 | | 2 | | 2 |
| D | | | | | | | | |
| A | 0 | | | | | | | |
| E | | | | | | | | |

## 2. Travis-style picking (G, 4/4)

IR, two lanes:
- `thumb: Bass:4 Alt:4 Bass:4 Alt:4`
- `fingers: 1:8 2:8 -:8 1:8 -:8 2:8 -:8 1:8`

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | 3 | | | 3 | | | | 3 |
| B | | 0 | | | | 0 | | |
| G | | | | | | | | |
| D | | | 0 | | | | 0 | |
| A | | | | | | | | |
| E | 3 | | | | 3 | | | |
| Rings | thumb notes a quarter each, under the finger notes (voice 2) | | | | | | | |

## 3. Alternating bass, "boom-chick" (C, 4/4)

IR: `strum: Bass:4 D Treble:4 Alt:4 D Treble:4`

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | | | 0 | | | | 0 | |
| B | | | 1 | | | | 1 | |
| G | | | 0 | | | | 0 | |
| D | | | | | 2 | | | |
| A | 3 | | | | | | | |
| E | | | | | | | | |
| Marks | | | ↓ | | | | ↓ | |

## 4. Folk strumming, "down, down-up, up-down-up" (C, 4/4)

IR: `strum: D Strum:4 D Strum:8 U Strum:8 -:8 U Strum:8 D Strum:8 U Strum:8`

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | 0 | | 0 | 0 | | 0 | 0 | 0 |
| B | 1 | | 1 | 1 | | 1 | 1 | 1 |
| G | 0 | | 0 | 0 | | 0 | 0 | 0 |
| D | 2 | | 2 | 2 | | 2 | 2 | 2 |
| A | 3 | | 3 | 3 | | 3 | 3 | 3 |
| E | | | | | | | | |
| Marks | ↓ | | ↓ | ↑ | | ↑ | ↓ | ↑ |

Question for review: should up-strokes hit every string, or only the treble strings (for example `U Treble(4)`)?

## 5. Down and up strokes on eighths (G, 4/4)

IR: `strum: D Strum:8 U Strum:8 D Strum:8 U Strum:8 D Strum:8 U Strum:8 D Strum:8 U Strum:8`

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 |
| B | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| G | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| D | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| A | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 |
| E | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 |
| Marks | ↓ | ↑ | ↓ | ↑ | ↓ | ↑ | ↓ | ↑ |

## 6. Waltz, "boom-chick-chick" (G, 3/4)

IR: `strum: Bass:4 D Treble:4 D Treble:4` (meter 3/4)

| | 1 | & | 2 | & | 3 | & |
|---|---|---|---|---|---|---|
| e | | | 3 | | 3 | |
| B | | | 0 | | 0 | |
| G | | | 0 | | 0 | |
| D | | | | | | |
| A | | | | | | |
| E | 3 | | | | | |
| Marks | | | ↓ | | ↓ | |

## 7. Bossa-style (Am, 4/4)

IR, two lanes:
- `thumb: Bass:4. Alt:8 Bass:4. Alt:8`
- `fingers: Treble:8 -:4 Treble:8 -:4 Treble:8 -:8`

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | 0 | | | 0 | | | 0 | |
| B | 1 | | | 1 | | | 1 | |
| G | 2 | | | 2 | | | 2 | |
| D | | | | 2 | | | | 2 |
| A | 0 | | | | 0 | | | |
| E | | | | | | | | |
| Rings | bass: dotted quarter on 1 and on 3, eighth on 2& and on 4& | | | | | | | |

Question for review: bossa right-hand figures vary a lot (often written in 2/4 with a different finger rhythm). Please replace this one with the figure you play.

## 8. Palm-muted rhythm (G, 4/4)

IR: `strum: [pm >Bass:8 Bass:8 Bass:8 Bass:8 >Bass:8 Bass:8 Bass:8 Bass:8]` (one span over all eight steps)

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | | | | | | | | |
| B | | | | | | | | |
| G | | | | | | | | |
| D | | | | | | | | |
| A | | | | | | | | |
| E | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 |
| Marks | PM > | PM | PM | PM | PM > | PM | PM | PM |

The TAB shows one "P.M." line over the whole bar (ADR-035 section 5); the table repeats PM per step only for clarity.

## 9. Percussive "boom-chuck" (C, 4/4)

IR: `strum: D Strum:4 D >x(Strum):4 D Strum:8 U Strum:8 D >x(Strum):4`

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | 0 | | x | | 0 | 0 | x | |
| B | 1 | | x | | 1 | 1 | x | |
| G | 0 | | x | | 0 | 0 | x | |
| D | 2 | | x | | 2 | 2 | x | |
| A | 3 | | x | | 3 | 3 | x | |
| E | | | | | | | | |
| Marks | ↓ | | ↓ > S | | ↓ | ↑ | ↓ > S | |

Under choice 13.4 of ADR-034, a percussive hit may also strike the muted string 6. `Strum` resolves to the sounded strings only, so string 6 stays empty here.

## 10. The user's example: `Bass treb | Bass treb treb` (C, 2/4, two bars)

Source: the user, 2026-11-09, with these durations: bar 1 is Bass 1/4 and Treble 1/4; bar 2 is Bass 1/4, Treble 1/8 and Treble 1/8.

IR: `strum: Bass:4 Treble:4 | Bass:4 Treble:8 Treble:8` (meter 2/4, length 2 bars)

| | 1 | & | 2 | & | ‖ 1 | & | 2 | & |
|---|---|---|---|---|---|---|---|---|
| e | | | 0 | | | | 0 | 0 |
| B | | | 1 | | | | 1 | 1 |
| G | | | 0 | | | | 0 | 0 |
| D | | | | | | | | |
| A | 3 | | | | 3 | | | |
| E | | | | | | | | |

If the three notes of the second bar were meant as equal parts of one beat, that would be a triplet. It would be written with tuplet values, never inferred from the step count (Q-03).

## What the user should send back

For each pattern 1 to 9: **confirm**, **correct** (the meaning, the IR, or the TAB), or **drop** it. Also add any pattern you play that is missing. Only confirmed patterns become goldens.
