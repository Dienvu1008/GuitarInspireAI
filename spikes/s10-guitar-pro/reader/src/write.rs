//! GP5 (5.00) writer of the spike model: the inverse of `gp::read` for version 5.
//! Sections the model does not hold are written with the values TuxGuitar writes
//! (page setup, MIDI channels, directions, track display data).

use crate::model::*;

#[derive(Default)]
struct Out {
    b: Vec<u8>,
}

impl Out {
    fn u8(&mut self, v: u8) {
        self.b.push(v);
    }
    fn i8(&mut self, v: i8) {
        self.b.push(v as u8);
    }
    fn i16(&mut self, v: i16) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }
    fn bytes(&mut self, v: &[u8]) {
        self.b.extend_from_slice(v);
    }
    /// ByteSizeString in a fixed buffer.
    fn fixed(&mut self, s: &str, size: usize) {
        let t = &s.as_bytes()[..s.len().min(size)];
        self.u8(t.len() as u8);
        self.bytes(t);
        self.bytes(&vec![0; size - t.len()]);
    }
    fn int_string(&mut self, s: &str) {
        self.i32(s.len() as i32);
        self.bytes(s.as_bytes());
    }
    fn int_byte_string(&mut self, s: &str) {
        let t = &s.as_bytes()[..s.len().min(255)];
        self.i32(t.len() as i32 + 1);
        self.u8(t.len() as u8);
        self.bytes(t);
    }
}

const INFO: [&str; 9] = [
    "title",
    "subtitle",
    "artist",
    "album",
    "words",
    "music",
    "copyright",
    "tab",
    "instructions",
];

pub fn write(song: &Song) -> Vec<u8> {
    let mut o = Out::default();
    o.fixed("FICHIER GUITAR PRO v5.00", 30);
    for name in INFO {
        let v = song
            .info
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        o.int_byte_string(v);
    }
    let notice: Vec<&str> = song
        .info
        .iter()
        .filter(|(k, _)| k == "notice")
        .map(|(_, v)| v.as_str())
        .collect();
    o.i32(notice.len() as i32);
    for n in notice {
        o.int_byte_string(n);
    }
    o.i32(song.lyrics_track);
    for i in 0..5 {
        let (bar, text) = song
            .lyrics
            .get(i)
            .map(|(b, t)| (*b, t.as_str()))
            .unwrap_or((0, ""));
        o.i32(bar);
        o.int_string(text);
    }
    // Page setup as TuxGuitar writes it (A4).
    for v in [210, 297, 10, 10, 15, 10, 100] {
        o.i32(v);
    }
    o.i16(0x01ff);
    for p in [
        "%TITLE%",
        "%SUBTITLE%",
        "%ARTIST%",
        "%ALBUM%",
        "Words by %WORDS%",
        "Music by %MUSIC%",
        "Words & Music by %WORDSMUSIC%",
        "Copyright %COPYRIGHT%",
        "All Rights Reserved - International Copyright Secured",
        "Page %N%/%P%",
    ] {
        o.int_byte_string(p);
    }
    o.int_byte_string("Moderate");
    o.i32(song.tempo);
    o.i32(song.key);
    o.i8(0); // octave
    for _ in 0..64 {
        o.i32(25);
        o.bytes(&[13, 8, 0, 0, 0, 0, 0, 0]);
    }
    for _ in 0..19 {
        o.i16(-1); // directions: none
    }
    o.i32(-1); // master reverb
    o.i32(song.headers.len() as i32);
    o.i32(song.tracks.len() as i32);
    for (m, h) in song.headers.iter().enumerate() {
        if m > 0 {
            o.u8(0);
        }
        header(&mut o, h);
    }
    o.u8(0);
    for t in &song.tracks {
        track(&mut o, t);
    }
    o.u8(0);
    for m in 0..song.headers.len() {
        for t in &song.measures {
            let measure = &t[m];
            for v in 0..2 {
                let empty = Vec::new();
                let beats = measure.voices.get(v).unwrap_or(&empty);
                o.i32(beats.len() as i32);
                for b in beats {
                    beat(&mut o, b);
                }
            }
            o.u8(0); // line break
        }
    }
    o.b
}

fn header(o: &mut Out, h: &Header) {
    let mut flags = 0u8;
    if h.numerator > 0 {
        flags |= 0x01;
    }
    if h.denominator > 0 {
        flags |= 0x02;
    }
    if h.repeat_start {
        flags |= 0x04;
    }
    if h.repeat_end > 0 {
        flags |= 0x08;
    }
    if h.alternative > 0 {
        flags |= 0x10;
    }
    if h.marker.is_some() {
        flags |= 0x20;
    }
    if h.key.is_some() {
        flags |= 0x40;
    }
    if h.double_bar {
        flags |= 0x80;
    }
    o.u8(flags);
    if h.numerator > 0 {
        o.u8(h.numerator);
    }
    if h.denominator > 0 {
        o.u8(h.denominator);
    }
    if h.repeat_end > 0 {
        o.u8(h.repeat_end);
    }
    if let Some(m) = &h.marker {
        o.int_byte_string(m);
        o.bytes(&[255, 0, 0, 0]);
    }
    if h.alternative > 0 {
        o.u8(h.alternative);
    }
    if let Some((k, t)) = h.key {
        o.i8(k);
        o.i8(t);
    }
    if flags & 0x03 != 0 {
        let beams = if h.numerator == 4 && h.denominator == 4 {
            [2, 2, 2, 2]
        } else {
            [h.numerator.clamp(1, 4), 1, 1, 1]
        };
        o.bytes(&beams);
    }
    if h.alternative == 0 {
        o.u8(0);
    }
    o.u8(h.triplet_feel);
}

