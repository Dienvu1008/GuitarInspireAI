"""Verify a trimmed SF2 against its source: sample counts, loop points, no truncation.

Standard library only. Checks that every sample kept in the trimmed file is copied
intact from the source (same frame count, same loop points relative to the sample
start), so the sample-pool compaction did not cut any sample short or lose a loop.

Usage:
    python verify.py <source.sf2> <trimmed.sf2>
"""

import struct
import sys

SHDR = 46


def read_chunks(buf, start, end):
    i = start
    while i + 8 <= end:
        cid = buf[i : i + 4].decode("latin-1")
        size = struct.unpack_from("<I", buf, i + 4)[0]
        yield cid, i + 8, size
        i += 8 + size + (size & 1)


def load(path):
    buf = open(path, "rb").read()
    assert buf[0:4] == b"RIFF" and buf[8:12] == b"sfbk", f"{path}: not an SF2"
    smpl = b""
    pdta = {}
    for cid, off, size in read_chunks(buf, 12, len(buf)):
        if cid != "LIST":
            continue
        typ = buf[off : off + 4].decode("latin-1")
        body, bend = off + 4, off + size
        if typ == "sdta":
            for sub, so, ss in read_chunks(buf, body, bend):
                if sub == "smpl":
                    smpl = buf[so : so + ss]
        elif typ == "pdta":
            for sub, so, ss in read_chunks(buf, body, bend):
                pdta[sub] = buf[so : so + ss]
    return smpl, pdta


def shdrs(pdta):
    raw = pdta["shdr"]
    out = []
    for i in range(0, len(raw) - SHDR + 1, SHDR):
        rec = raw[i : i + SHDR]
        name = rec[:20].split(b"\0")[0].decode("latin-1")
        start, end, startloop, endloop, rate = struct.unpack_from("<IIIII", rec, 20)
        pitch = rec[44]
        out.append((name, start, end, startloop, endloop, rate, pitch))
    return out


def main(src_path, trim_path):
    src_smpl, src_pdta = load(src_path)
    trim_smpl, trim_pdta = load(trim_path)
    src = shdrs(src_pdta)
    trim = shdrs(trim_path and trim_pdta)

    # The terminal "EOS" record is not a real sample.
    src_real = [s for s in src if s[0] != "EOS"]
    trim_real = [s for s in trim if s[0] != "EOS"]
    print(f"source samples : {len(src_real)}")
    print(f"trimmed samples: {len(trim_real)}  "
          f"({len(trim_real) / len(src_real) * 100:.1f}% kept)")
    print(f"source smpl pool : {len(src_smpl) // 2} frames")
    print(f"trimmed smpl pool: {len(trim_smpl) // 2} frames")

    # Index source samples by (name, frame_count, loop offsets) for matching.
    src_by_name = {}
    for name, start, end, sl, el, rate, pitch in src_real:
        src_by_name.setdefault(name, []).append(
            (end - start, sl - start, el - start, rate, pitch)
        )

    problems = []
    print("\ntrimmed samples:")
    for name, start, end, sl, el, rate, pitch in trim_real:
        frames = end - start
        loop_start = sl - start
        loop_end = el - start
        # Bounds inside the pool (plus the 46-frame guard the trim adds).
        pool_frames = len(trim_smpl) // 2
        in_bounds = 0 <= start <= end <= pool_frames
        loop_ok = start <= sl <= el <= end
        match = src_by_name.get(name, [])
        intact = any(
            m[0] == frames and m[1] == loop_start and m[2] == loop_end
            for m in match
        )
        flag = "" if (in_bounds and loop_ok and intact) else "  <-- PROBLEM"
        if flag:
            problems.append(name)
        print(
            f"  {name:<22} frames={frames:>7} loop=[{loop_start},{loop_end}] "
            f"rate={rate} pitch={pitch}{flag}"
        )
        if not in_bounds:
            print(f"      out of bounds: start={start} end={end} pool={pool_frames}")
        if not loop_ok:
            print(f"      bad loop: start={start} sl={sl} el={el} end={end}")
        if not intact:
            print(f"      no intact source match for '{name}' "
                  f"(frames={frames}, loop=[{loop_start},{loop_end}])")

    print()
    if problems:
        print(f"FAIL: {len(problems)} sample(s) with problems: {problems}")
        sys.exit(1)
    print("OK: every trimmed sample matches its source intact "
          "(frame count and loop points preserved, in bounds)")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
