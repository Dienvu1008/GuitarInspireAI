//! GP3, GP4 and GP5 (5.00) reader, one code path with the version as a parameter.
//! Layout facts from the format description (README section 3), corrected by byte
//! analysis of the fixtures; each correction is marked "(bytes)".

use crate::cursor::{Cursor, Res};
use crate::model::*;

pub fn read(c: &mut Cursor) -> Res<Song> {
    let mut song = Song::default();
    c.mark("version");
    song.version = c.byte_string_fixed(30)?;
    let v: u8 = match song.version.strip_prefix("FICHIER GUITAR PRO v") {
        Some(rest) if rest.starts_with('3') => 3,
        Some(rest) if rest.starts_with('4') => 4,
        Some(rest) if rest.starts_with("5.00") => 5,
        _ => return c.fail(format!("unsupported version {:?}", song.version)),
    };
    c.mark("info");
    let names: &[&str] = if v == 5 {
        &["title", "subtitle", "artist", "album", "words", "music", "copyright", "tab", "instructions"]
    } else {
        &["title", "subtitle", "artist", "album", "words", "copyright", "tab", "instructions"]
    };
    for name in names {
        let s = c.int_byte_string()?;
        if !s.is_empty() {
            song.info.push(((*name).into(), s));
        }
    }
    let lines = c.count(1000, "notice")?;
    for _ in 0..lines {
        let s = c.int_byte_string()?;
        if !s.is_empty() {
            song.info.push(("notice".into(), s));
        }
    }
    if v < 5 {
        c.bool()?; // triplet feel
    }
    if v >= 4 {
        c.mark("lyrics");
        c.i32()?;
        for _ in 0..5 {
            c.i32()?;
            c.int_string()?;
        }
    }
    if v == 5 {
        // (bytes) 5.00 has the page setup here and no RSE master effect.
        c.mark("page setup");
        c.skip(4 * 7 + 2)?;
        for _ in 0..10 {
            c.int_byte_string()?;
        }
        c.int_byte_string()?; // tempo name
    }
    c.mark("tempo");
    song.tempo = c.i32()?;
    song.key = c.i32()?;
    if v >= 4 {
        c.i8()?; // octave (bytes: a Byte in GP5 too)
    }
    c.mark("midi channels");
    c.skip(64 * 12)?;
    if v == 5 {
        c.skip(19 * 2)?; // directions
        c.i32()?; // master reverb
    }
    let measures = c.count(10_000, "measures")?;
    let tracks = c.count(127, "tracks")?;
    c.mark("measure headers");
    for m in 0..measures {
        if v == 5 && m > 0 {
            c.u8()?;
        }
        song.headers.push(header(c, v)?);
    }
    c.mark("tracks");
    if v == 5 {
        c.u8()?; // (bytes) one blank byte before the first track
    }
    for _ in 0..tracks {
        song.tracks.push(track(c, v)?);
    }
    if v == 5 {
        c.u8()?; // (bytes) and one after the last
    }
    c.mark("measures");
    song.measures = vec![Vec::new(); tracks];
    let voices = if v == 5 { 2 } else { 1 };
    for _ in 0..measures {
        for t in song.measures.iter_mut() {
            let mut m = Measure::default();
            for _ in 0..voices {
                let n = c.count(1000, "beats")?;
                let mut voice = Vec::with_capacity(n);
                for _ in 0..n {
                    voice.push(beat(c, v)?);
                }
                m.voices.push(voice);
            }
            if v == 5 {
                c.u8()?; // line break
            }
            t.push(m);
        }
    }
    if c.remaining() != 0 && std::env::var("S10_LENIENT").is_err() {
        return c.fail(format!("{} bytes left after the last measure", c.remaining()));
    }
    Ok(song)
}

