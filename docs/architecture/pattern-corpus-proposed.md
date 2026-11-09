# Pattern corpus (Phase 6a)

Purpose: The reference patterns for the pattern engine ([ADR-034](decisions/ADR-034-pattern-engine-ir-first.md)). Each pattern states its intended meaning and the TAB it should produce on one voicing. The patterns the user has confirmed become the golden fixtures of criterion PE-11.
Status: Reviewed and approved by the user on 2026-11-09, with ADR-034.
- Confirmed: 1, 2, 3, 6, 8 and 10.
- Confirmed with the user's corrections applied: 4, 5 and 9 (9 with amendment B).
- **Not confirmed:** 7, a deferred candidate that is not a golden.



Related documents: [ADR-034](decisions/ADR-034-pattern-engine-ir-first.md), [ADR-035](decisions/ADR-035-playing-techniques-in-the-score.md), [guitar-pattern-engine.md](guitar-pattern-engine.md).

**Authorship and scope.**
- Patterns 1 to 9 were proposed by the agent. The user reviewed them and confirmed or corrected their meaning.
- Pattern 10 is the user's own example.
- The confirmed patterns are reference fixtures for the semantics ADR-034 and ADR-035 define. They are not definitive representations of a musical genre.

## How to read this

- **Voicings** (string 1 is the highest, Q-02; `x` = muted). The regions follow ADR-034 section 5: Bass B, then the upper region U (the 3 highest sounded strings above B), then Alt.

  | Chord | Voicing (strings 6 to 1) | Bass | Upper region U | Alt |
  |---|---|---|---|---|
  | C | `x 3 2 0 1 0` | string 5, fret 3 (C) | strings 3, 2, 1 | string 4, fret 2 (E, the third: the only candidate) |
  | G | `3 2 0 0 0 3` | string 6, fret 3 (G) | strings 3, 2, 1 | string 4, open (D, the fifth; preferred over B on string 5) |
  | Am | `x 0 2 2 1 0` | string 5, open (A) | strings 3, 2, 1 | string 4, fret 2 (E, the fifth) |
  | D (binding fixture only) | `x x 0 2 3 2` | string 4, open (D) | strings 3, 2, 1 | none: `NoAlternateBass` |

- **IR lines** use the DSL sketch of ADR-034 section 9 only to state the meaning; the grammar is not final.
  - Each step is `target:value`: `4` quarter, `8` eighth, `4.` dotted quarter.
  - `-` is a rest; `+` strikes together; `>` accents; `D` and `U` are strokes.
  - `[pm ...]` is a palm-mute span; `sp(...)` is a string-percussive hit (ADR-035).
  - `thumb:` and `fingers:` are the two lanes; `strum:` is the single strum lane.
- **TAB tables:**
  - The columns are the eighth-note positions of the bar, and the rows are strings 1 (e) to 6 (E).
  - A number is the fret struck at that position, `x` is a string-percussive note on that string, and an empty cell means nothing new is struck.
  - The "Marks" row shows ↓ ↑ (stroke), `>` (accent), `PM` (palm mute) and `S` (string-percussive).
  - The "Rings" row gives the written duration of long notes.

## 1. Basic fingerpicking (Am, 4/4) — confirmed

IR: `fingers: Bass:8 3:8 2:8 3:8 1:8 3:8 2:8 3:8` (one lane, every note one eighth)

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | | | | | 0 | | | |
| B | | | 1 | | | | 1 | |
| G | | 2 | | 2 | | 2 | | 2 |
| D | | | | | | | | |
| A | 0 | | | | | | | |
| E | | | | | | | | |

## 2. Travis-style picking (G, 4/4) — confirmed

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

## 3. Alternating bass, "boom-chick" (C, 4/4) — confirmed

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

## 4. Folk strumming, "down, down-up, up-down-up" (C, 4/4) — confirmed with correction

Correction from the user: up-strokes play `Treble(4)`, not the full `Strum`.

IR: `strum: D Strum:4 D Strum:8 U Treble(4):8 -:8 U Treble(4):8 D Strum:8 U Treble(4):8`

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | 0 | | 0 | 0 | | 0 | 0 | 0 |
| B | 1 | | 1 | 1 | | 1 | 1 | 1 |
| G | 0 | | 0 | 0 | | 0 | 0 | 0 |
| D | 2 | | 2 | 2 | | 2 | 2 | 2 |
| A | 3 | | 3 | | | | 3 | |
| E | | | | | | | | |
| Marks | ↓ | | ↓ | ↑ | | ↑ | ↓ | ↑ |

