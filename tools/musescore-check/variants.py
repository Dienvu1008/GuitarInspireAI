"""ADR-035 interop investigation: candidate encodings for palm mute and a stroke on a
rest, derived from a techniques export in the FIRST 6a-1 encoding (commit 1290325:
palm mute as <other-technical> only, a rest stroke as <technical><down-bow/>).
Standard library only. The results are in benchmarks/musescore-musicxml.md.

Usage:
  git show 1290325:crates/mc-musicxml/tests/golden/techniques.musicxml > first.musicxml
  python tools/musescore-check/variants.py first.musicxml OUTDIR

Then validate OUTDIR/*.musicxml against the pinned XSD (as in
tools/musicxml-schema/validate.sh) and convert each with MuseScore (-o to .musicxml,
.mid, .mscz, .png). Writes, into the output directory:
  control.musicxml        golden, ElementTree round trip only (no change)
  palm-play-text.musicxml <play><mute>palm</mute></play> added; other-technical kept
  palm-play.musicxml      <play><mute>palm</mute></play> only; other-technical removed
  rest-direction.musicxml stroke on a rest as <direction> with
                          <other-direction smufl="guitarStrum..."/>; technical removed
  rest-both.musicxml      direction added, technical kept
  palm-play-words.musicxml <play> plus a "P.M." <words> direction with <dashes>
                          over each run of palm-muted beats
"""

import copy
import sys
import xml.etree.ElementTree as ET

SRC, OUT = sys.argv[1], sys.argv[2]


def load():
    return ET.parse(SRC)


def save(tree, name):
    tree.write(f"{OUT}/{name}.musicxml", encoding="UTF-8", xml_declaration=True)


def notes(tree):
    for part in tree.getroot().iter("part"):
        for measure in part.iter("measure"):
            for note in list(measure):
                if note.tag == "note":
                    yield measure, note


def palm(note):
    return [e for e in note.iter("other-technical") if (e.text or "").strip() == "palm mute"]


def add_play(note):
    # note content: ... notations*, lyric*, play?, listen? -> after the last notations/lyric
    play = ET.Element("play")
    ET.SubElement(play, "mute").text = "palm"
    kids = list(note)
    last = max(i for i, k in enumerate(kids) if k.tag in ("notations", "lyric"))
    note.insert(last + 1, play)


def palm_variant(keep_text):
    tree = load()
    n = 0
    for _, note in notes(tree):
        marks = palm(note)
        if not marks:
            continue
        add_play(note)
        n += 1
        if not keep_text:
            for tech in note.iter("technical"):
                for m in marks:
                    if m in list(tech):
                        tech.remove(m)
    return tree, n


def rest_variant(keep_technical):
    tree = load()
    n = 0
    for measure, note in list(notes(tree)):
        if note.find("rest") is None:
            continue
        nots = note.find("notations")
        tech = nots.find("technical") if nots is not None else None
        if tech is None:
            continue
        glyph = None
        if tech.find("down-bow") is not None:
            glyph = "guitarStrumDown"
        elif tech.find("up-bow") is not None:
            glyph = "guitarStrumUp"
        if glyph is None:
            continue
        # A direction placed directly before the rest is at the rest's onset (the
        # MusicXML time cursor), in the rest's voice; no offset needed.
        d = ET.Element("direction", {"placement": "above"})
        dt = ET.SubElement(d, "direction-type")
        ET.SubElement(dt, "other-direction", {"smufl": glyph})
        ET.SubElement(d, "voice").text = note.find("voice").text
        measure.insert(list(measure).index(note), d)
        n += 1
        if not keep_technical:
            nots.remove(tech)
            if len(nots) == 0:
                note.remove(nots)
    return tree, n


def palm_words_variant():
    """<play> on each palm-muted note, plus a "P.M." <words> direction before each
    palm-muted beat: dashes start on the first beat of a run of consecutive muted
    beats and stop before its last beat (no dashes for a beat alone)."""
    tree, _ = palm_variant(True)
    for part in tree.getroot().iter("part"):
        for measure in part.iter("measure"):
            beats = []  # (index of first note element, muted?)
            kids = list(measure)
            for i, k in enumerate(kids):
                if k.tag == "note" and k.find("chord") is None:
                    beats.append([k])
                elif k.tag == "note" and beats:
                    beats[-1].append(k)
            muted = [any(n.find("play") is not None for n in b) for b in beats]
            runs, i = [], 0
            while i < len(beats):
                if muted[i]:
                    j = i
                    while j + 1 < len(beats) and muted[j + 1]:
                        j += 1
                    runs.append((i, j))
                    i = j + 1
                else:
                    i += 1
            for (a, b) in runs:
                def direction(dash):
                    d = ET.Element("direction", {"placement": "below"})
                    if dash != "stop":
                        dt = ET.SubElement(d, "direction-type")
                        ET.SubElement(dt, "words").text = "P.M."
                    if dash:
                        dt = ET.SubElement(d, "direction-type")
                        ET.SubElement(dt, "dashes", {"type": dash, "number": "1"})
                    ET.SubElement(d, "voice").text = beats[a][0].find("voice").text
                    return d
                kids = list(measure)
                measure.insert(kids.index(beats[a][0]), direction("start" if b > a else None))
                if b > a:
                    kids = list(measure)
                    measure.insert(kids.index(beats[b][0]), direction("stop"))
    return tree


save(load(), "control")
save(palm_words_variant(), "palm-play-words"); print("palm-play-words written")
t, n = palm_variant(True); save(t, "palm-play-text"); print("palm-play-text: play on", n, "notes")
t, n = palm_variant(False); save(t, "palm-play"); print("palm-play: play on", n, "notes")
t, n = rest_variant(False); save(t, "rest-direction"); print("rest-direction:", n, "rest(s)")
t, n = rest_variant(True); save(t, "rest-both"); print("rest-both:", n, "rest(s)")
