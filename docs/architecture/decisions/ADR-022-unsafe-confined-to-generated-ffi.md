# ADR-022 Unsafe code confined to the generated FFI module

Status: Accepted (user, 2026-10-31). Extended by [ADR-027](ADR-027-unsafe-exception-mmcss-module.md) with one more exception: the Windows MMCSS call in `mc-audio`.
Date: 2026-10-31
Related documents: [ADR-005](ADR-005-dart-rust-interop-frb.md), [security-and-privacy.md](../security-and-privacy.md), [dependency-rules.md](../dependency-rules.md), [fitness-functions.md](../fitness-functions.md), [interop-architecture.md](../interop-architecture.md)

## Context
The workspace sets `unsafe_code = "forbid"` for every crate (user decision 2026-10-31, confirmed again for FF-23). ADR-005 selects flutter_rust_bridge, whose generated Rust module (`frb_generated.rs`) contains `unsafe` blocks for the C ABI: 16 in the S1 spike. `forbid` cannot be overridden by an inner `allow`, so `mc-ffi` cannot compile under the workspace lints. The security baseline already expects "generated bindings only; no hand-written `unsafe` outside reviewed modules", but no mechanism enforced it.

Verified on 2026-10-31: `mc-ffi` with `edition = "2024"`, `unsafe_code = "deny"` and `#[allow(unsafe_code)]` on `mod frb_generated;` builds and passes `cargo clippy -D warnings` with codegen 2.13.0.

## Decision
- Every crate except `mc-ffi` keeps `[lints] workspace = true`, so `unsafe_code = "forbid"` stays in force everywhere else.
- `mc-ffi` restates the workspace lints in its own `[lints]` table with exactly one difference: `unsafe_code = "deny"`. It may add `unexpected_cfgs` for `cfg(frb_expand)`, which the generator needs.
- The only permitted `allow(unsafe_code)` in the repository is the attribute directly on `mod frb_generated;` in `crates/mc-ffi/src/lib.rs`. Hand-written code in `mc-ffi` (the `api` modules) contains no `unsafe`; the `deny` level rejects it.
- `frb_generated.rs` (Rust) and `lib/src/generated/` (Dart) are never edited by hand. FF-12 regenerates them with the pinned codegen and fails on any difference.
- New fitness function FF-34 (Critical, xtask `unsafe-scope`) checks all of the above on every push.

## Alternatives
- Keep `mc-ffi` outside the workspace, as the S1 spike did: rejected, because it would escape FF-01, FF-02, clippy and the other workspace checks.
- Keep `forbid` everywhere and drop flutter_rust_bridge: rejected, because it contradicts the accepted ADR-005 and would need hand-written FFI, which means more `unsafe`, not less.
- Workspace-wide `deny` with allow-lists: rejected, because it weakens every crate to protect one generated file.

## Trade-offs
The `mc-ffi` lint table must be kept in step with the workspace table by hand; FF-34 detects drift. The generated module is trusted as the output of a pinned, widely used generator, not reviewed line by line.

## Consequences
- `mc-ffi/Cargo.toml` carries its own `[lints]`; FF-34 compares it with `[workspace.lints]`.
- The generated module also carries `#[rustfmt::skip]`: the generator's output is not formatted in the workspace style, and reformatting it would make FF-12 (regenerate and diff) fail. Hand-written code stays under `cargo fmt --check`.
- `xtask` now uses `[lints] workspace = true` as well (it had no lint table before), so the forbid applies to the governance tool too.
- A future crate that needs `unsafe` (for example a counting allocator for FF-23, or platform code in `mc-audio`) needs a new ADR; ADR-022 does not extend to it.
- Panics at the API boundary are converted to errors by the generated code (frb default); hand-written API functions return `Result` for expected failures.

## Related fitness functions
FF-34 (unsafe confined to generated FFI module), FF-12 (generated bindings in sync), FF-02 (flutter_rust_bridge only in `mc-ffi`).

## Reconsideration conditions
flutter_rust_bridge changes how it generates code (for example, generated code without `unsafe`, or code spread over several modules), or another crate needs `unsafe`.
