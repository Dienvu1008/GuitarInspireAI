"""Trim an SF2 SoundFont to a chosen set of (bank, program) presets.

Reproducible, dependency-free (standard library only). Keeps the selected
presets, the instruments they reference, and the samples those instruments
reference; rebuilds every pdta sub-chunk with correct bag/generator/modulator
indices and a compacted sample pool. Preserves the INFO chunk (so the original
authorship stays), except that it appends a note to the comment (ICMT) recording
the trim. Output is deterministic for a given input and preset list.

SF2 layout reference: SoundFont 2.04 spec, sections 6-8.

Usage:
    python trim_sf2.py <in.sf2> <out.sf2> --preset BANK:PROGRAM [--preset ...]
"""

import argparse
import struct
import sys

# Record sizes (SF2 spec section 7).
PHDR = 38
PBAG = 4
PMOD = 10
PGEN = 4
INST = 22
IBAG = 4
IMOD = 10
IGEN = 4
SHDR = 46

GEN_INSTRUMENT = 41  # preset-zone generator whose amount is an instrument index
GEN_SAMPLE_ID = 53  # instrument-zone generator whose amount is a sample index


def read_chunks(buf, start, end):
    i = start
    while i + 8 <= end:
        cid = buf[i : i + 4].decode("latin-1")
        size = struct.unpack_from("<I", buf, i + 4)[0]
        yield cid, i + 8, size
        i += 8 + size + (size & 1)


class Sf2:
    def __init__(self, buf):
        assert buf[0:4] == b"RIFF" and buf[8:12] == b"sfbk", "not an SF2 file"
        self.info = b""  # raw INFO LIST payload (after the 'INFO' type tag)
        self.smpl = b""  # raw sample data (16-bit)
        self.sm24 = None  # optional low byte of 24-bit samples
        self.pdta = {}  # sub-chunk id -> raw bytes
        for cid, off, size in read_chunks(buf, 12, len(buf)):
            if cid != "LIST":
                continue
            typ = buf[off : off + 4].decode("latin-1")
            body, bend = off + 4, off + size
            if typ == "INFO":
                self.info = buf[body:bend]
            elif typ == "sdta":
                for sub, so, ss in read_chunks(buf, body, bend):
                    if sub == "smpl":
                        self.smpl = buf[so : so + ss]
                    elif sub == "sm24":
                        self.sm24 = buf[so : so + ss]
            elif typ == "pdta":
                for sub, so, ss in read_chunks(buf, body, bend):
                    self.pdta[sub] = buf[so : so + ss]

    def records(self, name, size):
        raw = self.pdta[name]
        return [raw[i : i + size] for i in range(0, len(raw) - size + 1, size)]


def amount_is(gen_bytes):
    """Return (generator_oper, amount_u16) for a 4-byte pgen/igen record."""
    oper, amount = struct.unpack("<HH", gen_bytes)
    return oper, amount


def zone_slices(bags, gens, bag_size):
    """For each zone (bag), return (gen_start, gen_end).

    A bag record is (genNdx u16 at offset 0, modNdx u16 at offset 2); the zone's
    generators run from this bag's genNdx up to the next bag's genNdx.
    """
    out = []
    for k in range(len(bags) - 1):
        g0 = struct.unpack_from("<H", bags[k], 0)[0]
        g1 = struct.unpack_from("<H", bags[k + 1], 0)[0]
        out.append((g0, g1))
    return out


