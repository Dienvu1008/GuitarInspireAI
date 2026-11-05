# Bundle size report (FF-29)

Purpose: Record the Windows release bundle size before and after bundling the SoundFont
(slice 4c, ADR-026), so the Phase 4 exit report and later size trends have a baseline.
Status: Measured 2026-10-31 on the reference machine (single machine, ADR-025). FF-29 is
Advisory: these are reference numbers, not a gate.

Measured from `flutter build windows --release` output at
`app/build/windows/x64/runner/Release/` (total of all files, recursive).

| Item | Bytes | MiB |
|---|---|---|
| Release bundle without the SoundFont | 31 979 856 | 30.50 |
| Trimmed SoundFont `GuitarSteel.sf2` | 2 108 670 | 2.01 |
| Release bundle with the SoundFont | 34 088 526 | 32.51 |
| **Delta from bundling the SoundFont** | **2 108 670** | **2.01** |

The delta is exactly the trimmed SoundFont: the Windows build copies it next to the
executable (`app/windows/CMakeLists.txt`) and nothing else changed. The SoundFont is the
trimmed FluidR3 derivative (10 of 1418 samples, program 25 only; see
`assets/soundfonts/PROVENANCE.md`): 2 108 670 bytes uncompressed, 1 836 913 bytes zipped.

For comparison, bundling the untrimmed `FluidR3_GM.sf2` (148 398 306 bytes) would have
added ~141 MiB, taking the bundle to ~172 MiB. The trim keeps the bundle near its
no-SoundFont size (ADR-026).

## Phase 5 (M5) bundle (2026-10-31)

After Phase 5 (tuner, chord finder, fretboard; `mc-dsp` with `realfft`/`rustfft` compiled
into the `mc_ffi` DLL):

| Item | MiB |
|---|---|
| Release bundle without the SoundFont | 32.37 |
| Release bundle with the SoundFont | 34.38 |
| Delta vs the Phase 4 bundle (32.51 with SoundFont) | +1.87 |

The +1.87 MiB is the tuner/chord code and the FFT dependency in the native library; the
SoundFont delta is unchanged. Still far below the untrimmed-SoundFont alternative.

## Method (reproducible)

```
flutter build windows --release    # from app/
# total:
(Get-ChildItem app/build/windows/x64/runner/Release -Recurse -File |
  Measure-Object -Property Length -Sum).Sum
# the SoundFont within the bundle:
(Get-Item app/build/windows/x64/runner/Release/GuitarSteel.sf2).Length
```

The "without the SoundFont" figure is the total minus the SoundFont file; both come from
the same build, so the delta is exact rather than two separate builds that could differ
for unrelated reasons.
