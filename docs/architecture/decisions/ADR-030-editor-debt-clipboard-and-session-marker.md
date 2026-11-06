# ADR-030 Editor debt (Phase 6b-1): beat clipboard, close prompt and session marker

Status: Accepted (2026-11-06). Acceptance criteria were fixed before any code; the three user choices of section 5 were decided by the user on 2026-11-06. 6b-1 is done when it meets section 4.
Date: 2026-11-06
Related documents: [score-model-and-undo.md](../score-model-and-undo.md), [project-format.md](../project-format.md), [domain-invariants.md](../domain-invariants.md), [ADR-016](ADR-016-undo-structural-sharing-snapshots.md), [ADR-020](ADR-020-invariants-vs-diagnostics.md), [phase-3-exit-report.md](../../roadmap/phase-3-exit-report.md), [mvp-known-limitations.md](../../roadmap/mvp-known-limitations.md)

## Context

Phase 6b-1 closes the editor gaps the MVP shipped with: copy and paste inside a track (Phase 5 O-4), and the crash-recovery items of Phase 3 (O-1 to O-3): the session marker, "Open original", and autosave on close with a close prompt. Loop selection (Phase 4 O-2) is deliberately deferred by the user; per-track SoundFont choice is deferred and needs its own ADR.

Findings from the code (not assumptions):

- The core has no beat range and no clipboard. The only range edit is `ShiftFrets` over a bar range, which is atomic (all or nothing). Undo is snapshot based (ADR-016): one `execute` is one undo step.
- `RecoveryStore::begin_session` and `end_session` already exist in `mc-app` and are tested, but nothing in `mc-ffi` or Dart calls them, so recovery is offered whenever autosaves exist.
- The Windows runner and the Dart app handle no window close at all. New, Open and replace also drop unsaved work without asking.

## Decision

### 1. Clipboard: music rules in Rust, selection in Dart (GR-05)

- The **selection** (an anchor and a cursor, in one track and one voice) is pure UI state in `editor_feature`. It is never stored in the document and never part of undo.
- The **clipboard content** is a plain-value fragment: a sequence of beats, each with a `DurationSpec` and its notes (string, fret, tie flag, effects). Dart builds it from the beat data it already reads; it holds no ids. It lives at **app level**, not per document, so a fragment copied in one document can be pasted into another document (or another track) in the same running app.
- A fragment stores **strings and frets, not pitches**. Pasting into a track with a different tuning or capo keeps the same strings and frets, so the sounding pitch changes. This is intended and is stated in the user documentation.
- The selection is a **contiguous run of beats in one voice, possibly across bars**. The fragment is a flat beat list: bar lines, time signatures and tempo marks are not copied, and paste never creates bars. Selecting whole bars (keeping bar structure) is **not** part of 6b-1 and is recorded as deferred on purpose.
- **Paste is one new atomic core command** (`ScoreCommand::PasteBeats`), validated and applied in Rust. Dart never edits notes one by one to paste. A paste is exactly one undo step. A cut is copy plus one atomic range removal (`ScoreCommand::RemoveBeats`), also one undo step.
- Paste **inserts** at the cursor; it does not overwrite. Bars that become underfull or overfull are diagnostics and never block (ADR-020, Q-09). Paste never creates or removes master bars.
- A fragment note whose string number is higher than the target track's string count, or whose fret exceeds the target's fret count, makes the **whole paste fail** with a typed error and changes nothing.
- A tie whose partner note is no longer the same string and fret after the paste is cleared by the same normalisation `ShiftFrets` already uses (`normalize_ties`), and the removal is reported as an event.
- The clipboard is **in-app only** in 6b-1. Nothing is written to the operating-system clipboard: there is no interchange text format for TAB beats, and inventing one belongs to an ADR of its own.
- No change to the `.giproj` format: the clipboard and selection are not persisted, so no format version bump (project-format section 3).

### 2. Session marker and recovery

