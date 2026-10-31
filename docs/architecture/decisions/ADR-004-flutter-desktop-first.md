# ADR-004 Flutter presentation, desktop first on Windows

Status: Accepted
Date: 2026-10-31
Related documents: [cross-platform-ui.md](../cross-platform-ui.md), [alternative-architectures.md](../../research/alternative-architectures.md), [performance-requirements.md](../performance-requirements.md)

## Context
Targets are desktop, mobile and possibly Web. The user chose desktop first, Windows as initial validation platform, mobile second, Web as nice-to-have.

## Decision
Flutter for all UI. Develop and validate on Windows. Keep macOS and Linux buildable (weekly CI) but do not polish until post-MVP. Mobile layouts are designed but not built before Phase 7.

## Alternatives
Tauri or web UI; Rust-native UI (egui, iced, Slint); Kotlin Multiplatform; native per platform.

## Trade-offs
One UI codebase and strong custom painting. Desktop-specific features (docking, multi-window) are less mature in Flutter [Assumption].

## Consequences
TAB canvas is custom painted. Platform specifics go into the `platform` Dart package.

## Related fitness functions
FF-05, FF-08, FF-29.

## Reconsideration conditions
S4 shows unacceptable canvas performance; Flutter desktop support regresses; Web becomes the primary target.
