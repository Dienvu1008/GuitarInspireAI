# Guitar Pro export checked with TuxGuitar 2.0.1 (ADR-036 G9) and timing (G11)

Purpose: Evidence for criteria G9 and G11 of [ADR-036](../docs/architecture/decisions/ADR-036-guitar-pro-adapter.md).
- G9: TuxGuitar's own readers open our exports, and their view equals our re-import.
- G11: import and export time.

Date: 2026-11-11. Reference machine ([reference-machine.md](reference-machine.md)), single machine (ADR-025).
Related: [S10 spike](../spikes/s10-guitar-pro/README.md). `ReadSummary.java` is the black-box program: it uses only TuxGuitar's public API.

## G9 procedure

Tools: TuxGuitar 2.0.1 installed in `C:\Program Files\tuxguitar`, and a JDK 21 (Temurin 21.0.13+9 was used).

1. Write the inputs. The command writes our exports of `feature_rich`, `techniques` and `reference(TYPICAL)` to GP5, GP4 and GP3. Next to each file it writes our re-import, `<file>.ours.txt`, in `ReadSummary`'s text form.
   ```powershell
   $env:GP_G9_DIR = "$env:TEMP\gp-g9"
   cargo test -p mc-guitarpro --test export g9 -- --ignored
   ```
2. Read each file with TuxGuitar:
   ```powershell
   $tg = 'C:\Program Files\tuxguitar'
   $cp = "$tg\lib\tuxguitar-lib.jar;$tg\lib\tuxguitar-gm-utils.jar;$tg\share\plugins\tuxguitar-gtp.jar"
   javac -encoding UTF-8 -cp $cp -d $env:TEMP\s10-gen spikes\s10-guitar-pro\generator\ReadSummary.java
   java -cp "$cp;$env:TEMP\s10-gen" ReadSummary $env:TEMP\gp-g9\feature_rich.gp5
   ```
3. Compare TuxGuitar's lines with `<file>.ours.txt`, line by line. The compared lines are:
   - each track with its name, string count and capo;
   - every voice line: duration, dot, tuplet, rest, pick stroke on voice 1, and each note with string, fret, tie, dead, accent, palm mute and let ring;
   - every bar with an alternate ending or a repeat close.

## G9 result

| File | TuxGuitar lines | Equal to our re-import |
|---|---|---|
| feature_rich.gp5 | 9 | 9 |
| feature_rich.gp4 | 8 | 8 |
| feature_rich.gp3 | 8 | 8 |
| techniques.gp5 | 3 | 2 |
| techniques.gp4 | 3 | 2 |
| techniques.gp3 | 3 | 3 |
| typical.gp5 | 605 | 605 |
| typical.gp4 | 605 | 605 |
| typical.gp3 | 605 | 605 |
| **Total** | **1 849** | **1 847** |

**TuxGuitar opens all 9 files.**

**The 2 differing lines** are the same note of `techniques`, in GP4 and GP5. It carries palm mute and let ring. TuxGuitar reads only the palm mute.
- The cause was checked on TuxGuitar's model (public API): `TGNoteEffect.setPalmMute(true)` clears let ring, and `setLetRing(true)` clears palm mute. The two are exclusive in TuxGuitar.
- Our file sets both bits, as the format allows, and our importer reads both back.
- This is a TuxGuitar limit, recorded as an evidence gap. The encoding is not changed.

**Found and fixed during this check.** TuxGuitar read our first GP3 and GP4 exports of `feature_rich` with one extra quarter rest. Cause: an empty (status 0) beat that carried a tempo change in an empty bar.
- In GP5, TuxGuitar treats such a beat as empty.
- In GP3 and GP4, it reads it as a rest.
- The exporter now writes no empty beats in GP3 and GP4. Such a tempo change goes to a beat of another track in the same bar, or it is dropped and counted.

## G11 result

Command: `cargo test -p mc-guitarpro --release --test export g11 -- --ignored --nocapture`. GP5, 11 runs each; the median and maximum are shown.

| Score | Notes | File size | Export median (max) | Import median (max) | Limit |
|---|---|---|---|---|---|
| reference(TYPICAL) | 9 600 | 84 523 bytes | 1.2 ms (2.0) | 3.4 ms (4.6) | 100 ms |
| reference(LARGE) | 288 000 | 2 049 147 bytes | 26.1 ms (39.1) | 73.1 ms (126.5) | 1 000 ms |
