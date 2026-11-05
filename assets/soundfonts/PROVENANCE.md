# SoundFont provenance

`GuitarSteel.sf2` is a trimmed derivative of FluidR3 GM, kept small enough to bundle and commit (ADR-026). It contains only the preset playback uses (ADR-024).

## Source

| Item | Value |
|---|---|
| Source archive | `fluid-soundfont_3.1.orig.tar.gz` |
| Source URL | https://deb.debian.org/debian/pool/main/f/fluid-soundfont/fluid-soundfont_3.1.orig.tar.gz |
| Archive size | 134 835 922 bytes |
| Archive SHA-256 | `2621ACAA1C78E4ABDB24BDD163230CC577E61276936D6AA6E3180582142F0343` |
| Source SoundFont | `fluid-soundfont-3.1/FluidR3_GM.sf2` |
| Source SoundFont size | 148 398 306 bytes |
| Source SoundFont SHA-256 | `74594E8F4250680ADF590507A306655A299935343583256F3B722C48A1BC1CB0` |

## License

Fluid (R3) SoundFont, copyright (c) 2000-2002, 2008 Frank Wen, released under the MIT license (the original `COPYING`, stored here as `FluidR3-COPYING.txt`). MIT permits use, modification and redistribution, including derivative works, provided the copyright notice and permission notice are included. `GuitarSteel.sf2` is such a derivative and is distributed under the same MIT license. The release third-party notices include this license and copyright.

## Trim

Reproducible from the repository (standard-library Python only). From the extracted source:

```
python tools/sf2-trim/trim_sf2.py FluidR3_GM.sf2 GuitarSteel.sf2 --preset 0:25
```

- Tool: `tools/sf2-trim/trim_sf2.py` (rebuilds every pdta sub-chunk, remaps instrument and sample indices, compacts the sample pool; the INFO chunk, with the original authorship, is preserved).
- Kept presets: bank 0, program 25, "Steel String Guitar" (General MIDI "Acoustic Guitar (steel)").
- Verify with `python tools/sf2-trim/probe.py GuitarSteel.sf2` (prints `bank=0 program=25 Steel String Guitar`).

## Result

| Item | Value |
|---|---|
| File | `GuitarSteel.sf2` |
| Size | 2 108 670 bytes |
| SHA-256 | `8C9B74EF954235164D8C5EA58FE10C8687F808D92FBB23BA388F5CE5FAB49EF8` |
| Presets | bank 0 program 25 ("Steel String Guitar") |
| Samples | 10 of 1418 in the source (0.7% kept) |
| Compressed size | 1 836 913 bytes (zip deflate, ~1.75 MiB) |
| Validated | loads in rustysynth 1.3.8; every semitone E2..=E6 (MIDI 40..=88) renders audio at the correct fundamental (within +4/-1 cents; measured by autocorrelation at 48 kHz) |

Regenerating from the same source archive produces a byte-identical file.

## Verification

Two reproducible checks guard the trim:

- `python tools/sf2-trim/verify.py <source FluidR3_GM.sf2> assets/soundfonts/GuitarSteel.sf2`
  confirms the sample count (10 of 1418), and that every kept sample is copied intact:
  same frame count and loop points as in the source, in bounds (the sample-pool
  compaction did not truncate a sample or lose a loop).
- `cargo test -p mc-audio --test soundfont_pitch -- --ignored` renders program 25 across
  E2..=E6 and asserts each note is audible at its expected pitch. E2 and the lowest notes
  are produced by pitch-shifting the "Steel E3" sample down; the test confirms there is
  no dead region. Device-free, run on the developer machine (ADR-025).
