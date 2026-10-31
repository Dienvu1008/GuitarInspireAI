# Performance Requirements

Purpose: Classify features by computational demand, state provisional budgets with their justification and baseline plan, and define graceful degradation per platform.
Status: Proposed. Budgets are provisional until Phase 0 and Phase 1 baselines exist; then they become Accepted by ADR amendment.
Related documents: [score-model-and-undo.md](score-model-and-undo.md), [interop-architecture.md](interop-architecture.md), [audio-dsp-architecture.md](audio-dsp-architecture.md), [fitness-functions.md](fitness-functions.md), [feasibility-assessment.md](../research/feasibility-assessment.md).

## 1. Reference environment

- Reference desktop: the developer's Windows 10 or 11 machine, recorded in `benchmarks/reference-machine.md` during Phase 0 (CPU, RAM, audio device, buffer size) [To define].
- Reference scores: small, typical and large as defined in [score-model-and-undo.md](score-model-and-undo.md), generated deterministically by a fixture generator.

## 2. Compute classes

| Class | Features | Platforms |
|---|---|---|
| Light | Theory, chord finder, fretboard, editing commands, layout, project load and save | All |
| Moderate (real-time) | Playback synthesis, tuner, vocal pitch tracking, onset and tempo | Desktop and mobile; Web later |
| Heavy | Source separation, polyphonic transcription, large-model arrangement | Desktop (GPU preferred); not mobile in initial versions |

## 3. Provisional budgets

| Metric | Budget | Justification | Baseline plan |
|---|---|---|---|
| Edit command + visible projection | p99 at most 16 ms on typical score; at most 33 ms on large | 60 fps feedback while typing; 30 fps acceptable on large scores | S1 and Phase 1 criterion benches |
| Open typical project | at most 500 ms | Perceived as immediate in desktop apps [Assumption] | S5 |
| Open large project | at most 2 s with progress | Acceptable with feedback | S5 |
| Save typical project | at most 300 ms, off UI thread | Never blocks editing | S5 |
| Playback start | under 100 ms from click to sound | Feels responsive for practice | S2 |
| Audio underruns | zero in 10 min reference playback | Audible glitches are unacceptable | S2 (A4) |
| Tuner latency | at most about 100 ms | Tuning feels live | S3 (A8) |
| Tuner accuracy | within plus or minus 2 cents steady state | Common tuner tolerance [Assumption] | S3 (A9) |
| Memory, large score open | under 300 MB process total excluding SoundFont [Assumption] | Desktop comfort | Phase 1 and 3 baselines |
| Undo memory | under 64 MB default budget | See undo policy | Phase 1 |
| TAB scroll | no frame over 16 ms on 500-bar track | Smooth scrolling | S4 |

Regression policy after baselines: Important flag when a benchmark regresses more than 10 percent versus the stored baseline on the same machine in three consecutive runs (to filter noise). The 10 percent value is a starting point justified by typical criterion noise of a few percent [Assumption]; adjust after observing variance.

## 4. Graceful degradation

| Situation | Behaviour |
|---|---|
| No audio output device | Editor works; playback disabled with message |
| No microphone or permission denied | Tuner shows reason and link to OS settings |
| SoundFont missing or invalid | Fallback sine synth [Proposed] or disable playback with message |
| Large score on low-end machine | Coarser layout caching, progressive layout of visible systems first |
| Heavy AI feature on mobile | Not offered, or offered as desktop-only with explanation |
| No GPU | CPU path with time estimate; long jobs cancellable |
| Web (later) | No microphone tuner unless secure context and permission; no heavy ML |

## 5. Platform capability matrix (planned)

| Feature | Windows | macOS | Linux | Android | iOS | Web |
|---|---|---|---|---|---|---|
| Editor, playback | MVP | Post-MVP port | Post-MVP port | Phase 7 | Phase 7 | Phase 11 |
| Tuner | MVP | Port | Port | Phase 7 | Phase 7 | Phase 11 (permission) |
| Vocal analysis | Phase 8 | Phase 8 | Phase 8 | Phase 8 (light) | Phase 8 (light) | Later |
| Transcription and separation | Phase 9 | Phase 9 | Phase 9 | No (initially) | No (initially) | No |
