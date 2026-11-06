# GuitarInspireAI MVP — Release Notes (M5)

Purpose: What the MVP build offers, for the milestone M5 release. The exit criteria
require release notes ([roadmap/README.md](README.md) section 1).
Status: M5 build on `phase-5-mvp`. Windows 10/11 x64.
Related documents: [mvp-known-limitations.md](mvp-known-limitations.md), [phase-5-exit-report.md](phase-5-exit-report.md).

## What you can do

A Windows desktop guitar TAB studio, useful without any AI:

- **Write TAB.** Multiple fretted tracks (add, remove, rename, reorder, switch); one
  active TAB track shown and edited at a time. Keyboard and mouse note entry; durations
  with dots and triplets; ties; let ring and dead notes; bars with time-signature
  changes; per-bar diagnostics; undo and redo.
- **Tune the instrument.** A microphone tuner that references the active track's tuning
  and capo (so it tunes drop-D when the track is drop-D), with a cents needle and the
  nearest string. Opens the microphone only while the tuner panel is shown.
- **Find chords.** Type a chord symbol and see the top voicings as fretboard diagrams for
  the track's tuning; insert a voicing into the TAB at the cursor with one click.
- **Play it back.** All tracks play together (independent of which is displayed), with
  play, stop, click-to-seek, a 0.25x–2.0x tempo factor and per-track mute, through a
  bundled SoundFont. A moving cursor follows the playback.
- **Interchange and save.** New, open, save, save as, autosave and crash recovery in the
  native `.giproj` format; import and export Standard MIDI Files with a report.

## How to run

Build from source (Windows): `flutter build windows --release` in `app/`, then run
`app\build\windows\x64\runner\Release\guitar_inspire.exe`. The trimmed SoundFont ships
next to the executable; the first Play loads it.

## About the tuner (please read)

The tuner works and is accurate on test signals, but it has **not yet been checked against
a real guitar through a real microphone**. If it reads oddly on your instrument —
especially on the low E and A strings, or in a noisy room — that is a **known limitation,
not a surprise bug**. No real-guitar measurements were claimed; the tuner is labelled as
unverified on real instruments until a recorded set exists. If you
have a guitar and want to help, the recording harness and steps are in
`spikes/s3-tuner/results/README.md`; a recorded set lets us finalise the tuner
([ADR-029](../architecture/decisions/ADR-029-tuner-pitch-detection.md) moves from Draft to
Accepted).

## Notes and limitations

See [mvp-known-limitations.md](mvp-known-limitations.md). In short: Windows x64 only;
single-machine performance evidence (ADR-025); the tuner's real-guitar accuracy is not
yet measured (synthetic only, ADR-029 Draft); one active TAB track at a time; default
audio input and output devices only; no standard notation, MusicXML, Guitar Pro, mixer or
AI in this build.

## Licensing

Bundled SoundFont: trimmed FluidR3 GM (MIT), provenance and license in
`assets/soundfonts/`. Third-party dependency licenses are governed by ADR-015 and checked
by cargo-deny. The project license itself (Q-10) is still open and is decided before any
public distribution.
