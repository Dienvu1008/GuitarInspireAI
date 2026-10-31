# ADR-018 Notation layout in Rust, painting in Flutter

Status: Accepted (2026-10-31, after S4)
Date: 2026-10-31
Related documents: [cross-platform-ui.md](../cross-platform-ui.md), [bounded-contexts.md](../bounded-contexts.md), [technical-spikes.md](../../research/technical-spikes.md)

## Context
TAB layout logic (spacing, beaming, system breaking) is music logic that a CLI, PDF export and future SDK also need. Painting needs Flutter.

## Decision
`mc-notation` computes a display list in abstract units, using font metrics supplied by Dart (`LayoutMetrics`). Flutter paints the display list with `CustomPainter` and uses hit regions for interaction.

## Alternatives
Layout in Dart; Rust renders pixels into a texture; third-party engraving engine (Verovio, LGPL-3.0 [To verify]).

## Trade-offs
Reusable, testable layout (golden display lists), at the cost of a metrics contract between Dart and Rust and projection payload sizes. S4 measured the cost as negligible for TAB: 5 µs layout per 4-bar system in Rust, about 50 µs including FFI, 1.5 KB payload per system as typed arrays, and frame p99 under 12 ms while scrolling 500 bars. Synchronous layout calls from `build` are acceptable at this cost. Standard notation (Phase 6c) has much denser output and must be re-measured.

## Consequences
Golden tests of layout output in Rust. PDF export can reuse the display list.

## Related fitness functions
Interop benchmarks (payload size), FF-21.

## Reconsideration conditions
S4 shows payload or latency problems; standard notation engraving requires a third-party engine (S11).