- `mc-ffi` exposes `begin_session(dir) -> bool` (true when the previous session ended uncleanly) and `end_session(dir)`, over the existing `FsRecoveryStore`. Dart wraps them on `DocumentFactory`. `mc-app` logic is unchanged; the file layout (`session.lock`, `source`, `autosave-*.recovery`) is unchanged, so old recovery files stay readable.
- The app calls `begin_session` at startup and `end_session` only on a **clean exit**.
- Recovery is offered **only when the marker says the last session was unclean**, and only for entries **newer than their source file** (compare the entry timestamp with the source's modification time; an untitled document has no source and is always offered). This fixes O-1.
- The recovery dialog offers **Restore**, **Open original** and **Discard** per entry (project-format section 12). "Open original" opens the source file and leaves the autosave until the user decides, so choosing it loses nothing. This fixes O-2.

### 3. Close prompt and autosave on close

- When the user closes the window, or chooses New or Open, with a dirty document, the app asks **Save, Don't save, Cancel**. Cancel keeps the app open. Save uses the existing save and save-as paths, then discards that document's autosaves. Don't save discards that document's autosaves. Only then does `end_session` run. After a clean exit no autosave of that document remains.
- When the system ends the session and no prompt is possible (log off, shutdown), the app writes an autosave **and leaves the marker in place**, so the next start offers recovery. This is what "autosave on close" means in project-format section 12.
- The window-close hook uses the Flutter exit request (`AppLifecycleListener.onExitRequested`). Whether the pinned Flutter version delivers a cancelable exit request on Windows is **not yet verified**; it is checked first in 6b-1 (section 4, B1). If it does not work, the fallback is handling `WM_CLOSE` in the stock C++ runner, which is not Rust and does not touch the `unsafe` rules (FF-34).
- The 30-second autosave poll and the 2-minute core interval stay as they are.

## 4. Acceptance criteria (fixed before implementation)

Measured on the reference machine; single-machine evidence (ADR-025). These are not lowered to match a result.

| Id | Criterion | How it is checked |
|---|---|---|
| C1 | A paste is exactly one undo step, and undo restores a score equal to the one before | Property test over random fragments and cursors (same style as FF-14) |
| C2 | A failed paste (string or fret out of range, stale revision) changes nothing: score, revision, history and events | Unit and property tests |
| C3 | Copy then paste of a range inside one track reproduces the same beats (durations, strings, frets, effects); ties are kept when the partner is also copied | Property test |
| C4 | The H invariants hold after every paste and cut | Existing invariant property tests extended to the new commands |
| C5 | Cut is one undo step and undo restores the removed beats exactly | Test |
| C6 | Closing with a dirty document and choosing Save, Don't save or Cancel behaves as in section 3; New and Open ask too | Widget tests with the fake service, and the Windows manual script |
| C7 | After a clean exit, recovery is not offered at the next start; after a simulated unclean exit (marker left) it is | Core tests with `FsRecoveryStore` plus a Dart test of the start-up check |
| C8 | Entries older than their source file are not offered; "Open original" opens the source and keeps the autosave | Tests |
| C9 | A forced session end writes an autosave and keeps the marker | Test of the handler with a fake lifecycle event |
| B1 | The pinned Flutter delivers a cancelable exit request on Windows from the window close button | Manual check in the built app, result recorded in the exit report; if it fails, the C++ fallback is used and recorded |
| C10 | No new `unsafe`, no new dependency, no format version bump; public API snapshots change by additions only; frb bindings regenerated by the codegen only (FF-12) | Fitness functions and CI |

A real process kill and a real log-off are manual steps in the exit-report script; the automated tests cover the logic with fakes. The exit report states this limit.

## 5. User decisions (2026-11-06)

1. **Selection scope: a contiguous run of beats in one voice, possibly across bars.** Flat beat list, no bar structure kept. Whole-bar selection is deferred on purpose: it changes master bars, which every track shares, and needs its own rules (a later slice or ADR).
2. **Paste inserts.** No overwrite mode in 6b-1.
3. **Clipboard is in-app only.** The user copies content inside the app, not from other programs. Nothing is written to the operating-system clipboard. Getting content in from other programs goes through MIDI import (Phase 4), and MusicXML import and Guitar Pro (6d) are not part of this slice.

## Alternatives

- Paste as a series of existing `InsertBeat` and `SetNote` commands from Dart with one coalesce key: rejected because validation and atomicity would depend on Dart (GR-05), and a failure half way would leave a half-pasted score.
- Keeping the selection in the core: rejected; it is view state and would add public API for no domain rule.
- Offering recovery whenever autosaves exist (today): rejected; it offers recovery after a normal close, which is the defect O-1.

## Trade-offs

An in-app clipboard means no paste into other programs, which is acceptable for an editor whose first interchange target is MusicXML. Insert-only paste can overfill a bar; that matches the project rule that bar capacity is a diagnostic, not a block.

## Consequences

- New `ScoreCommand` variants (`PasteBeats`, `RemoveBeats`), new `Score` methods, new DTOs in `mc-ffi`, regenerated bindings, updated public API snapshots (additions only).
- `DocumentFactory` gains session methods; the app gains a close handler and the new recovery dialog.
- `mvp-known-limitations.md` loses the recovery item and gains: loop selection (deferred on purpose), per-track SoundFont program (deferred, needs an ADR), whole-bar selection (deferred on purpose), the in-app-only clipboard, and the strings-and-frets (not pitch) behaviour of paste across tunings.

## Related fitness functions

FF-12 (bindings in sync), FF-13 and FF-14 (invariants, undo identity), FF-19 (trailers), FF-22 (public API), FF-34 (no new `unsafe`).

## Reconsideration conditions

A text interchange format for TAB, a second platform with its own exit semantics, or multi-document windows would change sections 1 and 3.

## Implementation notes (6b-1, 2026-11-06)

Recorded so the decision matches what was built. None of these lowers a criterion of section 4.

- **B1 result: pass.** On the reference machine, with Flutter 3.47.2 and the stock Windows runner:
  1. The release build was started and one note was typed, so there was unsaved work.
  2. The same message the title-bar close button sends (`WM_SYSCOMMAND` / `SC_CLOSE`) was posted to the window.
  3. The app stayed open and showed the "Save changes?" dialog. A screenshot was checked.
  4. While the dialog was open, the recovery folder held the new autosave and `session.lock`.
  5. Choosing "Don't save" closed the app with exit code 0 and left the recovery folder empty: the marker was removed and the autosave discarded.
  
  The `WM_CLOSE` fallback in the C++ runner is therefore not needed and was not added. Single machine (ADR-025).
- **The fragment is built by the core, not by Dart** (differs from section 1, which said Dart builds it from the beat data it reads).
  - `copy_beats` in `mc-score`/`mc-app` returns the plain-value fragment. Dart only keeps it in the app-level clipboard and sends it back with `PasteBeats`.
  - There is one conversion instead of two, and Dart never interprets beat structure (GR-05). Behaviour seen by the user is the same.
- **"Unsaved work" is a core rule.** `DocumentSession::has_unsaved_work` is `is_dirty`, except for a new, never-saved document that still equals what it was created as.
  - Such a document is dirty, because it is not on disk, but it holds nothing to lose. Closing it does not prompt.
  - Opened and recovered documents follow `is_dirty` exactly.
  - Autosave timing (`autosave_due`) is unchanged.
- **MIDI import also asks**, because it replaces the current document like Open.
- **C9 is met by ordering.** The prompt writes an autosave before it asks, and the marker is removed only after the prompt resolves to an exit. A system shutdown while the prompt is open therefore leaves both, and the next start offers the work.
  - A real log-off is still a manual step of the exit-report script.
