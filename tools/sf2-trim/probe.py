"""Quick probe: list presets (bank, program, name) in an SF2 file."""
import struct
import sys


def read_chunks(buf, start, end):
    i = start
    while i + 8 <= end:
        cid = buf[i : i + 4].decode("latin-1")
        size = struct.unpack_from("<I", buf, i + 4)[0]
        yield cid, i + 8, size
        i += 8 + size + (size & 1)


def main(path):
    buf = open(path, "rb").read()
    assert buf[0:4] == b"RIFF" and buf[8:12] == b"sfbk"
    # Walk top-level LIST chunks to find pdta, then phdr.
    for cid, off, size in read_chunks(buf, 12, len(buf)):
        if cid != "LIST":
            continue
        typ = buf[off : off + 4].decode("latin-1")
        if typ != "pdta":
            continue
        for sub, soff, ssize in read_chunks(buf, off + 4, off + size):
            if sub != "phdr":
                continue
            n = ssize // 38
            for k in range(n):
                rec = soff + k * 38
                name = buf[rec : rec + 20].split(b"\0")[0].decode("latin-1")
                preset, bank = struct.unpack_from("<HH", buf, rec + 20)
                print(f"bank={bank:3d} program={preset:3d} {name}")


if __name__ == "__main__":
    main(sys.argv[1])
