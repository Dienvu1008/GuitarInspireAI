# Phase 3 Exit Report

Purpose: Evidence against the Phase 3 acceptance criteria (Flutter shell, bridge and desktop TAB editor), edit-feedback measurements, defects found while making the Windows build and acceptance flow work (slice 3f), open points for the user, and the manual exploratory test script.
Status: Draft for user acceptance. All criteria have local evidence; CI status in section 8.
Related documents: [implementation-phases.md](implementation-phases.md), [milestones.md](milestones.md), [phase-2-exit-report.md](phase-2-exit-report.md), [performance-requirements.md](../architecture/performance-requirements.md), [project-format.md](../architecture/project-format.md), [cross-platform-ui.md](../architecture/cross-platform-ui.md), [ADR-022](../architecture/decisions/ADR-022-unsafe-confined-to-generated-ffi.md).

## 1. Outcome

Phase 3 delivered the Phase 3 roadmap scope in six slices on branch `phase-3-editor`: TAB layout in Rust (3a), the bridge and Dart facade (3b), the app shell and packages (3c), keyboard editing and the inspector (3d), autosave and recovery (3e), and the Windows app build with the acceptance flow (3f). No playback, MIDI, tuner, AI or other Phase 4+ features were added.

| Area | Where |
|---|---|
| TAB layout (display list, hit regions, goldens) | `mc-notation`; `DocumentSession::tab_layout` in `mc-app` |
| Bridge (document handle, events, layout, recovery) | `mc-ffi` (frb 2.13.0, ADR-005, ADR-022); generated bindings committed (FF-12) |
| Dart facade, fake and contract tests | `packages/core_bridge` |
| Native build plugin | `packages/mc_ffi` (vendored cargokit, two local patches, see its README) |
| Editor (controller, TAB view, track list, inspector, keyboard) | `packages/editor_feature` |
| Theme, bundled font | `packages/ui_kit` (Noto Sans Mono, ADR-021) |
| File dialogs, app data paths | `packages/app_platform` |
| Composition root, shell, startup recovery, autosave | `app/` (`main.dart`, `src/startup.dart`, `src/shell.dart`, `src/autosave.dart`) |
| Acceptance flow, edit benchmark | `app/integration_test/acceptance_test.dart`, `app/tool/edit_latency.dart` |

## 2. Acceptance criteria