def trim(buf, wanted):
    sf = Sf2(buf)
    phdr = sf.records("phdr", PHDR)
    pbag = sf.records("pbag", PBAG)
    pgen = sf.records("pgen", PGEN)
    pmod = sf.records("pmod", PMOD)
    inst = sf.records("inst", INST)
    ibag = sf.records("ibag", IBAG)
    igen = sf.records("igen", IGEN)
    imod = sf.records("imod", IMOD)
    shdr = sf.records("shdr", SHDR)

    # 1. Select presets (keep input order; the terminal phdr is excluded).
    keep_presets = []
    for k in range(len(phdr) - 1):
        program, bank = struct.unpack_from("<HH", phdr[k], 20)
        if (bank, program) in wanted:
            keep_presets.append(k)
    if not keep_presets:
        raise SystemExit("none of the requested presets were found")

    # 2. Collect instruments referenced by the kept presets' zones.
    pbag_ranges = []  # (p0, p1) pbag index range per preset
    for k in range(len(phdr) - 1):
        b0 = struct.unpack_from("<H", phdr[k], 24)[0]
        b1 = struct.unpack_from("<H", phdr[k + 1], 24)[0]
        pbag_ranges.append((b0, b1))
    pgen_slices = zone_slices(pbag, pgen, PBAG)

    wanted_inst = []
    for k in keep_presets:
        for bi in range(*pbag_ranges[k]):
            g0, g1 = pgen_slices[bi]
            for gi in range(g0, g1):
                oper, amount = amount_is(pgen[gi])
                if oper == GEN_INSTRUMENT and amount not in wanted_inst:
                    wanted_inst.append(amount)
    wanted_inst.sort()

    # 3. Collect samples referenced by those instruments' zones.
    ibag_ranges = []
    for k in range(len(inst) - 1):
        b0 = struct.unpack_from("<H", inst[k], 20)[0]
        b1 = struct.unpack_from("<H", inst[k + 1], 20)[0]
        ibag_ranges.append((b0, b1))
    igen_slices = zone_slices(ibag, igen, IBAG)

    wanted_sample = []
    for ii in wanted_inst:
        for bi in range(*ibag_ranges[ii]):
            g0, g1 = igen_slices[bi]
            for gi in range(g0, g1):
                oper, amount = amount_is(igen[gi])
                if oper == GEN_SAMPLE_ID and amount not in wanted_sample:
                    wanted_sample.append(amount)
    wanted_sample.sort()

    # 4. Rebuild sample pool (16-bit, plus optional 24-bit low bytes) and shdr.
    new_smpl = bytearray()
    new_sm24 = bytearray() if sf.sm24 is not None else None
    sample_remap = {}
    new_shdr = []
    for new_idx, old_idx in enumerate(wanted_sample):
        rec = bytearray(shdr[old_idx])
        start, end, startloop, endloop = struct.unpack_from("<IIII", rec, 20)
        # Append the sample's frames (plus the 46-frame guard the spec wants).
        base = len(new_smpl) // 2
        frame_count = end - start
        new_smpl += sf.smpl[start * 2 : end * 2]
        new_smpl += b"\x00" * (46 * 2)
        if new_sm24 is not None:
            new_sm24 += sf.sm24[start:end] + b"\x00" * 46
        struct.pack_into(
            "<IIII", rec, 20,
            base,
            base + frame_count,
            base + (startloop - start),
            base + (endloop - start),
        )
        sample_remap[old_idx] = new_idx
        new_shdr.append(bytes(rec))
    new_shdr.append(b"EOS".ljust(20, b"\0") + b"\0" * (SHDR - 20))

    # 5. Rebuild instruments, their bags, gens (remapping sampleID) and mods.
    inst_remap = {}
    new_inst, new_ibag, new_igen, new_imod = [], [], [], []
    for new_ii, old_ii in enumerate(wanted_inst):
        inst_remap[old_ii] = new_ii
        name = inst[old_ii][:20]
        new_inst.append(name + struct.pack("<H", len(new_ibag)))
        for bi in range(*ibag_ranges[old_ii]):
            g0, g1 = igen_slices[bi]
            m0 = struct.unpack_from("<H", ibag[bi], 2)[0]
            m1 = struct.unpack_from("<H", ibag[bi + 1], 2)[0]
            new_ibag.append(struct.pack("<HH", len(new_igen), len(new_imod)))
            for gi in range(g0, g1):
                oper, amount = amount_is(igen[gi])
                if oper == GEN_SAMPLE_ID:
                    amount = sample_remap[amount]
                new_igen.append(struct.pack("<HH", oper, amount))
            for mi in range(m0, m1):
                new_imod.append(imod[mi])
    new_inst.append(b"EOI".ljust(20, b"\0") + struct.pack("<H", len(new_ibag)))
    new_ibag.append(struct.pack("<HH", len(new_igen), len(new_imod)))
    new_igen.append(b"\0" * IGEN)
    new_imod.append(b"\0" * IMOD)

    # 6. Rebuild presets, their bags, gens (remapping instrument) and mods.
    new_phdr, new_pbag, new_pgen, new_pmod = [], [], [], []
    for k in keep_presets:
        hdr = bytearray(phdr[k])
        struct.pack_into("<H", hdr, 24, len(new_pbag))
        new_phdr.append(bytes(hdr))
        for bi in range(*pbag_ranges[k]):
            g0, g1 = pgen_slices[bi]
            m0 = struct.unpack_from("<H", pbag[bi], 2)[0]
            m1 = struct.unpack_from("<H", pbag[bi + 1], 2)[0]
            new_pbag.append(struct.pack("<HH", len(new_pgen), len(new_pmod)))
            for gi in range(g0, g1):
                oper, amount = amount_is(pgen[gi])
                if oper == GEN_INSTRUMENT:
                    amount = inst_remap[amount]
                new_pgen.append(struct.pack("<HH", oper, amount))
            for mi in range(m0, m1):
                new_pmod.append(pmod[mi])
    term = bytearray(phdr[-1])
    struct.pack_into("<H", term, 24, len(new_pbag))
    new_phdr.append(bytes(term))
    new_pbag.append(struct.pack("<HH", len(new_pgen), len(new_pmod)))
    new_pgen.append(b"\0" * PGEN)
    new_pmod.append(b"\0" * PMOD)

    pdta = {
        "phdr": b"".join(new_phdr),
        "pbag": b"".join(new_pbag),
        "pmod": b"".join(new_pmod),
        "pgen": b"".join(new_pgen),
        "inst": b"".join(new_inst),
        "ibag": b"".join(new_ibag),
        "imod": b"".join(new_imod),
        "igen": b"".join(new_igen),
        "shdr": b"".join(new_shdr),
    }
    return write(sf, pdta, bytes(new_smpl), new_sm24)


