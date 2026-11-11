//! `RawSong` to GP3, GP4 or GP5 (5.00) bytes: the inverse of `read`. Sections the raw
//! model does not hold are written with the values TuxGuitar 2.0.1 writes for a new
//! song (page setup, MIDI channel table, directions, track display data), taken from
//! the bytes of the S10 fixtures.

use crate::bytes::Writer;
use crate::raw::*;

/// Default dynamics byte of a note (TuxGuitar's default velocity, S10 f02).
pub(crate) const DEFAULT_VELOCITY: i8 = 6;

pub(crate) fn write(song: &RawSong, v: GpVersion) -> Vec<u8> {
    let mut o = Writer::default();
    o.byte_string_fixed(v.header(), 30);
    let fields = if v == GpVersion::Gp5 { 9 } else { 8 };
    for i in 0..fields {
        o.int_byte_string(song.info.get(i).map(String::as_str).unwrap_or(""));
    }
    o.i32(song.notice.len() as i32);
    for n in &song.notice {
        o.int_byte_string(n);
    }
    if v != GpVersion::Gp5 {
        o.bool(song.triplet_feel);
    }
    if v != GpVersion::Gp3 {
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
    }
    if v == GpVersion::Gp5 {
        page_setup(&mut o);
    }
    o.i32(song.tempo);
    o.i32(song.key);
    if v != GpVersion::Gp3 {
        o.i8(0); // octave
    }
    channels(&mut o, song);
    if v == GpVersion::Gp5 {
        for _ in 0..19 {
            o.i16(-1); // directions: none
        }
        o.i32(-1); // master reverb
    }
    o.i32(song.headers.len() as i32);
    o.i32(song.tracks.len() as i32);
    for (m, h) in song.headers.iter().enumerate() {
        if v == GpVersion::Gp5 && m > 0 {
            o.u8(0);
        }
        header(&mut o, h, v);
    }
    if v == GpVersion::Gp5 {
        o.u8(0);
    }
    for t in &song.tracks {
        track(&mut o, t, v);
    }
    if v == GpVersion::Gp5 {
        o.u8(0);
    }
    for row in &song.bars {
        for bar in row {
            for i in 0..v.voices() {
                let voice = bar.voices.get(i).map(Vec::as_slice).unwrap_or(&[]);
                o.i32(voice.len() as i32);
                for b in voice {
                    beat(&mut o, b, v);
                }
            }
            if v == GpVersion::Gp5 {
                o.u8(0); // line break
            }
        }
    }
    o.out
}

/// A4 page setup and the header and footer templates, as TuxGuitar writes them.
fn page_setup(o: &mut Writer) {
    for x in [210, 297, 10, 10, 15, 10, 100] {
        o.i32(x);
    }
    o.i16(0x01ff);
    for t in [
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
        o.int_byte_string(t);
    }
    o.int_byte_string("Moderate"); // tempo name
}

/// The 64-entry MIDI channel table (4 ports x 16). Channels used by a track get a
/// steel-string guitar at full volume and centred; the others TuxGuitar's defaults.
fn channels(o: &mut Writer, song: &RawSong) {
    let used = |ch: i32| {
        song.tracks
            .iter()
            .any(|t| t.channel == ch || t.effect_channel == ch)
    };
    for i in 0..64 {
        let ch = i % 16 + 1;
        if i < 16 && used(ch) {
            o.i32(25);
            o.bytes(&[16, 8]);
        } else {
            o.i32(24);
            o.bytes(&[1, 1]);
        }
        o.bytes(&[0, 0, 0, 0, 0, 0]);
    }
}

fn header(o: &mut Writer, h: &RawHeader, v: GpVersion) {
    let mut flags = 0u8;
    for (on, bit) in [
        (h.numerator.is_some(), 0x01),
        (h.denominator.is_some(), 0x02),
        (h.repeat_start, 0x04),
        (h.repeat_close.is_some(), 0x08),
        (h.alternative != 0, 0x10),
        (h.marker.is_some(), 0x20),
        (h.key.is_some(), 0x40),
        (h.double_bar, 0x80),
    ] {
        if on {
            flags |= bit;
        }
    }
    o.u8(flags);
    if let Some(n) = h.numerator {
        o.u8(n);
    }
    if let Some(d) = h.denominator {
        o.u8(d);
    }
    if let Some(r) = h.repeat_close {
        o.u8(r);
    }
    if v != GpVersion::Gp5 && h.alternative != 0 {
        o.u8(h.alternative);
    }
    if let Some((name, colour)) = &h.marker {
        o.int_byte_string(name);
        o.bytes(colour);
    }
    if v == GpVersion::Gp5 && h.alternative != 0 {
        o.u8(h.alternative);
    }
    if let Some((k, minor)) = h.key {
        o.i8(k);
        o.i8(minor);
    }
    if v == GpVersion::Gp5 {
        if flags & 0x03 != 0 {
            let n = h.numerator.unwrap_or(4);
            let d = h.denominator.unwrap_or(4);
            let beams = if n == 4 && d == 4 {
                [2, 2, 2, 2]
            } else {
                [n.clamp(1, 4), 1, 1, 1]
            };
            o.bytes(&beams);
        }
        if h.alternative == 0 {
            o.u8(0);
        }
        o.u8(h.triplet_feel);
    }
}

