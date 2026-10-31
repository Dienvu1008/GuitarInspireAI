# ADR-005 Dart and Rust interop via flutter_rust_bridge, no Protobuf

Status: Proposed (Accepted when S1 passes)
Date: 2026-10-31
Related documents: [interop-architecture.md](../interop-architecture.md), [technology-evaluation.md](../../research/technology-evaluation.md), [technical-spikes.md](../../research/technical-spikes.md)

## Context
Dart must call Rust commands and queries and receive events and high-rate streams. Both sides ship in one binary.

## Decision
flutter_rust_bridge v2 with typed commands and queries, opaque handles, and typed `StreamSink` streams. frb types appear only in `mc-ffi`. No Protobuf and no JSON over FFI.

## Alternatives
Direct typed calls only (polling); Protobuf messages over FFI; Protobuf or gRPC IPC; hand-written C ABI.

## Trade-offs
Low overhead, one schema (Rust types), good debugging. Depends on a project with a small maintainer base, and generated code must be kept in sync.

## Consequences
`mc-ffi` is the composition root. Fallback is a hand-written C ABI in `mc-ffi` with the same application API.

## Related fitness functions
FF-01, FF-02, FF-12, interop benchmarks.

## Reconsideration conditions
S1 fails; frb becomes unmaintained; a sidecar process, plugin ABI, remote API, or non-Dart SDK binding is required (then evaluate Protobuf or a C ABI).
