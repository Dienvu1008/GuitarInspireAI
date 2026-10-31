# ADR-005 Dart and Rust interop via flutter_rust_bridge, no Protobuf

Status: Accepted (2026-10-31, after S1), with the amendments below
Date: 2026-10-31
Related documents: [interop-architecture.md](../interop-architecture.md), [technology-evaluation.md](../../research/technology-evaluation.md), [technical-spikes.md](../../research/technical-spikes.md), [m0-exit-report.md](../../roadmap/m0-exit-report.md)

## Context
Dart must call Rust commands and queries and receive events and high-rate streams. Both sides ship in one binary.

## Decision
flutter_rust_bridge v2 (pinned 2.13.0) with typed commands and queries, opaque handles, and typed `StreamSink` streams. frb types appear only in `mc-ffi`. Application types in `mc-app` stay free of frb and are exposed with `#[frb(mirror)]` (verified in S1 for structs and payload enums). No Protobuf and no JSON over FFI.

Amendments from S1 evidence:

1. Stream lifecycle is owned by Rust. Each stream has an explicit unsubscribe API and is closed when its handle is dropped. Dart code must not rely on awaiting `StreamSubscription.cancel()` to release a Rust sink; in S1 it never completed while Rust held the sink.
2. Values crossing FFI that Dart treats as numbers (revision, seq, ids, ticks) use types that map to Dart `int`, not `u64`, which maps to `BigInt`. The exact type is chosen in Phase 3 from the frb type mapping table [To verify].
3. The Dart side depends on `freezed`, `freezed_annotation` and `build_runner`, because payload enums map to `freezed` sealed classes. These are Dart dev-time and runtime dependencies to include in the FF-09 Dart audit.
4. CI preinstalls pinned `flutter_rust_bridge_codegen` and `cargo-expand`; frb otherwise installs `cargo-expand` unpinned.
5. The playback cursor in Dart extrapolates position from tempo between stream updates, so an isolated late update (S1: one 56 ms gap in 10 minutes) does not stall the cursor.
6. Phase 3 (2026-10-31): the generated Rust module contains `unsafe` blocks, which the workspace-wide `unsafe_code = "forbid"` rejects. Resolved by [ADR-022](ADR-022-unsafe-confined-to-generated-ffi.md): `mc-ffi` alone uses `deny`, with one `allow` on the generated module, checked by FF-34.

## Alternatives
Direct typed calls only (polling); Protobuf messages over FFI; Protobuf or gRPC IPC; hand-written C ABI.

## Trade-offs
Low overhead (S1: no-op round trip p99 160 µs, sync query p99 3 µs), one schema (Rust types), good debugging. Depends on a project with a small maintainer base, generated code must be kept in sync, and payload enums add the `freezed` toolchain on the Dart side.

## Consequences
`mc-ffi` is the composition root. Fallback is a hand-written C ABI in `mc-ffi` with the same application API.

## Related fitness functions
FF-01, FF-02, FF-12, interop benchmarks.

## Reconsideration conditions
frb becomes unmaintained; frb or `freezed` codegen becomes a recurring build blocker; a sidecar process, plugin ABI, remote API, or non-Dart SDK binding is required (then evaluate Protobuf or a C ABI).
