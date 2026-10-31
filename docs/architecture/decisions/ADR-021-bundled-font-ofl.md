# ADR-021 Bundled TAB font under the SIL Open Font License 1.1

Status: Accepted (user, 2026-10-31)
Date: 2026-10-31
Related documents: [ADR-015](ADR-015-dependency-license-policy.md), [ADR-018](ADR-018-notation-layout-in-rust.md), [cross-platform-ui.md](../cross-platform-ui.md), [risk-register.md](../../roadmap/risk-register.md)

## Context
TAB layout is computed in Rust from font metrics that Dart measures (ADR-018). If the app relies on a system font, metrics differ between machines and operating systems, which makes layout goldens and spacing unreliable (R-05). Bundling one font fixes the metrics. The common open fonts suitable for digits (Noto Sans Mono, and most Google Fonts) are licensed under the SIL Open Font License 1.1, which is not on the ADR-015 allow-list.

## Decision
The app bundles Noto Sans Mono (SIL OFL 1.1) as the TAB and editor font. OFL-1.1 is allowed for bundled font files only, not for code dependencies. Conditions:
- The font files are used unmodified; the original `OFL.txt` license file is stored next to them.
- Provenance is recorded in the repository: source URL, version, file names, sizes and SHA-256.
- The release third-party notices include the font license and copyright.
- The font is not sold by itself (OFL condition); bundling it in the application is permitted.

Evaluation (technical, not legal advice): usage category is (c) distributing an unmodified data file with the application. OFL permits bundling and redistribution with software, including commercial software; it applies to the font files only and does not extend to the application. Modified versions may not use the reserved font name, which does not apply to unmodified files.

## Alternatives
- System font (Segoe UI on Windows): no license question, but metrics depend on the operating system and version.
- A font under an allow-listed license (for example Apache-2.0 fonts): fewer suitable monospace digit fonts; Noto Sans Mono has broad glyph coverage and consistent digit widths.

## Trade-offs
A few hundred KB added to the app bundle, and one extra license in the notices, in exchange for deterministic layout metrics across machines.

## Consequences
The license check (FF-09, Dart part) treats files under the app's bundled font directory as data with recorded provenance. ADR-015's allow-list for code is unchanged.

## Related fitness functions
FF-09 (license policy), FF-29 (bundle size).

## Reconsideration conditions
The project license is chosen (Q-10) and conflicts with OFL notices, or a different font is needed for standard notation (Phase 6c).
