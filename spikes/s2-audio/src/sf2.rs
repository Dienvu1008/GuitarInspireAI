//! Generates a minimal SoundFont 2 file in memory: one preset, one instrument,
//! one looped sine sample at exactly 440 Hz (root key 69). Avoids any SoundFont
//! license question in the spike and gives an exact pitch reference for A7.

const SAMPLE_RATE: u32 = 44_000; // 100 samples per 440 Hz period, so the loop is seamless
const LEN: u32 = 4_400;
const LOOP_START: u32 = 100;
const LOOP_END: u32 = 4_100;

fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = id.to_vec();
    v.extend((data.len() as u32).to_le_bytes());
    v.extend(data);
    if data.len() % 2 == 1 {
        v.push(0);
    }
    v
}

fn list(kind: &[u8; 4], chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut body = kind.to_vec();
    for c in chunks {
        body.extend(c);
    }
    chunk(b"LIST", &body)
}

fn name20(s: &str) -> [u8; 20] {
    let mut n = [0u8; 20];
    n[..s.len()].copy_from_slice(s.as_bytes());
    n
}

fn u16s(v: &[u16]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

pub fn build() -> Vec<u8> {
    let info = list(
        b"INFO",
        &[chunk(b"ifil", &u16s(&[2, 1])), chunk(b"isng", b"EMU8000\0"), chunk(b"INAM", b"Spike\0")],
    );

    let mut smpl = Vec::new();
    for i in 0..LEN {
        let phase = (i % 100) as f64 / 100.0 * std::f64::consts::TAU;
        smpl.extend(((phase.sin() * 16_000.0) as i16).to_le_bytes());
    }
    smpl.extend([0u8; 46 * 2]); // required zero guard after each sample
    let sdta = list(b"sdta", &[chunk(b"smpl", &smpl)]);

    let mut phdr = Vec::new();
    for (name, bag) in [("Sine", 0u16), ("EOP", 1)] {
        phdr.extend(name20(name));
        phdr.extend(u16s(&[0, 0, bag]));
        phdr.extend([0u8; 12]);
    }
    let pbag = u16s(&[0, 0, 1, 0]);
    let pmod = vec![0u8; 10];
    let pgen = u16s(&[41, 0, 0, 0]); // instrument 0, terminal

    let mut inst = Vec::new();
    for (name, bag) in [("Sine", 0u16), ("EOI", 1)] {
        inst.extend(name20(name));
        inst.extend(bag.to_le_bytes());
    }
    let ibag = u16s(&[0, 0, 3, 0]);
    let imod = vec![0u8; 10];
    let release: i16 = -2400; // 0.25 s release (timecents)
    let igen = u16s(&[38, release as u16, 54, 1, 53, 0, 0, 0]); // release, loop mode, sampleID, terminal

    let mut shdr = Vec::new();
    shdr.extend(name20("Sine"));
    for v in [0u32, LEN, LOOP_START, LOOP_END, SAMPLE_RATE] {
        shdr.extend(v.to_le_bytes());
    }
    shdr.extend([69u8, 0]); // original pitch A4, correction 0
    shdr.extend(u16s(&[0, 1])); // link, mono sample
    shdr.extend(name20("EOS"));
    shdr.extend([0u8; 26]);

    let pdta = list(
        b"pdta",
        &[
            chunk(b"phdr", &phdr),
            chunk(b"pbag", &pbag),
            chunk(b"pmod", &pmod),
            chunk(b"pgen", &pgen),
            chunk(b"inst", &inst),
            chunk(b"ibag", &ibag),
            chunk(b"imod", &imod),
            chunk(b"igen", &igen),
            chunk(b"shdr", &shdr),
        ],
    );

    let mut body = b"sfbk".to_vec();
    body.extend(info);
    body.extend(sdta);
    body.extend(pdta);
    chunk(b"RIFF", &body)
}