fn track(o: &mut Out, t: &Track) {
    o.u8(if t.percussion { 0x09 } else { 0x08 });
    o.fixed(&t.name, 40);
    o.i32(t.tuning.len() as i32);
    for i in 0..7 {
        o.i32(t.tuning.get(i).copied().unwrap_or(0));
    }
    o.i32(1); // port
    o.i32(t.channel);
    o.i32(t.channel + 1); // effect channel
    o.i32(t.frets);
    o.i32(t.capo);
    o.bytes(&[255, 0, 0, 0]); // color
    o.i16(0x0143);
    o.bytes(&[0, 0, 0]);
    for v in [0, 0, 100] {
        o.i32(v);
    }
    o.bytes(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 255, 3]);
    for v in [-1, -1, -1, 0x00ff_ffff] {
        o.i32(v);
    }
}

fn beat(o: &mut Out, b: &Beat) {
    let effects = b.stroke != (0, 0) || b.pick_stroke != 0;
    let mut flags = 0u8;
    if b.dotted {
        flags |= 0x01;
    }
    if b.chord.is_some() {
        flags |= 0x02;
    }
    if b.text.is_some() {
        flags |= 0x04;
    }
    if effects {
        flags |= 0x08;
    }
    if b.tempo_change.is_some() {
        flags |= 0x10;
    }
    if b.tuplet.is_some() {
        flags |= 0x20;
    }
    if b.status != 1 {
        flags |= 0x40;
    }
    o.u8(flags);
    if b.status != 1 {
        o.u8(b.status);
    }
    o.i8(b.duration);
    if let Some(t) = b.tuplet {
        o.i32(t);
    }
    if let Some(ch) = &b.chord {
        chord(o, ch);
    }
    if let Some(t) = &b.text {
        o.int_byte_string(t);
    }
    if effects {
        let f1 = if b.stroke != (0, 0) { 0x40 } else { 0 };
        let f2 = if b.pick_stroke != 0 { 0x02 } else { 0 };
        o.u8(f1);
        o.u8(f2);
        if f1 != 0 {
            o.u8(b.stroke.0);
            o.u8(b.stroke.1);
        }
        if f2 != 0 {
            o.i8(b.pick_stroke);
        }
    }
    if let Some(tempo) = b.tempo_change {
        o.i8(-1); // instrument
        for v in [-1, -1, -1, -1] {
            o.i32(v); // RSE instrument
        }
        for _ in 0..6 {
            o.i8(-1);
        }
        o.int_byte_string("");
        o.i32(tempo);
        o.i8(0); // tempo change duration
        o.u8(0); // flags
        o.i8(-1); // wah
    }
    let mut strings = 0u8;
    for n in &b.notes {
        strings |= 0x40 >> (n.string - 1);
    }
    o.u8(strings);
    let mut notes: Vec<&Note> = b.notes.iter().collect();
    notes.sort_by_key(|n| n.string);
    for n in notes {
        note(o, n);
    }
    o.i16(0);
}

/// New-layout chord diagram (107 bytes), the fields we do not keep set as
/// TuxGuitar's GP5 writer sets them for a named chord.
fn chord(o: &mut Out, ch: &Chord) {
    o.u8(1); // new layout
    o.u8(1); // sharp
    o.bytes(&[0, 0, 0]);
    o.bytes(&[12, 0, 0]); // root, type, extension
    o.i32(-1); // bass
    o.i32(0); // tonality
    o.u8(0); // add
    o.fixed(&ch.name, 22);
    o.bytes(&[0, 0, 0]); // alterations
    o.i32(ch.first_fret);
    for i in 0..7 {
        o.i32(ch.frets.get(i).copied().unwrap_or(-1));
    }
    o.u8(0); // no barres
    o.bytes(&[0; 15]);
    o.bytes(&[0; 8]); // omissions, blank
    o.bytes(&[0; 8]); // fingering, show fingering
}

fn note(o: &mut Out, n: &Note) {
    let fx = n.palm_mute || n.let_ring || n.staccato;
    let mut flags = 0x20u8;
    if n.heavy_accent {
        flags |= 0x02;
    }
    if n.ghost {
        flags |= 0x04;
    }
    if fx {
        flags |= 0x08;
    }
    if n.velocity.is_some() {
        flags |= 0x10;
    }
    if n.accent {
        flags |= 0x40;
    }
    o.u8(flags);
    o.u8(if n.tie {
        2
    } else if n.dead {
        3
    } else {
        1
    });
    if let Some(v) = n.velocity {
        o.i8(v);
    }
    o.i8(n.fret);
    o.u8(0); // second flags
    if fx {
        o.u8(if n.let_ring { 0x08 } else { 0 });
        o.u8((if n.staccato { 0x01 } else { 0 }) | (if n.palm_mute { 0x02 } else { 0 }));
    }
}
