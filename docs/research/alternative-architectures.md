# Alternative Architectures

Purpose: Compare plausible stacks and system styles, and record why Rust core + Flutter UI as a modular monolith was selected.
Status: Accepted (selection). Reconsideration triggers listed below.
Related documents: [ADR-001](../architecture/decisions/ADR-001-modular-monolith.md), [ADR-002](../architecture/decisions/ADR-002-rust-core-source-of-truth.md), [ADR-004](../architecture/decisions/ADR-004-flutter-desktop-first.md), [technology-evaluation.md](technology-evaluation.md).

## 1. Stack alternatives

| Option | Strengths | Weaknesses | Verdict |
|---|---|---|---|
| A. All Dart (Flutter only) | One language, simplest build, hot reload everywhere | Real-time audio in Dart is not feasible (GC pauses, no RT thread control) so native audio code is needed anyway; DSP performance; no reusable SDK for CLI and third parties | Rejected |
| B. Flutter + C++ (JUCE or custom) | Mature audio ecosystem, JUCE plugin hosting | JUCE is GPL or commercial [To verify]; memory safety; C++ build complexity on 5 platforms; solo-developer productivity | Rejected for core. JUCE only if plugin hosting becomes a requirement |
| C. Tauri or Electron with web UI + Rust | Web tech UI, Rust backend, existing web notation libraries (alphaTab) | Mobile support immature for Tauri [Assumption]; canvas performance for large scores depends on webview; two runtimes | Rejected. Reconsider if Web becomes the primary target |
| D. Kotlin Multiplatform + Compose | Native feel on Android, shared logic | Desktop audio and DSP still needs native code; Windows and Linux Compose maturity; smaller music ecosystem | Rejected |
| E. Rust core + Flutter UI (selected) | Memory-safe performant core usable from CLI, tests, WASM and future SDK; one UI codebase; good Rust audio crates | Two languages and a bridge; frb bus factor; build complexity across platforms | Selected |
| F. Rust only (egui, iced, Slint) | One language | Weaker mobile story, accessibility, and rich text input [Assumption]; Slint is GPL or commercial or royalty-free with conditions [To verify] | Rejected. Kept as fallback UI if Flutter fails a spike |

## 2. System style alternatives

| Option | Verdict | Reason |
|---|---|---|
| Modular monolith (single process, crates as modules) | Selected | Solo developer, local-first, low-latency in-process calls, compiler-enforced boundaries |
| Microservices | Rejected | No independent scaling or team ownership need. Adds deployment and network failure modes |
| Core as a sidecar process (IPC, gRPC) | Deferred | Only justified for crash isolation of ML or a Python runtime, or a plugin ABI. See [ADR-005](../architecture/decisions/ADR-005-dart-rust-interop-frb.md) |
| Event-sourced core | Rejected for now | Undo, sync and audit do not require it yet. See [score-model-and-undo.md](../architecture/score-model-and-undo.md) |

## 3. Where the domain model lives

| Option | Verdict | Reason |
|---|---|---|
| Rust is the source of truth, Dart renders projections | Selected | One implementation of invariants. CLI, tests and future SDK share it |
| Duplicate model in Dart and Rust | Rejected | Two sources of truth drift. Invariants enforced twice |
| Dart model, Rust only for DSP | Rejected | Loses the Music Core SDK goal and puts invariants in UI code |

## 4. Reconsideration triggers

- frb fails S1 or becomes unmaintained: fall back to hand-written C ABI in `mc-ffi`, keeping the same application API.
- Flutter fails S4 (canvas performance) on desktop: evaluate a Rust-rendered canvas texture, or option F.
- Web becomes the primary platform: reevaluate option C.
- Plugin hosting (VST3, CLAP, AU) becomes a requirement: plugin host process and possibly C++ components.
