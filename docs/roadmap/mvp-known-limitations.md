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
- **Mixer (Phase 6b-3, ADR-032).** The Mixer panel (Tools menu, open by default) has
  Mute, Solo, a volume slider and a pan slider for each track. Before Phase 6b-3 the MVP
  had per-track mute in the playback core and controller only: **no button for it was
  placed on screen**, so the MVP release notes and the Phase 4 exit report overstated it
  (corrected there). Limits of the mixer:
  - Mute, solo, volume and pan are **session settings: they are not saved** with the
    project and are lost on reopen. Saving them is a possible later slice with its own
    format decision.
  - A muted track stays silent even when soloed.
  - **MIDI export does not carry the mix** (no volume or pan events), so an exported file
    does not sound like the app.
  - Tracks 16 and above share a MIDI channel with tracks 1 and above (channel 10 is
    skipped), so they share one volume and pan. The last one set wins.
  - The volume range tops out at 127 and the default is 100, so a track can be raised
    by 27 points at most.
  - **Glitch-free playback is not re-confirmed after this change.** The 10-minute glitch
    run (FF-18) now shows a few late callbacks on the reference machine, and so does the
    build from before the mixer. The zero-underrun result of Phase 4 no longer
    reproduces, and the cause is not yet found (ADR-032 section 6,
    `benchmarks/ff18-glitch.md`).
- One bundled SoundFont (trimmed FluidR3, program 25). User-supplied SoundFonts are a
  future capability. If the SoundFont is missing, playback is disabled with a message (no
  sine fallback).

## Editor and notation

- Several TAB tracks can be shown at once (Phase 6b-2, ADR-031): one stave per shown
  track in each line of music, with bar lines and simultaneous notes lined up. Editing,
  the cursor, the selection and the inspector act on the **active track** only; clicking
  another stave makes it active. Which tracks are shown is not saved: it resets when a
  document is opened.
- Every track is shown when a document opens; hide or show a track with the eye button
  in the track list. The active track cannot be hidden.
- **Editing can occasionally feel slow, on large scores and also on small ones.** On the
  reference machine, a note edit on the large test score (20 tracks of 300 bars) took
  about 15 to 19 ms (budget 33 ms) in 18 of 23 runs, with 4, 16 or 20 tracks shown.
  In the other 5 runs it took 40 to 48 ms, at every track count including 4. The same
  intermittent slow state was seen with code from before this feature, and its cause is
  not yet found (ADR-031). Typical scores (4 tracks, 150 bars) were within budget in
  every run of the last session. One machine only (ADR-025).
- No standard notation, no PDF.
- Effects are tie, let ring, dead note and, from Phase 6a-1 (ADR-035), accent, palm mute,
  string-percussive and a down or up stroke per beat. They are set in the inspector; there
  are no keyboard shortcuts for them yet. No lyrics.
- **Playing techniques (Phase 6a-1, ADR-035).** Limits:
  - The sound uses fixed values (for example accent +24 velocity, palm mute 80); there is no
    setting. When several techniques are on one note, one wins (string-percussive, dead, palm
    mute, let ring); all stay stored and shown.
  - A stroke changes no timing: the notes of a chord still start together (no strum
    spread), and MIDI export carries no stroke.
  - The TAB marks above the staff (arrow, `>`, `S`, "P.M.") can overlap volta brackets and
    the stems of an upper voice.
  - Files that use them are format 1.1. An older build opens them but cannot save over
    them (use Save As); it loses the techniques.
- Repeat editing is limited to simple start and end markers.
- **Patterns (Phases 6a-3 to 6a-5, ADR-034).** Tools > Pattern editor... makes and edits
  patterns on a grid, with user gestures; Tools > Apply pattern... applies one to a range
  of whole bars of the active track, for one chord voicing. Limits:
  - The pattern library belongs to the project (format 1.2). There is no library shared
    between projects. A file with a changed library cannot be saved over by an older build
    (it offers Save As).
  - The grid places a step one cell long; a longer step is made by resizing it (] and [).
    A step gets exactly one written value, so some lengths (for example 5/16) cannot be
    made; the editor says so.
  - Deleting a strike that a hold continues is refused; delete the hold first.
  - A beat is one unit of the meter (an eighth in 6/8).
  - The preview is a text summary (repetitions, notes, bars replaced, steps cut at the
    end), not a TAB preview.
  - One voicing for the whole range; there is no chord track.
  - A range with a pickup bar is refused. When the range is not a whole number of
    repetitions, you choose how to fit it each time.
  - The pattern replaces voices 1 and 2 of the bars it writes; it does not insert.
  - Holding a dead, palm-muted or string-percussive note, letting one cross a bar line, or
    cutting one at the end of the range to a length that needs more than one note value, is
    refused, because playback would sound it twice.
  - If the score changes between the preview and Apply, Apply is refused and you preview
    again.
  - The pattern text notation (Phase 6a-4, [pattern-text-syntax.md](../architecture/pattern-text-syntax.md))
    exists only inside the core and its tests. There is no way to type or load a pattern
    as text in the app yet.
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

- MIDI import and export. No Guitar Pro. No SQLite library index.
- **MusicXML export** (Phase 6b-4, [ADR-033](../architecture/decisions/ADR-033-musicxml-export-adapter.md)): File > Export MusicXML... writes MusicXML 4.0 as an uncompressed `.musicxml` file. Its limits:
  - **Export only**: MusicXML files cannot be opened or imported.
  - **TAB staves only**, one per track, as the app shows them. No standard-notation staff; standard notation is Phase 6c.
  - Compressed `.mxl` is not offered.
  - Bars with too few or too many beats are written as they are, and the export message says so. A short voice is filled with a rest; an overfull bar keeps all its notes, and some readers may mark it as irregular. No note is ever cut.
  - The mixer settings (volume, pan, solo, mute) and the tempo factor are not exported. Every track is written.
  - **Checked with MuseScore 4.7.5 only, on one machine.** Finale, Dorico and Guitar Pro were not checked, and layout quality was not judged. Known MuseScore behaviour (`benchmarks/musescore-musicxml.md`):
    - **MuseScore ignores the capo.** A track with a capo is re-fingered in MuseScore (frets shifted, notes moved to other strings), although the file follows the MusicXML definition.
    - Dead notes show as fret numbers, not X.
    - Playing techniques (ADR-035): accents and strokes on notes are kept (one accent per
      chord). A stroke on a rest is shown, but lost when MuseScore saves MusicXML again.
      **Palm mute and string-percussive are lost** in MuseScore.
      - The file follows MusicXML 4.0: palm mute playback as `<play><mute>palm</mute></play>`, and a stroke on a rest as a SMuFL strum symbol.
      - The palm-mute and string-percussive marks are `<other-technical>` text, because 4.0 has no element for them; the export message counts them.
      - MuseScore also loses its own palm mute through MusicXML.
    - A tie into a repeated section is not played again on the second pass.
    - MuseScore's own MIDI playback does not sustain let ring, although the let-ring marks are read and kept.
- Resolved in Phase 6b-1 (ADR-030): the session marker, the "save changes?" prompt on
  window close, New, Open and MIDI import, "Open original", and offering only autosaves
  newer than their source. Kept here as a record; it is no longer a limitation.

## Not in the MVP at all

Standard notation, PDF, MusicXML (export added in Phase 6b-4), Guitar Pro, mixer (added in Phase 6b-3), audio tracks, recording, vocal
analysis, any AI or ML feature, plugins, SDK, mobile and Web.