fn header(c: &mut Cursor, v: u8) -> Res<Header> {
    let mut h = Header::default();
    let flags = c.u8()?;
    if flags & 0x01 != 0 {
        h.numerator = c.u8()?;
    }
    if flags & 0x02 != 0 {
        h.denominator = c.u8()?;
    }
    h.repeat_start = flags & 0x04 != 0;
    if flags & 0x08 != 0 {
        h.repeat_end = c.u8()?;
    }
    if v < 5 && flags & 0x10 != 0 {
        h.alternative = c.u8()?;
    }
    if flags & 0x20 != 0 {
        h.marker = Some(c.int_byte_string()?);
        c.skip(4)?; // color
    }
    if v == 5 && flags & 0x10 != 0 {
        h.alternative = c.u8()?; // (bytes) after the marker in GP5
    }
    if flags & 0x40 != 0 {
        h.key = Some((c.i8()?, c.i8()?));
    }
    h.double_bar = flags & 0x80 != 0;
    if v == 5 {
        if flags & 0x03 != 0 {
            c.skip(4)?; // beam groups
        }
        if flags & 0x10 == 0 {
            c.u8()?; // (bytes) blank when there is no alternate ending
        }
        h.triplet_feel = c.u8()?;
    }
    Ok(h)
}

fn track(c: &mut Cursor, v: u8) -> Res<Track> {
    let mut t = Track::default();
    let flags = c.u8()?;
    t.percussion = flags & 0x01 != 0;
    t.name = c.byte_string_fixed(40)?;
    let strings = c.count(7, "strings")?;
    for i in 0..7 {
        let p = c.i32()?;
        if i < strings {
            t.tuning.push(p);
        }
    }
    c.i32()?; // port
    t.channel = c.i32()?;
    c.i32()?; // effect channel
    t.frets = c.i32()?;
    t.capo = c.i32()?;
    c.skip(4)?; // color
    if v == 5 {
        c.i16()?; // display flags
        c.skip(3)?; // auto accentuation, bank, humanize
        c.skip(3 * 4 + 12)?; // clef transposes, unknown
        c.skip(4 * 4)?; // RSE instrument
    }
    Ok(t)
}

fn beat(c: &mut Cursor, v: u8) -> Res<Beat> {
    let mut b = Beat {
        status: 1,
        ..Beat::default()
    };
    let flags = c.u8()?;
    if flags & 0x40 != 0 {
        b.status = c.u8()?;
    }
    b.duration = c.i8()?;
    b.dotted = flags & 0x01 != 0;
    if flags & 0x20 != 0 {
        b.tuplet = Some(c.i32()?);
    }
    if flags & 0x02 != 0 {
        return c.fail("chord diagram: not handled by the spike");
    }
    if flags & 0x04 != 0 {
        b.text = Some(c.int_byte_string()?);
    }
    if flags & 0x08 != 0 {
        beat_effects(c, v, &mut b)?;
    }
    if flags & 0x10 != 0 {
        mix_table(c, v, &mut b)?;
    }
    let strings = c.u8()?;
    for s in 0..7u8 {
        if strings & (0x40 >> s) != 0 {
            b.notes.push(note(c, v, s + 1)?);
        }
    }
    if v == 5 {
        let flags2 = c.i16()?;
        if flags2 & 0x0800 != 0 {
            c.u8()?;
        }
    }
    Ok(b)
}

fn beat_effects(c: &mut Cursor, v: u8, b: &mut Beat) -> Res<()> {
    let f1 = c.u8()?;
    let f2 = if v >= 4 { c.u8()? } else { 0 };
    if v == 3 {
        for (bit, name) in [
            (0x01, "vibrato"),
            (0x02, "wide vibrato"),
            (0x04, "natural harmonic"),
            (0x08, "artificial harmonic"),
            (0x10, "fade in"),
        ] {
            if f1 & bit != 0 {
                b.unsupported.push(name.into());
            }
        }
        if f1 & 0x20 != 0 {
            let kind = c.u8()?;
            // (bytes) an Int follows for every kind, not only for the tremolo bar
            // (f12.gp3: a tap is followed by 4 more bytes before the strings).
            c.i32()?;
            if kind == 0 {
                b.unsupported.push("tremolo bar".into());
            } else {
                b.unsupported.push(format!("slap/tap {kind}"));
            }
        }
    } else {
        for (bit, name) in [(0x02, "wide vibrato"), (0x10, "fade in")] {
            if f1 & bit != 0 {
                b.unsupported.push(name.into());
            }
        }
        if f2 & 0x01 != 0 {
            b.unsupported.push("rasgueado".into());
        }
        if f1 & 0x20 != 0 {
            let kind = c.i8()?;
            b.unsupported.push(format!("slap/tap {kind}"));
        }
        if f2 & 0x04 != 0 {
            bend(c)?;
            b.unsupported.push("tremolo bar".into());
        }
    }
    if f1 & 0x40 != 0 {
        b.stroke = (c.u8()?, c.u8()?);
    }
    if f2 & 0x02 != 0 {
        b.pick_stroke = c.i8()?;
    }
    Ok(())
}

