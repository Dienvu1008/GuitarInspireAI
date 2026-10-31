# ADR-007 Audio thread isolation

Status: Accepted (2026-10-31, after S2) with the amendments below and with the residual OS scheduling risk documented. A8 to A10 (tuner) are still open until S3 in Phase 5.
Date: 2026-10-31
Related documents: [audio-dsp-architecture.md](../audio-dsp-architecture.md), [performance-requirements.md](../performance-requirements.md), [technical-spikes.md](../../research/technical-spikes.md), [m0-exit-report.md](../../roadmap/m0-exit-report.md)

## Context
Audio callbacks must meet hard deadlines. Libraries do not guarantee real-time safety on their own.

## Decision
Separate control plane (non-RT control thread, SPSC rings) from data plane (RT callbacks). RT code lives in `mc-audio/src/rt/`, has no allocation, locks, I/O, logging or FFI, and swaps preallocated timelines by pointer with off-thread deallocation. Tuner analysis runs on its own thread. S2 validated this design with cpal 0.18.2, rustysynth 1.3.8 and rtrb 0.4.1: zero allocations in the callback, callback p99 at most 3.2 percent of the 10 ms period, 99 timeline swaps without glitches or leaks.

Amendments from S2 evidence:

1. The audio callback thread must be promoted to real-time scheduling. Without it, late callbacks up to 37 ms (more than the output buffer) occurred within 60 s. Two implementations were measured as equivalent: cpal's `realtime` feature (adds `audio_thread_priority`, MPL-2.0) and an own call through `windows-sys` (MIT OR Apache-2.0). Decision (user, 2026-10-31): use the own call. It is a small Windows-only platform module inside the `mc-audio` adapter that calls `AvSetMmThreadCharacteristicsW("Audio")` once on the first callback with a preallocated task name. It is not part of the Music Core domain. No cross-platform priority abstraction is introduced now; other platforms get their own small promotion when they are actually supported. cpal's `realtime` feature stays disabled, which the ADR-015 license policy enforces in CI.
2. Underrun detection cannot rely on cpal error callbacks on WASAPI shared render (zero xruns were reported even with 44 ms gaps). Callback timing (interval against reported output latency, and callback duration against the period) is a permanent part of audio diagnostics and testing: FF-18, and the engine's runtime diagnostics counters.
3. WASAPI shared mode fixes the period at 480 frames at 48 kHz on the reference machine. `BufferSize::Fixed` is not honored. Lower latency would need exclusive mode or ASIO, which are not in MVP scope.
4. After loading a SoundFont, render a short silent warm-up before starting playback.

## Alternatives
Mutex-protected shared state; rendering on the UI or command thread into a large buffer; using rodio's higher-level abstractions.

## Trade-offs
Glitch-free design at the cost of more structure (rings, swap protocol) and testing effort. Residual risk: rare OS-level stalls (up to 44 ms observed once in five 120 s runs with promotion) are outside the engine's control.

## Consequences
Assumptions A1 to A7 are validated on the reference machine; A8 to A10 remain for S3. RT code reviewed with a checklist.

## Related fitness functions
FF-16, FF-17, FF-18.

## Reconsideration conditions
Stalls with promotion become audible in practice on typical hardware (then evaluate larger buffering, exclusive mode, or a render-ahead design); latency requirements for live monitoring or effects processing arrive (may need exclusive mode or ASIO).
