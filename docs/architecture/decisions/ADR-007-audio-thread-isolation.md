# ADR-007 Audio thread isolation

Status: Proposed (Accepted when S2 and S3 pass)
Date: 2026-10-31
Related documents: [audio-dsp-architecture.md](../audio-dsp-architecture.md), [performance-requirements.md](../performance-requirements.md)

## Context
Audio callbacks must meet hard deadlines. Libraries do not guarantee real-time safety on their own.

## Decision
Separate control plane (non-RT control thread, SPSC rings) from data plane (RT callbacks). RT code lives in `mc-audio/src/rt/`, has no allocation, locks, I/O, logging or FFI, and swaps preallocated timelines by pointer with off-thread deallocation. Tuner analysis runs on its own thread.

## Alternatives
Mutex-protected shared state; rendering on the UI or command thread into a large buffer; using rodio's higher-level abstractions.

## Trade-offs
Glitch-free design at the cost of more structure (rings, swap protocol) and testing effort.

## Consequences
Assumptions A1 to A10 must be validated. RT code reviewed with a checklist.

## Related fitness functions
FF-16, FF-17, FF-18.

## Reconsideration conditions
A1 fails (synth replacement); latency requirements for live monitoring or effects processing arrive (may need exclusive mode or ASIO).
