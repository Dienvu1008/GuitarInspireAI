# Security and Privacy

Purpose: Identify the threat surface of a local-first music application and the controls for untrusted files, microphone data, dependencies and future network features.
Status: Accepted for MVP controls. Remote features: Deferred.
Related documents: [project-format.md](project-format.md), [data-architecture.md](data-architecture.md), [ai-music-architecture.md](ai-music-architecture.md), [ADR-010](decisions/ADR-010-local-first.md), [ADR-015](decisions/ADR-015-dependency-license-policy.md).

## 1. Threat surface (MVP)

| Surface | Threat | Control |
|---|---|---|
| Native project files | Path traversal, decompression bombs, malformed JSON, huge allocations | ZIP rules and limits ([project-format.md](project-format.md) section 11), in-memory reads, bounded readers, validating builders, fuzzing (FF-15) |
| MIDI import | Malformed chunks, huge track counts | Parser limits, validating builder, fuzzing |
| SoundFont files | Malformed RIFF data in a third-party parser | Size limit, load on control thread, catch errors; fuzz rustysynth loader in Phase 4 [Proposed] |
| Microphone | Privacy | Capture only while tuner is visible; never stored or transmitted in MVP; clear indicator in UI |
| Dependencies | Supply chain, vulnerable crates | cargo-deny advisories and sources (FF-10), pinned lockfile, minimal features (for example zip deflate only) |
| FFI boundary | Memory safety | Generated bindings only; no hand-written `unsafe` outside reviewed modules; panics converted to errors at the API boundary |
| Autosave files | Data exposure on shared machines | Stored in per-user app data |

## 2. Privacy principles

- Local-first: no network access in MVP. No telemetry in MVP.
- Any future telemetry, crash reporting, cloud sync or remote AI requires explicit opt-in, a description of the data sent, and data minimization (for example, send only the selected audio range).
- AI provenance records never include personal data beyond asset hashes.

## 3. Future network features (Deferred)

- Remote inference and cloud sync: TLS, authenticated APIs, per-feature consent, ability to delete remote data.
- Plugins: untrusted code; requires process isolation or a sandboxed runtime before any third-party plugin is allowed ([ADR-017](decisions/ADR-017-plugin-architecture-deferred.md)).
