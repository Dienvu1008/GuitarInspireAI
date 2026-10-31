# Spikes (throwaway)

Purpose: Phase 0 experiments S1, S2, S4, S5 defined in [technical-spikes.md](../docs/research/technical-spikes.md).

Rules:

- Spike code is throwaway. It is never imported by production crates and is excluded from the root Cargo workspace (`exclude = ["spikes"]`). Each spike has its own workspace and lockfile.
- Spike code does not follow production architecture rules; it exists only to produce evidence.
- Evidence (raw measurement output) is stored in each spike's `results/` folder. Conclusions are in [m0-exit-report.md](../docs/roadmap/m0-exit-report.md).
- Phase 1 must not copy spike code into production without review; reimplement against the architecture.

| Folder | Spike |
|---|---|
| `s2-audio/` | S2 real-time playback: cpal (WASAPI), rustysynth, rtrb |
| `s5-format/` | S5 project format: ZIP + JSON, atomic replace, unknown-field detection |
| `s1_s4_flutter/` | S1 frb interop and S4 TAB rendering (one throwaway Flutter app, two scenarios; `fake_app/` stands in for `mc-app` without any frb dependency) |

Reproducing (Windows):

- S2: `cargo run --release -- offline`, `-- bufsize`, `-- device <seconds> <load>`; add `--features realtime` for the priority experiment.
- S5: `cargo run --release`.
- S1 and S4: `flutter_rust_bridge_codegen generate`, `flutter build windows --profile`, then `build\windows\x64\runner\Profile\s1_s4_flutter.exe <s1|s4> <out.json> [stream_seconds]`.
