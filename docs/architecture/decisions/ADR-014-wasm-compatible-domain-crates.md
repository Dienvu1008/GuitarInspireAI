# ADR-014 WASM-compatible domain crates

Status: Accepted
Date: 2026-10-31
Related documents: [dependency-rules.md](../dependency-rules.md), [cross-platform-ui.md](../cross-platform-ui.md), [technical-spikes.md](../../research/technical-spikes.md)

## Context
Web is a future target. Decisions that would block WASM later: threads in domain code, file system access in domain crates, native C dependencies in the core, blocking audio APIs, and synchronous file I/O assumptions in the app layer.

## Decision
Domain crates (`mc-theory`, `mc-score`, `mc-guitar`, `mc-pattern`, `mc-notation`, `mc-playback`, `mc-dsp`, `mc-project` on byte streams) must compile for `wasm32-unknown-unknown` and use no threads, clocks, file system or native libraries. Threading and I/O live in `mc-app` adapters and `mc-audio`.

## Alternatives
Ignore WASM until needed.

## Trade-offs
Small constraint now (no `std::fs` or threads in domain); avoids a large refactor later. `mc-app` and `mc-audio` are not yet WASM-checked.

## Consequences
Known remaining Web blockers: frb Web requirements [To verify, S6], cpal AudioWorklet needs nightly and cross-origin isolation, single-threaded Web unless SharedArrayBuffer is available, SoundFont size download.

## Related fitness functions
FF-03, FF-04.

## Reconsideration conditions
Web dropped as a goal (remove FF-04 by ADR), or Web promoted to MVP (extend checks to `mc-app`).
