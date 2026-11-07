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

## ADR-032 X12 attempt (2026-11-07): not met, inconclusive

Same procedure, same machine, after the mixer change (per-channel volume and pan polled once per callback). **X12 is not met by this attempt.** The criterion is unchanged: 0 late callbacks in 10 minutes, and the callback maximum may not grow by more than 5 µs p99.

| Run | Code | Late callbacks | Max interval | Max callback | Device errors / rebuilds |
|---|---|---|---|---|---|
| 10 min | with mixer (`bc8cda4` + 6b-3c working tree) | **8** | 18.5 ms | 18.42 ms | 0 / 0 |

Because the code from before the change might also fail on the machine in its present state, an interleaved A/B followed: 2-minute runs, alternating the pre-mixer build (`12f869e`, worktree) and the current build, three rounds each:

| Round | Base (`12f869e`) late | Current late |
|---|---|---|
| 1 | 0 (max callback 10.18 ms at 90 s) | 3 |
| 2 | 1 | 0 (max callback 13.29 ms at 90 s) |
| 3 | 0 (max callback 11.27 ms at 90 s) | 5, with 2 device errors and 1 device rebuild |
| Total in 6 min | **1** | **8** |

What this shows and what it does not:
- The base build also produced a late callback, so the machine was not in the state of the 2026-10-31 run (0 late, max callback 9.69 ms). The Kiro IDE was using a large share of CPU during the runs (total CPU 17 to 43 % in a spot check), and round 3 of the current build saw device errors.
- The current build had more late callbacks than the base. With these few runs, that cannot be put down to load alone. **A regression is not ruled out.**
- The procedure runs a debug build. In debug, the callback maximum was already 9.69 ms against a 10 ms period on 2026-10-31, so there is very little headroom.
- An offline release-mode measurement of the mix polling itself (`mix_overhead_measurement`, 20 000 blocks of 480 frames, product synthesizer, the two engines timed alternately) found no measurable cost. p99 was 233.8 µs with the polling and 234.7 µs without. With a volume change on every block, the figures were 306.2 µs and 307.4 µs. The differences (−0.9 µs and −1.2 µs) are within noise. This measures the code path, not the device run, so it does not replace X12.

Next step: repeat the 10-minute run and the interleaved A/B with the machine quiet (after a restart, no second IDE), and record the result here. Until then, X12 is an open evidence gap for 6b-3.