fn bend(c: &mut Cursor) -> Res<()> {
    c.i8()?;
    c.i32()?;
    let n = c.count(100, "bend points")?;
    c.skip(n * 9)?;
    Ok(())
}

fn mix_table(c: &mut Cursor, v: u8, b: &mut Beat) -> Res<()> {
    let instrument = c.i8()?;
    if v == 5 {
        c.skip(16)?; // RSE instrument
    }
    let mut values = [0i8; 6];
    for x in values.iter_mut() {
        *x = c.i8()?;
    }
    if v == 5 {
        c.int_byte_string()?; // tempo name
    }
    let tempo = c.i32()?;
    for x in values {
        if x != -1 {
            c.i8()?;
        }
    }
    if tempo != -1 {
        c.i8()?; // duration of the tempo change
        b.tempo_change = Some(tempo);
    }
    if v >= 4 {
        c.u8()?; // "all tracks" flags
    }
    if v == 5 {
        c.i8()?; // wah
    }
    if instrument != -1 || values.iter().any(|x| *x != -1) {
        b.unsupported.push("mix table change".into());
    }
    Ok(())
}

fn note(c: &mut Cursor, v: u8, string: u8) -> Res<Note> {
    let mut n = Note {
        string,
        ..Note::default()
    };
    let flags = c.u8()?;
    n.heavy_accent = flags & 0x02 != 0;
    n.ghost = flags & 0x04 != 0;
    n.accent = flags & 0x40 != 0;
    if flags & 0x20 != 0 {
        match c.u8()? {
            1 => {}
            2 => n.tie = true,
            3 => n.dead = true,
            k => return c.fail(format!("note type {k}")),
        }
    }
    if v < 5 && flags & 0x01 != 0 {
        c.skip(2)?; // time-independent duration
        n.unsupported.push("time-independent duration".into());
    }
    if flags & 0x10 != 0 {
        n.velocity = Some(c.i8()?);
    }
    if flags & 0x20 != 0 {
        n.fret = c.i8()?;
    }
    if flags & 0x80 != 0 {
        c.skip(2)?;
        n.unsupported.push("fingering".into());
    }
    if v == 5 {
        if flags & 0x01 != 0 {
            c.f64()?;
        }
        c.u8()?; // second flags
    }
    if flags & 0x08 != 0 {
        note_effects(c, v, &mut n)?;
    }
    Ok(n)
}

fn note_effects(c: &mut Cursor, v: u8, n: &mut Note) -> Res<()> {
    let f1 = c.u8()?;
    let f2 = if v >= 4 { c.u8()? } else { 0 };
    if f1 & 0x01 != 0 {
        bend(c)?;
        n.unsupported.push("bend".into());
    }
    if f1 & 0x02 != 0 {
        n.unsupported.push("hammer-on/pull-off".into());
    }
    if v == 3 && f1 & 0x04 != 0 {
        n.unsupported.push("slide".into());
    }
    n.let_ring = f1 & 0x08 != 0;
    if f1 & 0x10 != 0 {
        c.skip(if v == 5 { 5 } else { 4 })?;
        n.unsupported.push("grace note".into());
    }
    if v == 3 {
        return Ok(());
    }
    n.staccato = f2 & 0x01 != 0;
    n.palm_mute = f2 & 0x02 != 0;
    if f2 & 0x04 != 0 {
        c.i8()?;
        n.unsupported.push("tremolo picking".into());
    }
    if f2 & 0x08 != 0 {
        c.u8()?;
        n.unsupported.push("slide".into());
    }
    if f2 & 0x10 != 0 {
        let kind = c.u8()?;
        if v == 5 {
            match kind {
                2 => c.skip(3)?,
                3 => c.skip(1)?,
                _ => {}
            }
        }
        n.unsupported.push(format!("harmonic {kind}"));
    }
    if f2 & 0x20 != 0 {
        c.skip(2)?;
        n.unsupported.push("trill".into());
    }
    if f2 & 0x40 != 0 {
        n.unsupported.push("vibrato".into());
    }
    Ok(())
}
