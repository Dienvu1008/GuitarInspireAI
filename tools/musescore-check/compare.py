"""ADR-033 E8 and E9: compare our exports with what MuseScore makes of them.

Standard library only. Two commands:

  compare.py midi OURS.mid THEIRS.mid
      Notes of both Standard MIDI Files as (pitch, onset, length) in quarter notes,
      repeats as each file plays them. Every note must match in pitch and onset.
      A MuseScore note that ends exactly one of its own ticks early counts as the
      same length (MuseScore releases every note one tick before its written end).
      Other length differences are classified: "dead" (our velocity 40, ADR-024
      dead note), "ours rings longer" (let ring is rendered to the end of the bar by
      our exporter), or "mismatch".

  compare.py strings OURS.musicxml THEIRS.musicxml
      String and fret of every note, per part and measure, as multisets. Also counts
      <tied type="let-ring"/> in each file.

  compare.py techniques OURS.musicxml THEIRS.musicxml
      ADR-035 TQ-8: for every note (matched by part, measure, string, fret and
      occurrence) and every rest (by part, measure and occurrence), which of our
      technique marks MuseScore kept: accent, down-bow, up-bow, x notehead and the
      <other-technical> texts. Prints per technique "kept", "lost" or "partly kept",
      plus the elements MuseScore wrote instead on those notes. A loss is reported,
      not hidden; the exit status is 1 when anything was lost.

Exit status 0 when there is no mismatch, 1 otherwise.
"""

from __future__ import annotations

import sys
import xml.etree.ElementTree as ET
from collections import Counter, defaultdict
from fractions import Fraction


def _vlq(data: bytes, i: int) -> tuple[int, int]:
    v = 0
    while True:
        b = data[i]
        i += 1
        v = (v << 7) | (b & 0x7F)
        if not b & 0x80:
            return v, i


def midi_ppq(path: str) -> int:
    return int.from_bytes(open(path, "rb").read()[12:14], "big")


def midi_notes(path: str) -> list[tuple[int, Fraction, Fraction, int]]:
    """(pitch, onset, length, velocity) for every note, in quarter notes."""
    data = open(path, "rb").read()
    assert data[:4] == b"MThd", f"{path}: not a MIDI file"
    ntrks = int.from_bytes(data[10:12], "big")
    ppq = int.from_bytes(data[12:14], "big")
    assert not ppq & 0x8000, "SMPTE timing not supported"
    i = 8 + int.from_bytes(data[4:8], "big")
    notes = []
    for _ in range(ntrks):
        assert data[i : i + 4] == b"MTrk"
        end = i + 8 + int.from_bytes(data[i + 4 : i + 8], "big")
        i += 8
        tick, status = 0, 0
        sounding: dict[tuple[int, int], list[tuple[int, int]]] = defaultdict(list)
        while i < end:
            delta, i = _vlq(data, i)
            tick += delta
            if data[i] & 0x80:
                status = data[i]
                i += 1
            if status == 0xFF:
                i += 1  # meta type
                length, i = _vlq(data, i)
                i += length
                continue
            if status in (0xF0, 0xF7):
                length, i = _vlq(data, i)
                i += length
                continue
            kind, ch = status & 0xF0, status & 0x0F
            if kind in (0xC0, 0xD0):
                i += 1
                continue
            a, b = data[i], data[i + 1]
            i += 2
            if kind == 0x90 and b > 0:
                sounding[(ch, a)].append((tick, b))
            elif kind == 0x80 or (kind == 0x90 and b == 0):
                if sounding[(ch, a)]:
                    on, vel = sounding[(ch, a)].pop(0)
                    notes.append((a, Fraction(on, ppq), Fraction(tick - on, ppq), vel))
        i = end
    return sorted(notes)