| Criterion ([implementation-phases.md](implementation-phases.md) Phase 3) | Evidence | Result |
|---|---|---|
| Create a 2-track song | "Add track" in the track list (copies the active track's instrument; Dart decides nothing musical, GR-05). Integration test step 1; widget test `Add track adds a second track` | Met |
| Switch the active TAB track from the track list | Integration test (tap "Track 2", enter a note there, track 1 unchanged); widget tests in `editor_feature` and `app` | Met |
| Enter notes by keyboard | Integration test against the real core: "1","2" gives fret 12 on string 1 (one beat inserted), Right, Down, "3" gives a second beat on string 2; widget test for Q-15 coalescing | Met |
| Undo and redo | Integration test: one undo step per note including the beat it created; redo restores it | Met |
| Save, reopen identically | Integration test: Ctrl+S (first save asks for a location), Ctrl+N, Ctrl+O; every track name, tuning, beat duration and note compared before and after; reopened document is clean | Met |
| Recover after a forced kill | Integration test: edit, autosave, drop the whole app without saving, start again, recovery dialog, recover; content equal, document dirty, saving clears the recovery files. Rust tests `autosave_recover_and_discard_on_save` (mc-ffi) and `a_recovered_document_keeps_its_location_and_starts_unsaved` (mc-app); Dart real-core test. A real process kill is step 9 of the manual script (section 7); see open point O-1 | Met (simulated kill automated; real kill manual) |
| Edit feedback within budget on the typical score | Section 3: pessimistic p99 13.7 ms or less against 16 ms; no edit missed the next frame | Met on the development machine; reference machine run pending (O-6) |

Milestone M3 demo ("create a 2-track song in the app, save, kill the process, recover, reopen"): the integration test runs it end to end in the built Windows app against the real core; the manual script covers the same with a real process kill.

## 3. Edit feedback (performance-requirements: p99 <= 16 ms typical, <= 33 ms large)

Measured with `app/tool/edit_latency.dart` in a profile build of the real app (FFI, Dart refresh, widget rebuild, TAB painting, rasterization), 200 note edits after 30 warm-up edits, on the mc-score fixture shapes. Development machine, not a quiet reference machine; three runs each after all changes.

| Shape | Compute p50 (command + build + raster) | Pessimistic p99 (sum of the three p99) | Edits that missed the next frame |
|---|---|---|---|
| Typical (4 tracks x 150 bars x 8 beats x 2 notes) | 6.4 to 6.7 ms | 12.3 to 13.7 ms (budget 16) | 0 of 600 |
| Large (20 tracks x 300 bars x 16 beats x 3 notes) | 13.6 to 14.5 ms | 19.0 to 23.4 ms (budget 33) | 14 of 600 |

| Part (p99) | Typical | Large |
|---|---|---|
| Command round trip over FFI | 0.2 to 0.4 ms | 0.4 to 0.6 ms |
| Summary query | 0.14 to 0.22 ms | 0.10 to 0.15 ms |
| TAB layout query (whole active track) | 1.3 to 1.6 ms | 5.8 to 8.6 ms |
| Frame build (includes layout query and painting) | 5.4 to 6.3 ms | 10.3 to 13.1 ms |
| Frame raster | 6.6 to 7.3 ms | 8.3 to 9.9 ms |

The first measurement, before the 3f fixes, was over budget on the typical score (edit to frame p99 38.8 ms, frame build p99 16.1 ms). Four changes brought it within budget (section 4, items P-1 to P-4). Rust benches after the changes (`cargo bench -p mc-app --bench commands`): `edit_and_relayout` p99 0.72 ms typical and 3.9 ms large; note edit including the undo snapshot p99 15.3 µs typical and 87.7 µs large (Phase 2: 11.4 µs and 21.8 µs; the increase is the incremental diagnostics update, P-4).

Debug-mode key-to-state times in the integration test (91 ms first key including JIT warm-up, 44 ms later) are recorded for reference only; debug builds are not comparable with the budget.

## 4. Defects found and fixed in slice 3f

Making the Windows build and the acceptance flow work against the real core found defects that the fake-based widget tests could not see, mostly because the real core answers asynchronously.

| # | Defect | Fix |
|---|---|---|
| D-1 | Windows build: cargokit joined the manifest path onto the plugin symlink path; CMake does not resolve Windows directory symlinks, so the crate was not found | cargokit patch 1 (`packages/mc_ffi/README.md`) |
| D-2 | Windows build: cargokit looked for `mc-ffi.dll` (package name) instead of `mc_ffi.dll` (lib name), copied nothing and still exited 0; the failure surfaced later in the CMake INSTALL step | cargokit patch 2 |
| D-3 | The shell created a new `KeyboardHandler` per key event, so multi-digit fret entry (Q-15) never coalesced | One handler per shell, disposed with it |
| D-4 | Typing a digit on the append slot inserted a beat without awaiting it, then looked for the beat: with the real core no note was written; two fast keys could also act on stale state | Key actions run in order through a queue, each reading the state when it runs |
| D-5 | Right from the last beat jumped to the next bar, so the append slot of the current bar was unreachable and successive notes could not be typed without Insert | Right/Left step through the append slot; tested |
| D-6 | Delete on a beat without notes threw; Delete on an empty string removed a note on another string | Delete removes only the note on the cursor string |
| D-7 | No way to add a track in the UI (needed for the 2-track criterion) | "Add track" button in the track list |
| D-8 | The recovery dialog used a context above the `MaterialApp`, so it would throw on any start with recovery files | The startup gate gives the dialog its navigator context; widget tests |
| D-9 | "Discard all" deleted nothing (`discardRecovery('')`); discarding removed only the newest of the rotated autosaves | `discard_recovery(dir, project_id)` removes all autosaves of a project |
| D-10 | A recovered document opened as clean and untitled, and its recovery file was deleted right away: a second crash, or closing without noticing, lost the recovered work | `DocumentSession::recover`: keeps the original location, starts dirty, keeps autosaving; files stay until the next successful save |
| D-11 | A normal save never removed recovery files (project-format section 12 requires it), so a saved document was offered for recovery again | `save` and `save_as` in `mc-ffi` discard the project's recovery files after success |
| D-12 | Tab was mapped to an editor intent that did nothing, so the editor swallowed Tab and keyboard users could not move focus to the menus and buttons | Tab unmapped; widget test checks that it moves focus |

Performance changes (section 3):

| # | Cause | Change |
|---|---|---|
| P-1 | Each painter laid out every fret label again on every frame (cache per painter, new painters on every edit) | Shared, bounded label cache |
| P-2 | Every edit repainted every visible system: `SystemDto ==` compares lists by identity | `shouldRepaint` compares painted content; only the edited system repaints |
| P-3 | One command emits several events; each triggered a summary query and a rebuild, and the controller queried once more after the command | Events folded into one refresh per batch; refresh skipped when nothing visible changed |
| P-4 | `summary()` ran a full D-01 scan of the score (6.8 ms on the large score) | `Score::diagnostics_since` reuses results for bars shared with the previous snapshot (ADR-016); the session keeps diagnostics current on every change. New property test: equal to a full scan after random commands, undo and redo |

## 5. Fitness functions and tests

28 of 34 checks active (unchanged in 3f); `current_phase = 3`. Phase 3 activated FF-05, FF-06, FF-08 (Dart, `ci:dart-fitness`), FF-12 (bindings in sync) and FF-34 (unsafe scope, ADR-022). FF-29 (bundle size) moved to Phase 4 because it needs a release artifact.

| Id | Phase 3 evidence |
|---|---|
| FF-12 | Regenerating the bindings with `--no-dart-format` changes nothing; CI job `bridge` |
| FF-22 | Snapshots updated for `DocumentSession::recover` (mc-app) and `Score::diagnostics_since` (mc-score); no other API change |
| FF-34 | 9 crates, 93 source files; `allow(unsafe_code)` only on `mod frb_generated;` |
| FF-13, FF-14 | Unchanged and passing; a new property in the same suite checks incremental diagnostics |

Tests: Rust workspace 184 passing. Dart: core_bridge 18 (contract against fake and real core, recovery round trip), ui_kit 3, editor_feature 4, app 11 (shell, keyboard entry, Tab focus, add track, startup recovery, Dart fitness), plus 2 Windows integration tests against the real core.

CI gains a step in the `bridge` job: `flutter build windows --debug` and `flutter test integration_test -d windows` (protected path; `Architecture-Change: ADR-004`). The latency benchmark is not run in CI because runner timings are not comparable.

No ADR was reversed. No fitness function, lint or dependency rule was weakened.

## 6. Open points for the user

| # | Point | Proposal |
|---|---|---|
| O-1 | The session marker of project-format section 12 is not wired: recovery is offered whenever autosaves exist, not only after an unclean shutdown. Because a save removes them, this means "there was unsaved work at the last exit", which includes a normal close with unsaved changes (Phase 3 has no "save changes?" prompt on close) | Keep for Phase 3; add the close prompt and the session marker together in Phase 4 or 5 |
| O-2 | Recovery offers Recover and Discard all, not "Open original" (section 12) | Add with O-1 |
| O-3 | Autosave: the core interval is 2 minutes (section 12), Dart polls every 30 s; autosave on window close is not implemented | Add with O-1 |
| O-4 | A new, never-saved document counts as dirty (core semantics; the title shows `*`) | Keep: an untitled document holds unsaved work, and it autosaves |
| O-5 | The inspector offers Tie on every note; the core rejects an invalid tie and the status bar shows the rule | Keep for Phase 3; disable the option from a core query later |
| O-6 | Edit-feedback numbers are from the development machine | Run `app/tool/edit_latency.dart` on the reference machine before Phase 4 planning |
| O-7 | The whole active track is laid out again on every revision (1.3 ms typical, up to 8.6 ms large) | Within budget; incremental layout only if Phase 4 needs the headroom |
| O-8 | cargokit patches are local; the Linux CMake file and podspec keep upstream behaviour and are unverified (Windows-only MVP, Q-13) | Verify when another platform is in scope |

## 7. Manual exploratory test script (Windows)

Run after deleting `%APPDATA%\dev.guitarinspire\guitar_inspire\recovery` (per-user app data from `SystemAppPaths`; company and product names come from `app/windows/runner/Runner.rc`). Build: `flutter build windows --debug` in `app/`, then start `app\build\windows\x64\runner\Debug\guitar_inspire.exe`.

1. The window shows the menu bar, the track list with "Guitar", an empty TAB with 4 bars, the inspector ("No beat selected") and `* Untitled`.
2. Type `1` `2`: bar 1 shows fret 12 on the top string; the status bar shows the cursor on beat 1. Wait a second, type `3`: the fret becomes 3 (new step after the 600 ms window).
3. Right, Down, `5`, Right, Down, `7`: two more beats on strings 2 and 3. Left onto a beat and press `E`: it becomes an eighth; `.` toggles the dot. Bars that do not add up to the time signature (D-01) are tinted and counted in the status bar.
4. Ctrl+Z three times, Ctrl+Y three times: notes disappear and return one at a time.
5. Click "Add track", then "Track 2": the TAB switches to the empty track; enter `0` `Right` `2`. Click "Guitar": the first track's notes are unchanged.
6. Click a fret number with the mouse: the cursor and the inspector follow; toggle "Let ring" and "Dead note".
7. Ctrl+S: a save dialog appears; save as `m3.giproj`. The title shows `m3.giproj` without `*`.
8. Ctrl+N, then Ctrl+O and open `m3.giproj`: both tracks and all notes are back.
9. Forced kill: make one more edit (title shows `*`), wait at least 2.5 minutes (autosave interval 2 min plus one 30 s poll), then end the process in Task Manager (or `Stop-Process -Name guitar_inspire -Force`).
10. Start the app: "Recover documents" lists `m3.giproj`. Choose it: the last edit is there and the title shows `* m3.giproj`. Ctrl+S saves over the original; restart: no recovery prompt.
11. Repeat step 9, then choose "Discard all" on the next start: the app opens an empty document; restart again: no prompt.
12. Keyboard only: Tab moves focus to the menus and buttons and back; every control has a visible label. (A full WCAG review needs assistive-technology testing; not part of this script.)

## 8. CI status

Local runs of every CI step passed on the final 3f state: fmt, clippy `-D warnings`, workspace tests (184), benchmark build and command benchmarks, `xtask fitness all --base d9fd26d`, cargo-deny, FF-12 regeneration, Dart analysis and tests in every package, `flutter build windows --debug`, and the integration tests. Slices 3a to 3e passed GitHub Actions (last: `d9fd26d`). The 3f commit is verified on its push.

## 9. Closure

Phase 3 closes when the user accepts this report (or decides the open points differently) and `phase-3-editor` is merged into `main` with CI passing. Next phase per the roadmap: Phase 4 (playback and MIDI), on a new branch.
