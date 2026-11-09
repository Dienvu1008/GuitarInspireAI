# ADR-035 Playing techniques in the score: accent, palm mute, string-percussive, stroke direction (Phase 6a-1)

Status: Accepted (2026-11-09).
- The acceptance criteria in section 7 were fixed before any code.
- The user confirmed the decisions in section 8 on revision 1 (`8e3c8ee`) and approved revision 2 (`a20dbe3`). Implemented in slice 6a-1.
Date: 2026-11-09
Related documents: [ADR-034](ADR-034-pattern-engine-ir-first.md), [ADR-024](ADR-024-playback-rendering-conventions-and-rt-failures.md), [ADR-030](ADR-030-editor-debt-clipboard-and-session-marker.md), [ADR-033](ADR-033-musicxml-export-adapter.md), [project-format.md](../project-format.md), [domain-invariants.md](../domain-invariants.md)

## Context

The user asked that patterns carry palm mute, dead note, let ring, slap or percussive hit, accent and down or up strokes. These must survive into the score, and playback and TAB must agree on what they mean (2026-11-09). They are techniques in the model, not player-specific words.

Findings from the code:

- **A note has only `tie`, `let_ring` and `dead`** (`mc_score::NoteEffects`). A beat has no stroke direction. Nothing in `mc-score`, `mc-project` (format 1.0), `mc-playback`, `mc-notation`, `mc-musicxml`, the clipboard (ADR-030), the commands or the inspector knows accent, palm mute, string-percussive or direction.
- **Playback today** (ADR-024):
  - Normal velocity 96.
  - A dead note is 1/64 long at velocity 40.
  - Let ring sounds to the end of the played bar.
  - If a note is both dead and let ring, dead wins.
- **The TAB display list** carries `dead` and `let_ring` on the fret glyph; Flutter only paints it (ADR-018).
- **Format policy:** additive optional fields are a minor version (project-format section 5). An older reader sees newer-minor content as unknown (U1) and protects the original with Save As (U2).
- **Techniques are useful without patterns:** users write palm-muted riffs and accented strums by hand. So this is a score change for everyone, landed before the pattern engine (ADR-034 slice 6a-1).

## Decision (proposed)

### 1. Model (`mc-score`)

| Technique | Level | Field |
|---|---|---|
| Accent | Note | `NoteEffects.accent: bool` |
| Palm mute | Note | `NoteEffects.palm_mute: bool` |
| String-percussive hit | Note | `NoteEffects.string_percussive: bool` (decision 8.2) |
| Dead note | Note | existing `NoteEffects.dead` |
| Let ring | Note | existing `NoteEffects.let_ring` |
| Stroke direction | Beat | `Beat.stroke: Option<Stroke>`, where `Stroke` is `Down` or `Up` |

- **String-percussive** is a percussive attack on a string that keeps its fretted position and its sounding pitch. It is not a body slap or golpe; those need their own event model and ADR (decision 8.2).
- Techniques are per note, so a chord can be partly muted.
- Direction is per beat, because a stroke moves across the whole chord.
- **No combination is rejected.** Released files may already hold dead and let ring together, and an invariant cannot reject them. Instead the sound follows a fixed precedence: string-percussive > dead > palm mute > let ring > normal. Accent applies on top of whichever wins.
- Every stored flag is kept, saved and shown in the editor, whatever the precedence.

### 2. Commands and editing (`mc-app`, editor)

- `SetNoteEffects` carries all five flags.
- A new `SetBeatStroke { beat, stroke }` sets the direction.
- Both are undoable, and both bump the revision.
- The clipboard (`ClipBeat`, `ClipNote`, ADR-030) carries the new flags and the stroke, so copy and paste keep them.
- MIDI import sets them to their defaults (off). MIDI does not carry them.
- The inspector gains toggles for accent, palm mute and string-percussive, and a Down / none / Up choice for the stroke.
- No new musical shortcuts (decision 8.4). Every new control works with standard keyboard navigation and activation (Tab, Space or Enter, arrow keys), with semantic labels.
- Dart only shows the flags and sends the commands.

### 3. Project format 1.1 (`mc-project`)

- `format_version` becomes 1.1.
- The note DTO gains optional `accent`, `palm_mute` and `string_percussive`; the beat DTO gains an optional `stroke`. All are omitted when off.
- A 1.0 file loads unchanged, with every new flag off.
- A 1.1 file that uses none of the new fields is byte-identical to what 1.0 wrote, except the version.
- A new fixture `v1.1/techniques`, a schema snapshot (FF-32) and round-trip tests (FF-11) are added. The 1.0 fixtures are untouched (append-only).
- A 1.0 app opening a 1.1 file reports newer-minor content (U1) and refuses Save over it (U2), as the policy defines.

