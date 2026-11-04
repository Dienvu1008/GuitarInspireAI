# ADR-025 Single-machine performance and glitch evidence accepted (R-21 deferred)
Status: Accepted (2026-10-31, user decision at Phase 4 entry)
Date: 2026-10-31
Related documents: [risk-register.md](../../roadmap/risk-register.md) (R-02, R-21), [milestones.md](../../roadmap/milestones.md) (M4), [ADR-007](ADR-007-audio-thread-isolation.md), [performance-requirements.md](../performance-requirements.md), [benchmarks/reference-machine.md](../../../benchmarks/reference-machine.md)

## Context
R-21 requires S2 and S4 to be re-run on at least one other Windows machine before M4, and R-02 plans a second-machine re-measurement of audio glitches in Phase 4. The project has one machine (the reference machine recorded in `benchmarks/reference-machine.md`). No second machine is available.

## Decision
1. The second-machine measurement is not a condition for M4. R-21 stays open; the risk is accepted and deferred until a second machine is available (at the latest before the first public release).
2. All performance and glitch evidence (S2 re-runs, FF-18, A1 to A7 in the product build, edit-feedback and benchmark numbers) is valid only for the reference machine. Reports state this; they do not present it as covering other hardware.
3. FF-18 still runs, on the reference machine. Each run records the machine configuration (CPU, RAM, OS build, audio device and driver, SoundFont file and SHA-256) in `benchmarks/` so later runs on other machines can be compared.

## Alternatives
- Block M4 until a second machine exists: stops the project for a hardware reason with no plan to obtain it.
- Borrow or rent a machine for one run: possible later; not available now.
- Hosted CI runners as the second machine: they have no audio device and noisy timing; their numbers would not be comparable.

## Trade-offs
Faster progress against unknown behaviour on lower-end or different audio hardware (USB interfaces, onboard codecs, other drivers). Mitigations already in place: MMCSS promotion, callback-timing diagnostics, warm-up after SoundFont load, and the runtime underrun counters that will show problems on user machines.

## Consequences
M4 evidence says "one machine". R-21 is listed as open in the Phase 4 exit report. The M0 exit report's plan for a second machine is superseded by this ADR.

## Related fitness functions
FF-18 (glitch test), FF-21 (benchmarks).

## Reconsideration conditions
A second Windows machine becomes available; a user or tester reports glitches; before the first public release.