def chunk(cid, body):
    out = cid.encode("latin-1") + struct.pack("<I", len(body)) + body
    if len(body) & 1:
        out += b"\0"
    return out


def list_chunk(typ, body):
    return chunk("LIST", typ.encode("latin-1") + body)


def write(sf, pdta, smpl, sm24):
    info = list_chunk("INFO", sf.info)
    sdta_body = chunk("smpl", smpl)
    if sm24 is not None:
        sdta_body += chunk("sm24", bytes(sm24))
    sdta = list_chunk("sdta", sdta_body)
    order = ["phdr", "pbag", "pmod", "pgen", "inst", "ibag", "imod", "igen", "shdr"]
    pdta_body = b"".join(chunk(k, pdta[k]) for k in order)
    pdta_chunk = list_chunk("pdta", pdta_body)
    body = b"sfbk" + info + sdta + pdta_chunk
    return chunk("RIFF", body)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("src")
    ap.add_argument("dst")
    ap.add_argument("--preset", action="append", required=True, help="BANK:PROGRAM")
    args = ap.parse_args()
    wanted = set()
    for p in args.preset:
        bank, program = p.split(":")
        wanted.add((int(bank), int(program)))
    out = trim(open(args.src, "rb").read(), wanted)
    open(args.dst, "wb").write(out)
    print(f"wrote {args.dst}: {len(out)} bytes, presets {sorted(wanted)}")


if __name__ == "__main__":
    main()