### 4. Playback (`mc-playback`, extends ADR-024)

| Winning technique | Velocity | Note-off |
|---|---|---|
| Normal | 96 | end of the beat (as today) |
| Let ring | 96 | end of the played bar (as today) |
| Palm mute | 80 | the earlier of the beat end and onset + 1/16 whole note |
| Dead | 40 | the earlier of the beat end and onset + 1/64 (as today) |
| String-percussive | 70 | the earlier of the beat end and onset + 1/64 |

- A technique's length limit is applied as the earlier of the limit and the written beat end.
- **Accent** adds 24 to the winning technique's velocity, capped at 127 (normal 96 becomes 120; let ring 96 becomes 120; palm mute 80 becomes 104; dead 40 becomes 64; string-percussive 70 becomes 94).
- **Stroke direction** changes no timing in 6a: all notes of a beat start together, with no strum spread (decision 8.3).
- String-percussive notes sound with the bundled guitar SoundFont at their fret pitch. There is no drum sound.
- MIDI export uses the same compiled timeline, so it carries these velocities and lengths.
- Existing scores without the new flags compile to the same timeline as before; the timeline golden is unchanged.

### 5. TAB display (`mc-notation` display list, painted by Flutter)

- **Accent:** a `>` glyph above the beat, when any note of the beat is accented.
- **Palm mute:**
  - A "P.M." label with a dashed line over each run of consecutive beats of a voice in which every sounding note is palm-muted.
  - The line ends at the last such beat and is broken at system ends.
  - A beat where only some notes are palm-muted gets a "P.M." on that beat only.
- **Stroke:** a down arrow (↓) or up arrow (↑) above the beat.
- **String-percussive:** the fret glyph is drawn as `x` (as for dead), with a small "S" above the beat (decision 8.2).
- **Dead and let ring:** unchanged.
- Glyph positions are decided in Rust. Painting only draws them (ADR-018).
- Layout goldens gain one score that uses every technique.

### 6. MusicXML (`mc-musicxml`, extends ADR-033)

- Each technique maps to the element the MusicXML 4.0 specification defines for it. **The mapping is verified against the specification and the XSD in 6a-1, not assumed here.** Candidates to check:
  - `<accent/>` in `<articulations>`
  - `<down-bow/>` and `<up-bow/>`, or `<arpeggiate direction>`, for strokes
  - a palm-mute element if 4.0 has one
- **A string-percussive note is not mapped to `<golpe/>`**, which means tapping the pickguard (user review).
- A technique with no element is written as `<other-technical>` text. It is counted in the report as approximated.
- The ADR results record, for each technique: the exact mapping with its reference page, what MuseScore does with it, and any approximation.
- The schema job (E1) and the MuseScore procedure (E8, E9) run on the new fixture.

## 7. Acceptance criteria (fixed before implementation)

Single machine (ADR-025). Thresholds are not lowered to match a result.

| Id | Criterion | How it is checked |
|---|---|---|
| TQ-1 | Every new flag and the stroke can be set, cleared, undone and redone; each change is one revision; nothing else changes | Session tests |
| TQ-2 | Copy and paste keep every flag and the stroke (ADR-030 clipboard) | Session and property tests |
| TQ-3 | Format 1.1: every flag round-trips byte for byte; a 1.0 fixture loads with the flags off; a 1.1 file without new content differs from 1.0 output only in the version; the `mc-cli` built from the M5 release (`2fe101f`, format 1.0) opens the 1.1 fixture, reports newer-minor unknown content (U1) and refuses to save over it (U2); released fixtures unchanged; new fixture and schema snapshot | `mc-project` tests, FF-11, FF-32 |
| TQ-4 | Playback follows section 4 exactly: velocity and note-off per technique (each limit applied as the earlier of the limit and the beat end), precedence, accent offset with cap; every stored flag stays stored when another technique wins; scores without new flags give the same timeline as before (golden unchanged) | Timeline tests per technique, property test over random flags, golden |
| TQ-5 | MIDI export carries the same velocities and lengths as playback | `mc-midi` test |
| TQ-6 | TAB: accent, palm-mute runs (broken at system ends), stroke arrows and string-percussive marks appear where section 5 says; a partly muted beat shows "P.M." on that beat only | Layout goldens plus targeted tests |
| TQ-7 | MusicXML: each technique is written as the specification defines; the mapping is cited in the ADR results with the reference page; no string-percussive note is written as `<golpe/>`; approximations are counted; the schema job passes on the new fixture | `mc-musicxml` tests, CI schema job |
| TQ-8 | MuseScore 4.7.5 opens the export of the new fixture; for each technique the result after its MusicXML re-export is recorded (kept or lost). A loss in MuseScore is an evidence gap, not a pass | `tools/musescore-check` run, results file |
| TQ-9 | The inspector shows and toggles every flag and the stroke by mouse and by standard keyboard navigation and activation, with semantic labels; no new musical shortcut is added; the editor sends one command per change | Widget tests |
| TQ-10 | MIDI import leaves every new flag off; no other import or export path drops a flag silently | Tests |
| TQ-11 | No music rule in Dart; FF-01, FF-03, FF-05, FF-12, FF-22 (additions only) and FF-34 pass | Fitness functions, CI |