def compare_midi(ours_path: str, theirs_path: str) -> int:
    ours, theirs = midi_notes(ours_path), midi_notes(theirs_path)
    tick = Fraction(1, midi_ppq(theirs_path))
    print(f"notes: ours {len(ours)}, MuseScore {len(theirs)}")
    by_start = defaultdict(list)
    for p, on, ln, v in theirs:
        by_start[(p, on)].append(ln)
    missing, cats = [], Counter()
    examples: dict[str, list] = defaultdict(list)
    for p, on, ln, v in ours:
        lens = by_start.get((p, on))
        if not lens:
            missing.append((p, on))
            continue
        # Take the closest length at the same pitch and onset.
        their = min(lens, key=lambda x: abs(x - ln))
        lens.remove(their)
        if their == ln:
            cats["same length"] += 1
            continue
        if their == ln - tick:
            cats["same length (MuseScore one tick early)"] += 1
            continue
        if v == 40:
            cat = "dead (ours 1/64, velocity 40)"
        elif ln > their:
            cat = "ours rings longer (let ring)"
        else:
            cat = "mismatch"
        cats[cat] += 1
        if len(examples[cat]) < 5:
            examples[cat].append((p, str(on), str(ln), str(their)))
    extra = sum(len(v) for v in by_start.values())
    for cat, n in sorted(cats.items()):
        print(f"  {cat}: {n}")
        for e in examples.get(cat, []):
            print(f"      pitch {e[0]} onset {e[1]} ours {e[2]} theirs {e[3]}")
    print(f"  ours without a MuseScore note at the same pitch and onset: {len(missing)}")
    for m in missing[:10]:
        print(f"      pitch {m[0]} onset {m[1]}")
    print(f"  MuseScore notes without one of ours: {extra}")
    bad = len(missing) + extra + cats["mismatch"]
    print("RESULT:", "match" if bad == 0 else f"{bad} difference(s)")
    return 0 if bad == 0 else 1


