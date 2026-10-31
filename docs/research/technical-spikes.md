# Technical Spikes

Purpose: Define the proof-of-concept experiments (S1 to S12) that answer questions documentation cannot, with question, timebox, success criteria and the phase gate each one unblocks.
Status: Proposed. These are roadmap activities. None were executed during the documentation assignment.
Related documents: [implementation-phases.md](../roadmap/implementation-phases.md), [dependency-map.md](../roadmap/dependency-map.md), [technology-evaluation.md](technology-evaluation.md), [audio-dsp-architecture.md](../architecture/audio-dsp-architecture.md), [risk-register.md](../roadmap/risk-register.md).

Rules:

- Spike code is throwaway, kept in a `spikes/` folder or a branch, never merged into production crates.
- Timeboxes are maxima. When a timebox expires, the spike stops and records what is known; the decision is then made with that information or the risk is escalated in the [risk register](../roadmap/risk-register.md).
- Each spike ends with a short result note (pass, fail, findings, decision impact) appended to this document or the related ADR.

| Id | Question | Timebox (FTE days, max) | Success criteria | Gate |
|---|---|---|---|---|
| S1 | Does frb v2 on Windows support our interop shape: opaque document handle, async execute, event `StreamSink`, 60 Hz position stream, mirroring of `mc-app` types without frb annotations inside `mc-app`, acceptable build and reload workflow? | 2 | All features work in a minimal Flutter Windows app; no-op round trip p99 under 2 ms; 60 Hz stream for 10 min without UI jank; mirroring works or a DTO fallback is documented | Phase 0 exit; ADR-005 |
| S2 | Are cpal (WASAPI), rustysynth and rtrb real-time safe and fast enough together? Assumptions A1 to A7 | 3 | Criteria A1 to A7 in [audio-dsp-architecture.md](../architecture/audio-dsp-architecture.md) | Phase 0 exit; ADR-007 |
| S3 | Can owned YIN or MPM give a usable tuner from the Windows microphone? Assumptions A8 to A10 | 2 | Criteria A8 to A10 | Phase 5 entry (budgeted inside Phase 5) |
| S4 | Can Rust layout + Flutter `CustomPainter` render and scroll a 500-bar TAB track smoothly? | 2 | No frame over 16 ms while scrolling on the reference machine; layout payload per system measured; metrics contract workable | Phase 0 exit; ADR-018 |
| S5 | ZIP + JSON format: size and load time for small, typical, large scores; Windows atomic replace semantics including OneDrive and antivirus locks; `zip` crate with deflate only; `flatten` performance | 2 | Typical open under 500 ms, large under 2 s; replace strategy chosen with failure behaviour documented; unknown-field preservation prototype works at nested levels | Phase 0 exit; ADR-008 |
| S6 | What blocks a Web build? frb Web smoke build with a domain crate | 1 (optional, outside Phase 0 estimate) | Blocker list documented | Informs ADR-014 |
| S7 | License audit of Phase 1 to 5 dependencies (transitive) and SoundFont selection | 1 | cargo-deny config passes on a dependency list; SoundFont with permissive license selected; NOTICE requirements listed | Phase 0 exit (before Phase 1 dependencies are added) |
| S8 | ONNX runtime choice: ort vs tract running Basic Pitch on Windows CPU | 3 | Model runs; latency and memory measured; packaging size known; licenses confirmed | Phase 9 entry |
| S9 | Source separation (Demucs class) on desktop CPU and GPU; weights license | 4 | Time per minute of audio, memory, quality on a test set; license decision | Phase 9 |
| S10 | Guitar Pro import: GP3 to GP5 and GP7 structure, legal and format risks, effort estimate | 3 | Parse fixture files of each version into a test model; risk note | Phase 6d entry |
| S11 | Standard notation engraving: own engine vs Verovio (LGPL-3.0 [To verify]) vs alternatives | 4 | Quality comparison on sample scores; licensing and linking plan; effort estimate | Phase 6c entry |
| S12 | Android and iOS: frb packaging, cpal AAudio and CoreAudio latency, AVAudioSession handling | 4 | Tuner and playback run on one Android and one iOS device; latency measured | Phase 7 entry |

## Phase 0 budget reconciliation

- Mandatory Phase 0 spikes: S1, S2, S4, S5, S7 = at most 10 FTE days (2 FTE weeks).
- Foundation work (repository, CI skeleton, cargo-deny, protected-path check, steering, reference machine record): 3 to 5 FTE days.
- Phase 0 total: 2 FTE weeks if spikes finish under their timeboxes and foundation takes 3 days or fewer within the same window, up to 3 FTE weeks at the maxima. This matches the 2 to 3 FTE week estimate in [implementation-phases.md](../roadmap/implementation-phases.md).
- S3 is budgeted in Phase 5; S6 is optional and outside the estimate.

Priority order if time is short: S1, S2, S4, S5 (highest architectural risk), then S7.
