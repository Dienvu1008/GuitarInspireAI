# ADR-027 Unsafe exception for the Windows MMCSS call in mc-audio

Status: Accepted (user, 2026-10-31, option A at the Phase 4 audio review)
Date: 2026-10-31
Extends: [ADR-022](ADR-022-unsafe-confined-to-generated-ffi.md)
Related documents: [ADR-007](ADR-007-audio-thread-isolation.md), [ADR-024](ADR-024-playback-rendering-conventions-and-rt-failures.md), [security-and-privacy.md](../security-and-privacy.md), [dependency-rules.md](../dependency-rules.md), [audio-dsp-architecture.md](../audio-dsp-architecture.md)

## Context
ADR-007 amendment 1 requires the audio callback thread to be promoted to real-time scheduling on Windows, with an own call to `AvSetMmThreadCharacteristicsW("Audio")` through `windows-sys` instead of cpal's `realtime` feature (its `audio_thread_priority` dependency is MPL-2.0, banned by ADR-015). Spike S2 measured the effect: without promotion, 16 callbacks arrived late within 60 s (up to 37 ms, more than the 33 to 53 ms output buffer); with it, 600 s had none.

`AvSetMmThreadCharacteristicsW` is an `unsafe fn` (raw pointers). ADR-022 confines `unsafe` to the generated bridge module, and FF-34 enforces that. ADR-022 itself says a crate needing `unsafe`, "for example ... platform code in `mc-audio`", needs a new ADR. Slice 4b therefore shipped promotion as a stub reporting `Promotion::Unavailable` until this decision.

## Decision
1. `mc-audio` gets one exception, mirroring the `mc-ffi` mechanism of ADR-022. `mc-audio` restates the workspace lints in its own `[lints]` table with exactly one difference, `unsafe_code = "deny"`; no other key may be added.
2. The only permitted `#[allow(unsafe_code)]` in `mc-audio` is the attribute directly on `mod mmcss;` in `crates/mc-audio/src/lib.rs` (the module is compiled on Windows only). The module is `crates/mc-audio/src/mmcss.rs`.
3. `mmcss.rs` contains exactly one `unsafe` block: the call to `AvSetMmThreadCharacteristicsW`, preceded by a `// SAFETY:` comment. It exposes one safe function that returns whether the promotion succeeded. Nothing else in the crate, including the real-time core, contains `unsafe`.
4. Safety argument for that call:
   - the task name is a `static` NUL-terminated UTF-16 array (`"Audio"`), valid for the whole program, so the pointer is valid and nothing is allocated (GR-10);
   - the task index is written to a local `u32` that outlives the call;
   - the returned handle is only tested for null and never dereferenced; it is not released, because the callback thread lives as long as the stream and the registration ends with the thread;
   - the call is made once, on the first callback, by the callback thread itself, which is the thread to promote;
   - failure (null handle) is not an error for the engine: it reports `Promotion::Unavailable` and runs at normal priority.
5. FF-34 (`cargo xtask fitness unsafe-scope`) is extended from one exception to a fixed list of two: `mc-ffi` with `mod frb_generated;` and `mc-audio` with `mod mmcss;`. For `mc-audio` it also checks that `mmcss.rs` has exactly one `unsafe` token outside comments and that a `// SAFETY:` comment precedes it. Any other `allow(unsafe_code)`, any other crate-local lint table and any other exception still fail.
6. `windows-sys =0.61.2` (MIT OR Apache-2.0, features `Win32_Foundation` and `Win32_System_Threading` only) is a Windows-only dependency of `mc-audio`, already declared in `architecture/allowed-deps.toml`.

## Alternatives
- A safe wrapper crate for MMCSS (B): none known with an allow-listed license; a survey would still leave an unreviewed `unsafe` inside a dependency. It can replace this module later without changing the engine.
- Running without promotion (C): measured as unacceptable in S2 (late callbacks up to 37 ms).
- cpal `realtime` feature: MPL-2.0 `audio_thread_priority`, banned (ADR-015).
- Putting the call in `mc-ffi` or a new crate for platform code: `mc-ffi` must not depend on audio internals, and a crate for one call adds a crate for nothing; the call belongs next to the callback it promotes.

## Trade-offs
The exception adds a second lint table to keep in step with the workspace by hand; FF-34 detects drift as it does for `mc-ffi`. The single `unsafe` is hand-written and reviewed here, unlike the generated module.

## Consequences
- `mc-audio/Cargo.toml` carries its own `[lints]` table; `windows-sys` is added under `[target.'cfg(windows)'.dependencies]`.
- `platform::promote_callback_thread` returns `Promoted` or `Unavailable` on Windows and `NotNeeded` elsewhere; `EngineStatus::promotion` reports it.
- security-and-privacy.md, dependency-rules.md section 3a and the architecture guardrails (steering) name the second exception.
- Any further `unsafe` anywhere still needs a new ADR.

## Related fitness functions
FF-34 (unsafe scope), FF-16 and FF-17 (the callback body stays free of allocation and forbidden constructs; the promotion call is made before the callback core runs), FF-18 (late callbacks are measured with promotion on).

## Reconsideration conditions
A maintained safe MMCSS crate with an allow-listed license appears; a second platform needs its own promotion (it would get its own module and its own ADR); `windows-sys` changes the signature.