- Beat 3 is a **true rest**: no attack and no motion. A silent stroke motion (a "ghost" strum) is not a rest. If it is needed later, it becomes the action `StrokeOnly(Direction)` (ADR-034 section 10), and it is not part of this pattern.
- Each down-stroke is written as a quarter or an eighth, so its notes stop at the next attack. Let ring is not used here.

## 5. Down and up strokes on eighths (G, 4/4) — confirmed with correction

Correction from the user: up-strokes play `Treble(4)`.

IR: `strum: D Strum:8 U Treble(4):8 D Strum:8 U Treble(4):8 D Strum:8 U Treble(4):8 D Strum:8 U Treble(4):8`

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 |
| B | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| G | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| D | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| A | 2 | | 2 | | 2 | | 2 | |
| E | 3 | | 3 | | 3 | | 3 | |
| Marks | ↓ | ↑ | ↓ | ↑ | ↓ | ↑ | ↓ | ↑ |

## 6. Waltz, "boom-chick-chick" (G, 3/4) — confirmed

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

## 7. Bossa-style candidate (Am, 4/4) — NOT CONFIRMED (deferred)

This is a deferred candidate kept for later discussion. **It is not a golden fixture** (PE-11).

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

Right-hand figures for this style vary a lot. This one waits for the user's own figure.

## 8. Palm-muted bass ostinato (G, 4/4) — confirmed (renamed)

IR: `strum: [pm >Bass:8 Bass:8 Bass:8 Bass:8 >Bass:8 Bass:8 Bass:8 Bass:8]` (one palm-mute span over all eight steps)

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | | | | | | | | |
| B | | | | | | | | |
| G | | | | | | | | |
| D | | | | | | | | |
| A | | | | | | | | |
| E | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 |
| Marks | PM > | PM | PM | PM | PM > | PM | PM | PM |

The TAB shows one "P.M." line over the whole bar (ADR-035 section 5). The table repeats PM per step only for clarity.

Playback (ADR-035 section 4): every note is at velocity 80 and 1/16 long, so it stops before the next eighth. The accented notes are at velocity 104.

## 9. String-percussive boom-chuck (C, 4/4) — confirmed with correction

Correction from the user:
- The "chuck" is a string-percussive (deadened) chord hit on the **sounded** strings of the voicing, not a body slap.
- Each such note keeps its fretted position and pitch (ADR-035 `string_percussive`).
- No muted string is added: string 6, muted in C, stays empty.

IR: `strum: D Strum:4 D >sp(Strum):4 D Strum:8 U Treble(4):8 D >sp(Strum):4`

| | 1 | & | 2 | & | 3 | & | 4 | & |
|---|---|---|---|---|---|---|---|---|
| e | 0 | | x | | 0 | 0 | x | |
| B | 1 | | x | | 1 | 1 | x | |
| G | 0 | | x | | 0 | 0 | x | |
| D | 2 | | x | | 2 | 2 | x | |
| A | 3 | | x | | 3 | | x | |
| E | | | | | | | | |
| Marks | ↓ | | ↓ > S | | ↓ | ↑ | ↓ > S | |

- The `x` notes are on the frets of the voicing, so the stored note is string 1 fret 0, string 2 fret 1, and so on, each with `string_percussive`. TAB draws them as `x`.
- Playback: velocity 70 + 24 = 94, 1/64 long.
- The up-stroke on 3& is `U Treble(4)`, consistent with patterns 4 and 5 (user amendment B, 2026-11-09). The bass string 5 is not struck on it.

## 10. The user's example: `Bass treb | Bass treb treb` (C, 2/4, two bars) — confirmed

Source: the user, 2026-11-09, with exactly these durations: bar 1 is Bass 1/4 and Treble 1/4; bar 2 is Bass 1/4, Treble 1/8 and Treble 1/8.

IR: `strum: Bass:4 Treble:4 | Bass:4 Treble:8 Treble:8` (meter 2/4, length 2 bars)

| | 1 | & | 2 | & | ‖ 1 | & | 2 | & |
|---|---|---|---|---|---|---|---|---|
| e | | | 0 | | | | 0 | 0 |
| B | | | 1 | | | | 1 | 1 |
| G | | | 0 | | | | 0 | 0 |
| D | | | | | | | | |
| A | 3 | | | | 3 | | | |
| E | | | | | | | | |

If the three notes of bar 2 were meant as equal parts of one beat, that would be a triplet. It would be written with tuplet values, never inferred from the step count (Q-03).