def _local(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def strings_of(path: str):
    """{(part index, measure index): Counter((string, fret))} and the let-ring count."""
    root = ET.parse(path).getroot()
    out: dict[tuple[int, int], Counter] = defaultdict(Counter)
    let_ring = 0
    parts = [e for e in root if _local(e.tag) == "part"]
    for pi, part in enumerate(parts):
        measures = [e for e in part if _local(e.tag) == "measure"]
        for mi, m in enumerate(measures):
            for note in m.iter():
                if _local(note.tag) != "note":
                    continue
                s = f = None
                for e in note.iter():
                    t = _local(e.tag)
                    if t == "string":
                        s = int(e.text)
                    elif t == "fret":
                        f = int(e.text)
                    elif t == "tied" and e.get("type") == "let-ring":
                        let_ring += 1
                if s is not None and f is not None:
                    out[(pi, mi)][(s, f)] += 1
    return out, let_ring, len(parts)


def compare_strings(ours_path: str, theirs_path: str) -> int:
    ours, ring_ours, parts_ours = strings_of(ours_path)
    theirs, ring_theirs, parts_theirs = strings_of(theirs_path)
    total = sum(sum(c.values()) for c in ours.values())
    total_theirs = sum(sum(c.values()) for c in theirs.values())
    print(f"parts: ours {parts_ours}, MuseScore {parts_theirs}")
    print(f"notes with string and fret: ours {total}, MuseScore {total_theirs}")
    diffs = []
    for key in sorted(set(ours) | set(theirs)):
        if ours.get(key, Counter()) != theirs.get(key, Counter()):
            diffs.append(key)
    for key in diffs[:10]:
        print(f"  part {key[0]+1} measure {key[1]}: ours {dict(ours.get(key, {}))} "
              f"theirs {dict(theirs.get(key, {}))}")
    print(f"measures that differ: {len(diffs)}")
    print(f"let ring (<tied type=\"let-ring\"/>): ours {ring_ours}, MuseScore {ring_theirs}")
    print("RESULT:", "match" if not diffs and parts_ours == parts_theirs else "differences")
    return 0 if not diffs and parts_ours == parts_theirs else 1


TECHNIQUES = ("accent", "down-bow", "up-bow", "notehead x",
              "other-technical: palm mute", "other-technical: string percussive")


def techniques_of(path: str):
    """{key: (set of our technique names, set of other element names)} per note/rest."""
    root = ET.parse(path).getroot()
    out = {}
    parts = [e for e in root if _local(e.tag) == "part"]
    for pi, part in enumerate(parts):
        measures = [e for e in part if _local(e.tag) == "measure"]
        for mi, m in enumerate(measures):
            seen = Counter()
            for note in m:
                if _local(note.tag) != "note":
                    continue
                tags = {}
                for e in note.iter():
                    tags.setdefault(_local(e.tag), []).append(e)
                if "rest" in tags:
                    if "type" not in tags:
                        continue  # padding or measure rest
                    base = (pi, mi, "rest")
                else:
                    s = int(tags["string"][0].text) if "string" in tags else None
                    f = int(tags["fret"][0].text) if "fret" in tags else None
                    base = (pi, mi, s, f)
                key = base + (seen[base],)
                seen[base] += 1
                found = set()
                for name in ("accent", "down-bow", "up-bow"):
                    if name in tags:
                        found.add(name)
                if any((e.text or "").strip() == "x" for e in tags.get("notehead", [])):
                    found.add("notehead x")
                for e in tags.get("other-technical", []):
                    found.add("other-technical: " + (e.text or "").strip())
                others = {t for t in tags if t not in (
                    "note", "pitch", "step", "octave", "alter", "duration", "voice",
                    "type", "chord", "notations", "technical", "string", "fret",
                    "articulations", "rest", "staff", "stem", "dot", "beam",
                    "accent", "down-bow", "up-bow", "notehead", "other-technical")}
                out[key] = (found, others)
    return out


def compare_techniques(ours_path: str, theirs_path: str) -> int:
    ours, theirs = techniques_of(ours_path), techniques_of(theirs_path)
    print(f"notes and rests: ours {len(ours)}, MuseScore {len(theirs)}")
    missing = [k for k in ours if k not in theirs]
    print(f"ours without a MuseScore note at the same place: {len(missing)}")
    lost_any = bool(missing)
    for name in TECHNIQUES:
        have = [k for k, (f, _) in ours.items() if name in f]
        kept = [k for k in have if k in theirs and name in theirs[k][0]]
        added = [k for k, (f, _) in theirs.items() if name in f and (k not in ours or name not in ours[k][0])]
        if not have:
            verdict = "not in our file"
        elif len(kept) == len(have):
            verdict = "kept"
        elif kept:
            verdict = "partly kept"
        else:
            verdict = "lost"
        lost_any |= bool(have) and len(kept) < len(have)
        instead = Counter()
        for k in have:
            if k in theirs and name not in theirs[k][0]:
                instead.update(theirs[k][1])
        extra = f"; MuseScore wrote instead on those notes: {dict(instead)}" if instead else ""
        print(f"  {name}: ours {len(have)}, kept {len(kept)}, MuseScore-only {len(added)} -> {verdict}{extra}")
    # Rest strokes separately, since a stroke on a rest is a corner case.
    rests = [k for k, (f, _) in ours.items() if k[2] == "rest" and f & {"down-bow", "up-bow"}]
    rests_kept = [k for k in rests if k in theirs and ours[k][0] & {"down-bow", "up-bow"} <= theirs[k][0]]
    print(f"  stroke on a rest: ours {len(rests)}, kept {len(rests_kept)}")
    print("RESULT:", "all kept" if not lost_any else "losses (evidence gap)")
    return 0 if not lost_any else 1


if __name__ == "__main__":
    cmd, a, b = sys.argv[1:4]
    sys.exit({"midi": compare_midi, "strings": compare_strings,
              "techniques": compare_techniques}[cmd](a, b))