## 8. Decisions (user review of revision 1, 2026-11-09)

1. **Render constants confirmed** as the initial convention:
   - normal 96;
   - accent +24, capped at 127, applied to the winning technique's velocity;
   - palm mute 80, at most 1/16 whole note;
   - dead 40, at most 1/64;
   - string-percussive 70, at most 1/64;
   - let ring 96, sounding to the end of the played bar.

   Each limit is the earlier of the limit and the written beat end.
2. **String-percussive confirmed** as a per-note string technique with a fretted position and a sounding pitch. TAB shows `x` plus `S`. The field is named `string_percussive`, to keep it apart from body percussion. Body slap and golpe need a separate event model and ADR and are never represented by this flag.
3. **Strum spread deferred.** The notes of one stroke start together in 6a.
4. **No new musical shortcuts in 6a-1.** The inspector stays fully usable with standard keyboard navigation and activation.
5. **Precedence confirmed:** string-percussive > dead > palm mute > let ring > normal. Accent applies on top of the winner. Every stored flag is kept when another takes precedence.
6. **MusicXML:** TQ-7 kept. Each mapping is verified against MusicXML 4.0 and the XSD. A string-percussive note is never `<golpe/>`. The exact mapping, MuseScore's behaviour and any approximation go into the results.

## 9. Results (slice 6a-1, 2026-11-09)

### MusicXML mapping (verified against the MusicXML 4.0 reference)

| Technique | Written as | Reference |
|---|---|---|
| Accent | `<notations><articulations><accent/></articulations>` on each accented note | [accent](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/accent/), [articulations](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/articulations/) |
| Down-stroke | `<technical><down-bow/>`, once per beat, on its first note | [down-bow](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/down-bow/): the reference says the symbol is also used for down-strokes on plucked instruments |
| Up-stroke | `<technical><up-bow/>`, as above | [up-bow](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/up-bow/) |
| Stroke on a rest | its own `<notations><technical><down-bow/>` (or `<up-bow/>`) on the rest | [technical](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/technical/) |
| Palm mute | `<technical><other-technical>palm mute</other-technical>`; counted in `techniques_approximated` | 4.0 has no palm-mute element; [other-technical](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/other-technical/) |
| String-percussive | `<notehead>x</notehead>` plus `<technical><other-technical>string percussive</other-technical>`; counted. Pitch, string and fret are kept | [notehead](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/notehead/), other-technical. Never [golpe](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/golpe/), which means tapping the pickguard |
| Dead, let ring | unchanged (ADR-033) | |

- `<arpeggiate>` was rejected for strokes: it means a rolled chord ([arpeggiate](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/arpeggiate/)), and 6a has no strum spread.
- Precedence is a playback rule. The file keeps every stored flag, so a dead note with let ring still has its let-ring tie.
- `<technical>` and `<notations>` allow their children in any order in the XSD. The schema job validates the new fixture export and the new golden `techniques.musicxml` (CI, E1).

### MuseScore 4.7.5 (TQ-8)

[benchmarks/musescore-musicxml.md](../../../benchmarks/musescore-musicxml.md), section TQ-8:
- Kept: up-strokes and down-strokes on notes, and the `x` notehead (MuseScore draws digits on TAB, as for dead notes).
- Accents: one accent per chord. A chord where only some notes are accented cannot be told apart from a fully accented one.
- **Lost: palm mute, string-percussive (MuseScore then reads the note as a dead note), and a stroke on a rest.** This is an evidence gap. The encoding is not changed to suit MuseScore.

### Evidence per criterion (local, reference machine)

