# Dependency Map

Purpose: Show dependencies between phases, spikes and user decisions.
Status: Proposed.
Related documents: [implementation-phases.md](implementation-phases.md), [technical-spikes.md](../research/technical-spikes.md), [open-questions.md](../research/open-questions.md), [milestones.md](milestones.md).

Arrows point from a prerequisite to the work it unblocks.

```mermaid
flowchart TD
  docs["Architecture package (this assignment)"]
  p0["Phase 0 Foundation"]
  s1["S1 frb interop"]
  s2["S2 RT playback"]
  s3["S3 tuner DSP"]
  s4["S4 TAB rendering"]
  s5["S5 project format"]
  s7["S7 license audit"]
  p1["Phase 1 Core domain and format"]
  p2["Phase 2 Guitar core"]
  p3["Phase 3 Desktop TAB editor"]
  p4["Phase 4 Playback and MIDI"]
  p5["Phase 5 Tools and MVP"]
  q1["Q-01 to Q-06 DSL decisions"]
  p6a["Phase 6a Pattern engine"]
  p6b["Phase 6b Multi-track and MusicXML"]
  s11["S11 engraving"]
  p6c["Phase 6c Standard notation and PDF"]
  s10["S10 Guitar Pro"]
  p6d["Phase 6d GP import"]
  s12["S12 mobile"]
  p7["Phase 7 Mobile"]
  p8["Phase 8 Vocal analysis"]
  s8["S8 ML runtime"]
  s9["S9 separation"]
  p9["Phase 9 MIR and ML"]
  p10["Phase 10 Arrangement"]
  p11["Phase 11 Ecosystem"]
  docs --> p0
  p0 --> s1
  p0 --> s2
  p0 --> s4
  p0 --> s5
  p0 --> s7
  s5 --> p1
  s7 --> p1
  p1 --> p2
  p1 --> p3
  p2 --> p3
  s1 --> p3
  s4 --> p3
  p3 --> p4
  s2 --> p4
  p2 --> p5
  p4 --> p5
  s3 --> p5
  p5 --> p6a
  q1 --> p6a
  p5 --> p6b
  p5 --> p6c
  s11 --> p6c
  p5 --> p6d
  s10 --> p6d
  p5 --> p7
  s12 --> p7
  p5 --> p8
  p8 --> p9
  s8 --> p9
  s9 --> p9
  p6a --> p10
  p9 --> p10
  p5 --> p11
```

Critical path to MVP: Phase 0 (S1, S4, S5) to Phase 1 to Phase 3 to Phase 4 to Phase 5. Phase 2 can run in parallel with early Phase 3 work in principle, but a solo developer executes sequentially; the estimates assume sequential work.
