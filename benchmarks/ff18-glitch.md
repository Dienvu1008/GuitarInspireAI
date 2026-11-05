# FF-18 playback glitch result

Purpose: Record the 10-minute playback glitch run (GR-10, A4; performance-requirements
section 3, "zero underruns in 10 min reference playback"). FF-18 is Important and manual:
hosted CI has no audio device, so this is run on the reference machine (ADR-025). Evidence
is valid for this machine only.

Procedure (reproducible):

```
$env:FF18_MINUTES=10
cargo test -p mc-audio --test glitch -- --ignored --nocapture
```

The test plays the reference performance (4 tracks, 64 bars, dense) on the default output
device, looping for the whole run, and counts late callbacks. cpal reports no xruns on
WASAPI shared render, so a late callback is one arriving more than 1.5x the device period
(10 ms at 48 kHz) after the previous one. The criterion is zero.

## Result (2026-10-31)

| Item | Value |
|---|---|
| Duration | 10 minutes |
| Machine | reference machine (see `reference-machine.md`): i5-12400F, 15.8 GiB, Windows 11 Pro 26H2 (10.0.26300, UBR 9818) |
| Output device | default; WASAPI shared, 48 000 Hz, period 480 frames (10 ms) |
| Thread promotion | `Promoted` (Windows MMCSS "Audio", ADR-027) |
| SoundFont | `GuitarSteel.sf2`, SHA-256 `8C9B74EF954235164D8C5EA58FE10C8687F808D92FBB23BA388F5CE5FAB49EF8` (trimmed FluidR3, ADR-026) |
| Callbacks | 60 013 |
| **Late callbacks** | **0** |
| Device errors | 0 |
| Device rebuilds | 0 |
| Max callback interval | 13.4 ms (threshold 15 ms = 1.5x period) |
| Max callback duration | 9.69 ms |
| Reported output latency | 32 854 us (~32.9 ms) |

Zero underruns in 10 minutes: the A4 criterion holds on this machine. A second machine is
deferred and R-21 stays open (ADR-025); this result does not generalize beyond the
machine above.
