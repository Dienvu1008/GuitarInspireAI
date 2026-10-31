# ADR-017 Plugin architecture deferred

Status: Deferred
Date: 2026-10-31
Related documents: [security-and-privacy.md](../security-and-privacy.md), [architecture-overview.md](../architecture-overview.md)

## Context
Plugins (user scripts, audio plugins such as VST3 or CLAP, importer plugins) are a long-term idea. A plugin ABI is a long-lived public contract and a security boundary.

## Decision
No plugin system before the core APIs stabilize. Internal extension points are ordinary Rust traits (ports, `Synth`, importers) and Cargo features.

## Alternatives
WASM plugin runtime now; dynamic library plugins; scripting (Lua, Rhai).

## Trade-offs
No third-party extensibility early; avoids freezing immature APIs and an unvetted security surface.

## Consequences
Third parties can use the Music Core as a library later (Phase 11) rather than as plugins.

## Related fitness functions
FF-22 (public API stability before any plugin ABI).

## Reconsideration conditions
Concrete plugin use cases with users; stable application API for at least one release cycle; sandboxing approach chosen (likely a WASM runtime or process isolation).
