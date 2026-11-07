# MVP Known Limitations

Purpose: List what the MVP (milestone M5, end of Phase 5) does not do or does only
partially, so users and the next phases know the boundaries. This is the "known
limitations list" the MVP exit criteria require ([roadmap/README.md](README.md) section 1).
Status: For the M5 build. Updated as items are addressed in later phases.
Related documents: [README.md](README.md), [implementation-phases.md](implementation-phases.md), [phase-5-exit-report.md](phase-5-exit-report.md), [risk-register.md](risk-register.md).

## Platform and evidence

- Windows 10/11 x64 only (Q-13). No macOS, Linux, mobile or Web build.
- All performance, latency, glitch and tuner-accuracy numbers are from one machine
  (ADR-025, R-21 open). A second machine is deferred; behaviour on other hardware,
  lower-end CPUs or different audio devices is unverified.

## Tuner

- The tuner's accuracy is confirmed on **synthetic** signals (within ~0.02 cents across
  E2..E6, allocation-free). **Real-guitar recordings have not been measured**: the
  development environment cannot capture a physical instrument, so [ADR-029](../architecture/decisions/ADR-029-tuner-pitch-detection.md)
  stays in Draft and the MPM-vs-YIN choice is provisional (MPM). A web virtual guitar was
  considered and rejected as not representative of real microphone input. Expect the low
  strings (E2, A2) to be the least robust with a noisy signal (R-11). The pre-set pass/fail
  thresholds (ADR-029 §2) are applied as-is and are not relaxed to match a contributed
  set. **If you have a guitar, you can help close this gap:** the capture harness and
  step-by-step procedure are in `spikes/s3-tuner/results/README.md`; recording the six
  open strings and running the analysis fills ADR-029 §5 and lets the tuner move from
  Draft to Accepted.
- Default input device only; no microphone picker (parallels playback's default-output
  rule). If there is no microphone or access is denied, the tuner shows a message and the
  rest of the app keeps working.
- Reference pitch A4 = 440 Hz, not configurable. No polyphonic tuning.

## Playback

- Default output device only; device selection is deferred. Metronome and count-in are
  not implemented (deferred from Phase 4). Loop selection is **deferred on purpose**
  (user decision, Phase 6): it needs its own interaction design (choosing a range,
  showing it, keeping it in step with the transport); it is not an oversight.
- One fixed instrument sound for every track (Acoustic Guitar steel, decided in Phase 4).
  A per-track SoundFont program is **deferred and needs an ADR**, because it changes
  both playback and MIDI export. The playback cursor follows the
  engine position but does not correct for the ~33 ms output latency, so it leads the
  sound by under a frame.
- One bundled SoundFont (trimmed FluidR3, program 25). User-supplied SoundFonts are a
  future capability. If the SoundFont is missing, playback is disabled with a message (no
  sine fallback).

## Editor and notation

- Several TAB tracks can be shown at once (Phase 6b-2, ADR-031): one stave per shown
  track in each line of music, with bar lines and simultaneous notes lined up. Editing,
  the cursor, the selection and the inspector act on the **active track** only; clicking
  another stave makes it active. Which tracks are shown is not saved: it resets when a
  document is opened.
- A document opens with at most **16 tracks shown**; the others start hidden and are
  shown from the track list (eye button). Showing all of them works, only slower.
- **Large scores with many tracks may feel slow when editing; this is not yet
  confirmed either way.**
  - On one day the reference machine kept a note edit on the large test score (20 tracks
    of 300 bars) within its 33 ms budget with up to 16 shown tracks, but not with 20.
  - On the next day the same, unchanged code measured 42 to 50 ms with 16 tracks on the
    same machine, for a reason not yet found (ADR-031).
  - Typical scores (4 tracks) were within budget on both days. One machine only (ADR-025).
- The active track cannot be hidden. Tracks added later are shown.
- No standard notation, no PDF.
- Effects are limited to tie, let ring and dead note. No lyrics. Repeat editing is limited
  to simple start and end markers.
- A pickup bar round-trips through MIDI as a full-length bar with a trailing rest.
- Copy and paste (Phase 6b-1, ADR-030) work on a run of beats in one voice, possibly
  across bars, selected with Shift+Left/Right or Shift+click. Paste inserts; there is no
  overwrite mode. Bar lines, time signatures and tempo marks are not copied, and paste
  never adds bars.
- Selecting **whole bars** (keeping bar structure) is deferred on purpose: it changes
  master bars, which every track shares, and needs its own rules.
- The clipboard is **in-app only**: nothing goes to or comes from the system clipboard,
  so you cannot paste TAB into or from other programs. It is shared by all documents
  while the app runs and is empty after a restart.
- Paste keeps **strings and frets, not pitches**. Pasting into a track with a different
  tuning or capo keeps the same fingering, so it sounds different. A note on a string or
  fret the target track does not have makes the whole paste fail, and nothing changes.

## Chord finder

- Symbol box plus the top voicings with the default constraints. Advanced constraints
  (max span, required bass, open/barre preference) are not exposed in the UI yet (T6).
  "Insert at cursor" places the voicing as one beat using the current beat duration.

## Interchange and storage

- MIDI import and export only. No MusicXML or Guitar Pro. No SQLite library index.
- Resolved in Phase 6b-1 (ADR-030): the session marker, the "save changes?" prompt on
  window close, New, Open and MIDI import, "Open original", and offering only autosaves
  newer than their source. Kept here as a record; it is no longer a limitation.

## Not in the MVP at all

Standard notation, PDF, MusicXML, Guitar Pro, mixer, audio tracks, recording, vocal
analysis, any AI or ML feature, plugins, SDK, mobile and Web.