fn track(o: &mut Writer, t: &RawTrack, v: GpVersion) {
    let flags = if v == GpVersion::Gp5 { 0x08 } else { 0x00 };
    o.u8(flags | t.percussion as u8);
    o.byte_string_fixed(&t.name, 40);
    o.i32(t.tuning.len() as i32);
    for i in 0..7 {
        o.i32(t.tuning.get(i).copied().unwrap_or(0));
    }
    o.i32(t.port);
    o.i32(t.channel);
    o.i32(t.effect_channel);
    o.i32(t.frets);
    o.i32(t.capo);
    o.bytes(&[255, 0, 0, 0]); // colour
    if v == GpVersion::Gp5 {
        o.i16(0x0143);
        o.bytes(&[0, 0, 0]);
        for x in [0, 0, 100] {
            o.i32(x);
        }
        o.bytes(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 255, 3]);
        for x in [-1, -1, -1, 0x00ff_ffff] {
            o.i32(x);
        }
    }
}

fn beat(o: &mut Writer, b: &RawBeat, v: GpVersion) {
    let brush = b.brush != (0, 0);
    let pick = b.pick_stroke != 0 && v != GpVersion::Gp3;
    let effects = brush || pick;
    let mix = b.tempo.is_some();
    let mut flags = 0u8;
    for (on, bit) in [
        (b.dotted, 0x01),
        (b.chord.is_some() && v != GpVersion::Gp3, 0x02),
        (b.text.is_some(), 0x04),
        (effects, 0x08),
        (mix, 0x10),
        (b.tuplet.is_some(), 0x20),
        (b.status != BeatStatus::Normal, 0x40),
    ] {
        if on {
            flags |= bit;
        }
    }
    o.u8(flags);
    match b.status {
        BeatStatus::Normal => {}
        BeatStatus::Empty => o.u8(0),
        BeatStatus::Rest => o.u8(2),
    }
    o.i8(b.duration);
    if let Some(t) = b.tuplet {
        o.i32(t);
    }
    if let Some(ch) = b.chord.as_ref().filter(|_| v != GpVersion::Gp3) {
        chord(o, ch);
    }
    if let Some(t) = &b.text {
        o.int_byte_string(t);
    }
    if effects {
        o.u8(if brush { 0x40 } else { 0 });
        if v != GpVersion::Gp3 {
            o.u8(if pick { 0x02 } else { 0 });
        }
        if brush {
            let (up, down) = b.brush;
            if v == GpVersion::Gp5 {
                o.u8(down);
                o.u8(up);
            } else {
                o.u8(up);
                o.u8(down);
            }
        }
        if pick {
            o.i8(b.pick_stroke);
        }
    }
    if let Some(tempo) = b.tempo {
        o.i8(-1); // instrument
        if v == GpVersion::Gp5 {
            for _ in 0..4 {
                o.i32(-1); // RSE instrument
            }
        }
        for _ in 0..6 {
            o.i8(-1);
        }
        if v == GpVersion::Gp5 {
            o.int_byte_string(""); // tempo name
        }
        o.i32(tempo);
        o.i8(0); // transition duration
        if v != GpVersion::Gp3 {
            o.u8(0); // "all tracks" flags
        }
        if v == GpVersion::Gp5 {
            o.i8(-1); // wah
        }
    }
    let mut strings = 0u8;
    for n in &b.notes {
        strings |= 0x40 >> (n.string - 1);
    }
    o.u8(strings);
    let mut notes: Vec<&RawNote> = b.notes.iter().collect();
    notes.sort_by_key(|n| n.string);
    for n in notes {
        note(o, n, v);
    }
    if v == GpVersion::Gp5 {
        o.i16(0);
    }
}

/// A chord diagram in the new layout (GP4 and GP5 only; never written to GP3).
fn chord(o: &mut Writer, ch: &RawChord) {
    o.u8(1);
    o.u8(1); // sharp
    o.bytes(&[0, 0, 0]);
    o.bytes(&[12, 0, 0]); // root, type, extension
    o.i32(-1); // bass
    o.i32(0); // tonality
    o.u8(0); // add
    o.byte_string_fixed(&ch.name, 22);
    o.bytes(&[0, 0, 0]);
    o.i32(ch.first_fret);
    for i in 0..7 {
        o.i32(ch.frets.get(i).copied().unwrap_or(-1));
    }
    o.u8(0);
    o.bytes(&[0; 15]);
    o.bytes(&[0; 8]);
    o.bytes(&[0; 8]);
}

fn note(o: &mut Writer, n: &RawNote, v: GpVersion) {
    let gp3 = v == GpVersion::Gp3;
    let accent = n.accent && !gp3;
    let heavy = n.heavy_accent && !gp3;
    let palm = n.palm_mute && !gp3;
    let stacc = n.staccato && !gp3;
    let fx = n.let_ring || palm || stacc;
    let mut flags = 0x20u8;
    for (on, bit) in [
        (heavy, 0x02),
        (n.ghost, 0x04),
        (fx, 0x08),
        (n.velocity.is_some(), 0x10),
        (accent, 0x40),
    ] {
        if on {
            flags |= bit;
        }
    }
    o.u8(flags);
    o.u8(match n.kind {
        NoteType::Normal => 1,
        NoteType::Tie => 2,
        NoteType::Dead => 3,
    });
    if let Some(vel) = n.velocity {
        o.i8(vel);
    }
    o.i8(n.fret);
    if v == GpVersion::Gp5 {
        o.u8(0); // second flags
    }
    if fx {
        o.u8(if n.let_ring { 0x08 } else { 0 });
        if !gp3 {
            o.u8(stacc as u8 | (palm as u8) << 1);
        }
    }
}
