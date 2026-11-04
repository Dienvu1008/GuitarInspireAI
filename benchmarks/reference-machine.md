# Reference Machine

Purpose: Record the machine used for M0 spike measurements and later benchmark baselines (performance-requirements section 1), so later runs on other machines can be compared.
Status: Recorded 2026-10-31; Phase 4 entry details added 2026-10-31.

This is the only measurement machine. All performance, latency and glitch evidence (S1 to S5, FF-18, A1 to A7, benchmarks, edit feedback) is valid for this machine only. A second machine is deferred and R-21 stays open ([ADR-025](../docs/architecture/decisions/ADR-025-single-machine-evidence-accepted.md)).

| Item | Value |
|---|---|
| CPU | Intel Core i5-12400F (12th Gen), 6 cores, 12 threads, base clock reported 2.5 GHz |
| RAM | 15.8 GiB |
| GPU | NVIDIA GeForce RTX 5060, driver 32.0.16.1088 |
| OS | Windows 11 Pro 26H2, 10.0.26300, UBR 9818 |
| Power plan | Balanced |
| Audio devices | USB Audio 2.0 interface; NVIDIA High Definition Audio (HDMI); onboard High Definition Audio Device |
| Audio drivers | USB Audio 2.0: Microsoft class driver 10.0.26100.9818 (2026-10-25); High Definition Audio Device: Microsoft 10.0.26100.9818 (2026-10-25); NVIDIA High Definition Audio 1.4.5.7 (2026-07-22) |
| Audio mode | WASAPI shared; period 480 frames at 48 kHz (S2); default output device and its rate are logged by each run |
| Real-time antivirus | Microsoft Defender real-time protection enabled |
| OneDrive | Not running; workspace not in a synced folder |
| Rust | 1.98.0 stable, x86_64-pc-windows-msvc |
| Flutter | 3.47.2 stable, Dart 3.13.2 |
| C++ toolchain | Visual Studio Community 2026 18.5.2 |

## SoundFont used for playback measurements

| Item | Value |
|---|---|
| File | `FluidR3_GM.sf2`, Fluid (R3) release 3, Frank Wen, MIT (original `COPYING`) |
| Source | Debian source package `fluid-soundfont_3.1.orig.tar.gz` from `deb.debian.org/debian/pool/main/f/fluid-soundfont/` |
| Archive | 134 835 922 bytes, SHA-256 `2621ACAA1C78E4ABDB24BDD163230CC577E61276936D6AA6E3180582142F0343` |
| SoundFont | 148 398 306 bytes, SHA-256 `74594E8F4250680ADF590507A306655A299935343583256F3B722C48A1BC1CB0` |
| Status | Candidate; not bundled yet (bundling decision pending, Phase 4 slice 4c) |

S2 used the GeneralUser GS bank and a sine test font; Phase 4 runs use the file above. Each FF-18 result file repeats the machine summary, the output device, its sample rate and the SoundFont SHA-256.