| Id | Evidence | Result |
|---|---|---|
| TQ-1 | `mc-app/tests/techniques.rs`: all 32 flag combinations and the stroke values. Each change is one revision and one undo step, and equals the old score with only that change (aggregate comparison). Undo and redo restore each state. Mutation checks: removing the stroke assignment fails the test | Met |
| TQ-2 | `techniques.rs` (32 combinations times 3 stroke values, copy and paste into a second track); `clipboard.rs` property tests, whose random fragments now carry random flags and strokes (the first beat also compares the stroke). Mutation checks: dropping the stroke in session copy or in paste is caught | Met |
| TQ-3 | `mc-project/tests/roundtrip.rs`: the `techniques` round trip (flag counts in `score.json`, off flags omitted); v1.0 fixtures load equal to v1.1 with every flag off; the v1.0 and v1.1 `score.json` of `small` and `feature-rich` are byte-identical; v1.1 fixtures and schemas appended, 1.0 untouched. `mc-cli` built at `2fe101f` opens `v1.1/techniques.package` and reports 25 unknown items, with paths. Saving over the file fails ("cannot be saved back; use Save As") and leaves it byte-identical; Save As to a new file works | Met |
| TQ-4 | `mc-playback/tests/techniques.rs`: an exhaustive table (96 cases: 5 flags times 3 beat lengths), the cap, flags kept, the fixture render, and a stroke that changes no timing. The existing golden is unchanged | Met |
| TQ-5 | `mc-midi` `tq5_export_carries_the_playback_velocities_and_lengths`: SMF notes (channel, on, off, key, velocity) equal the timeline; velocities 96, 120, 80, 104, 94, 40 and 70 occur | Met |
| TQ-6 | `mc-notation` golden `techniques.txt` and three targeted tests (marks on their beats, runs over bar lines, a partial beat on its own, a break at the system end). Existing goldens unchanged | Met |
| TQ-7 | `mc-musicxml` `tq7_...`, plus the E3 checker extended to every flag and stroke on 200 random scores (coverage counted). Six mutation checks are all caught. The XSD schema job runs in CI | Met locally; CI schema job: see the 6a-1 report |
| TQ-8 | Above | MuseScore opens the file; **losses recorded (gap)** |
| TQ-9 | `editor_feature/test/inspector_test.dart`: mouse toggles, one command per change, Tab plus Space or Enter for every new control, semantic labels and checked states, stroke on a rest. Three of four mutations caught (the fourth removes a redundant guard; `SegmentedButton` already ignores the selected segment). No shortcut was added (`key_bindings.dart` unchanged) | Met |
| TQ-10 | `mc-midi` `tq10_import_leaves_every_technique_off`; the Dart contract test runs on the real core, so the FFI carries every flag and the stroke both ways; project, clipboard and MusicXML paths are covered above. MIDI export renders techniques as velocity and length, and carries no stroke direction (it has no sound in 6a). This is documented, not silent (known limitations) | Met |
| TQ-11 | Dart only maps flags to commands and paints glyphs. FF results: see the 6a-1 report (`cargo xtask fitness all`); FF-12 regenerated twice with 14 identical hashes; FF-22 snapshots additions only (mc-app +5, mc-notation +6, mc-playback +4, mc-project +32, mc-score +27 lines, 0 removed) | See report |

### Known limitations of 6a-1

- TAB marks (stroke, accent, `S`, "P.M.") sit in fixed rows above the staff. Metrics are unchanged, so the marks can overlap volta brackets and the stems of an upper voice.
- The MuseScore losses above.
- A stroke has no sound effect (no strum spread, decision 8.3), so MIDI export does not carry it.
## Alternatives

- **A body slap as this per-note flag.** Rejected by the user: a body hit has no string or pitch; it needs its own event model.
- **Techniques only in the pattern IR, not in the score.** Rejected: the user requires that nothing is lost in the score and that TAB and playback agree.
- **Accent and palm mute per beat.** Rejected: one note of a chord can be muted or accented alone.
- **Reject contradictory combinations.** Rejected: released 1.0 files may hold dead and let ring together, and an invariant cannot reject them.
- **A major format version.** Rejected: the change is additive; the policy calls for a minor version.

## Trade-offs

- Users of an older build cannot save over a 1.1 file; this is the policy, not a regression.
- Fixed render numbers are a convention. A user who wants a softer accent must wait for a setting.
- MusicXML and MuseScore coverage of these techniques is unknown until 6a-1 checks it.

## Consequences

- `mc-score` model and commands.
- `mc-app` commands and clipboard.
- `mc-project` format 1.1.
- `mc-playback` render table.
- `mc-notation` glyphs.
- `mc-musicxml` mapping.
- `mc-ffi` DTOs (bindings regenerated).
- The editor inspector.
- ADR-024 gains an amendment note for the render table.
- Known limitations and release notes are updated.

## Related fitness functions

FF-11, FF-12, FF-19, FF-22, FF-32, FF-34; the schema job of ADR-033.

## Reconsideration conditions

- A stroke-spread requirement.
- Body slap, golpe and other unpitched or muted-string percussion (a separate ADR).
- User-adjustable render settings.
- A technique that MusicXML or MuseScore cannot carry at all.
