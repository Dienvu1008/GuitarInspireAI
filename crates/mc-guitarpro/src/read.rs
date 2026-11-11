//! GP3, GP4 and GP5 (5.00) reader into `RawSong`. One code path with the version as a
//! parameter. The layout follows the format description (ADR-036 section 2), corrected
//! by byte analysis of the S10 fixtures; each correction is marked "(bytes)" and
//! listed in the S10 README, section 6.2.

use crate::Feature;
use crate::bytes::{ReadError, Reader, Res};
use crate::raw::*;

pub(crate) const MAX_BARS: usize = 10_000;
pub(crate) const MAX_TRACKS: usize = 127;
pub(crate) const MAX_STRINGS: usize = 7;
pub(crate) const MAX_BEATS: usize = 1_000;
pub(crate) const MAX_BEND_POINTS: usize = 100;

/// Why a file could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    Version(String),
    Layout(ReadError),
}

impl From<ReadError> for Failure {
    fn from(e: ReadError) -> Self {
        Failure::Layout(e)
    }
}

/// Recognise the version from the first bytes.
pub(crate) fn version(data: &[u8]) -> Result<(GpVersion, String), Failure> {
    if data.starts_with(b"PK") {
        return Err(Failure::Version(
            "a ZIP archive (Guitar Pro 7 or later)".into(),
        ));
    }
    if data.starts_with(b"BCFZ") || data.starts_with(b"BCFS") {
        return Err(Failure::Version("a GPX file (Guitar Pro 6)".into()));
    }
    let not_gp = || Failure::Version("not a Guitar Pro file".into());
    let text = Reader::new(data)
        .byte_string_fixed(30)
        .map_err(|_| not_gp())?;
    let v = match text.strip_prefix("FICHIER GUITAR PRO v") {
        Some(rest) if rest.starts_with("3.") => GpVersion::Gp3,
        Some(rest) if rest.starts_with("4.") => GpVersion::Gp4,
        Some("5.00") => GpVersion::Gp5,
        Some(_) => return Err(Failure::Version(text)),
        None => return Err(not_gp()),
    };
    Ok((v, text))
}

pub(crate) fn read(data: &[u8]) -> Result<(GpVersion, RawSong), Failure> {
    let (v, _) = version(data)?;
    let mut c = Reader::new(data);
    c.skip(31)?;
    let song = song(&mut c, v)?;
    if c.remaining() != 0 {
        return Err(Failure::Layout(ReadError {
            at: c.pos(),
            what: format!("{} bytes after the last measure", c.remaining()),
        }));
    }
    Ok((v, song))
}

fn song(c: &mut Reader, v: GpVersion) -> Res<RawSong> {
    let mut s = RawSong::default();
    let fields = if v == GpVersion::Gp5 { 9 } else { 8 };
    for _ in 0..fields {
        s.info.push(c.int_byte_string()?);
    }
    let lines = c.count(1_000, "notice lines")?;
    for _ in 0..lines {
        s.notice.push(c.int_byte_string()?);
    }
    if v != GpVersion::Gp5 {
        s.triplet_feel = c.bool()?;
    }
    if v != GpVersion::Gp3 {
        s.lyrics_track = c.i32()?;
        for _ in 0..5 {
            let bar = c.i32()?;
            let text = c.int_string()?;
            s.lyrics.push((bar, text));
        }
    }
    if v == GpVersion::Gp5 {
        // (bytes) 5.00 has the page setup here and no RSE master effect.
        c.skip(4 * 7 + 2)?;
        for _ in 0..10 {
            c.int_byte_string()?;
        }
        c.int_byte_string()?; // tempo name
    }
    s.tempo = c.i32()?;
    s.key = c.i32()?;
    if v != GpVersion::Gp3 {
        c.i8()?; // octave (bytes: a Byte in GP5 too)
    }
    c.skip(64 * 12)?; // MIDI channels
    if v == GpVersion::Gp5 {
        c.skip(19 * 2)?; // directions
        c.i32()?; // master reverb
    }
    let measures = c.count(MAX_BARS, "bars")?;
    let tracks = c.count(MAX_TRACKS, "tracks")?;
    for m in 0..measures {
        if v == GpVersion::Gp5 && m > 0 {
            c.u8()?;
        }
        s.headers.push(header(c, v)?);
    }
    if v == GpVersion::Gp5 {
        c.u8()?; // (bytes) one blank byte before the first track
    }
    for _ in 0..tracks {
        s.tracks.push(track(c, v)?);
    }
    if v == GpVersion::Gp5 {
        c.u8()?; // (bytes) and one after the last
    }
    for _ in 0..measures {
        let mut row = Vec::with_capacity(tracks);
        for _ in 0..tracks {
            let mut bar = RawBar::default();
            for _ in 0..v.voices() {
                let n = c.count(MAX_BEATS, "beats in a voice")?;
                let mut voice = Vec::with_capacity(n.min(c.remaining()));
                for _ in 0..n {
                    voice.push(beat(c, v)?);
                }
                bar.voices.push(voice);
            }
            if v == GpVersion::Gp5 {
                c.u8()?; // line break
            }
            row.push(bar);
        }
        s.bars.push(row);
    }
    Ok(s)
}

fn header(c: &mut Reader, v: GpVersion) -> Res<RawHeader> {
    let mut h = RawHeader::default();
    let flags = c.u8()?;
    if flags & 0x01 != 0 {
        h.numerator = Some(c.u8()?);
    }
    if flags & 0x02 != 0 {
        h.denominator = Some(c.u8()?);
    }
    h.repeat_start = flags & 0x04 != 0;
    if flags & 0x08 != 0 {
        h.repeat_close = Some(c.u8()?);
    }
    if v != GpVersion::Gp5 && flags & 0x10 != 0 {
        h.alternative = c.u8()?;
    }
    if flags & 0x20 != 0 {
        let name = c.int_byte_string()?;
        let colour = c.bytes(4)?;
        h.marker = Some((name, [colour[0], colour[1], colour[2], colour[3]]));
    }
    if v == GpVersion::Gp5 && flags & 0x10 != 0 {
        h.alternative = c.u8()?; // (bytes) after the marker in GP5
    }
    if flags & 0x40 != 0 {
        h.key = Some((c.i8()?, c.i8()?));
    }
    h.double_bar = flags & 0x80 != 0;
    if v == GpVersion::Gp5 {
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

fn track(c: &mut Reader, v: GpVersion) -> Res<RawTrack> {
    let mut t = RawTrack::default();
    let flags = c.u8()?;
    t.percussion = flags & 0x01 != 0;
    t.name = c.byte_string_fixed(40)?;
    let at = c.pos();
    let strings = c.i32()?;
    if !(1..=MAX_STRINGS as i32).contains(&strings) {
        return Err(ReadError {
            at,
            what: format!("{strings} strings; a track has 1 to {MAX_STRINGS}"),
        });
    }
    for i in 0..MAX_STRINGS {
        let p = c.i32()?;
        if i < strings as usize {
            t.tuning.push(p);
        }
    }
    t.port = c.i32()?;
    t.channel = c.i32()?;
    t.effect_channel = c.i32()?;
    t.frets = c.i32()?;
    t.capo = c.i32()?;
    c.skip(4)?; // colour
    if v == GpVersion::Gp5 {
        c.i16()?; // display flags
        c.skip(3)?; // auto accentuation, bank, humanize
        c.skip(3 * 4 + 12)?; // clef transposes and unknown bytes
        c.skip(4 * 4)?; // RSE instrument
    }
    Ok(t)
}

fn beat(c: &mut Reader, v: GpVersion) -> Res<RawBeat> {
    let mut b = RawBeat::default();
    let flags = c.u8()?;
    if flags & 0x40 != 0 {
        b.status = match c.u8()? {
            0 => BeatStatus::Empty,
            2 => BeatStatus::Rest,
            _ => BeatStatus::Normal,
        };
    }
    let at = c.pos();
    b.duration = c.i8()?;
    if !(-2..=4).contains(&b.duration) {
        return Err(ReadError {
            at,
            what: format!("duration {} is not whole to sixty-fourth", b.duration),
        });
    }
    b.dotted = flags & 0x01 != 0;
    if flags & 0x20 != 0 {
        b.tuplet = Some(c.i32()?);
    }
    if flags & 0x02 != 0 {
        b.chord = Some(chord(c, v)?);
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
    if v == GpVersion::Gp5 {
        let flags2 = c.i16()?;
        if flags2 & 0x0800 != 0 {
            c.u8()?;
        }
    }
    Ok(b)
}

/// Chord diagram. A header byte selects the old (0) or new (1) layout.
fn chord(c: &mut Reader, v: GpVersion) -> Res<RawChord> {
    let mut ch = RawChord::default();
    match c.u8()? {
        0 => {
            // Old layout, from the format description only; no fixture shows it
            // (ADR-036 choice 7.7).
            ch.name = c.int_byte_string()?;
            ch.first_fret = c.i32()?;
            for _ in 0..6 {
                ch.frets.push(c.i32()?);
            }
        }
        1 if v != GpVersion::Gp3 => {
            // (bytes) 107 bytes in GP4 and GP5 alike, header included.
            c.skip(1 + 3)?; // sharp, blank
            c.skip(3)?; // root, type, extension
            c.skip(4 + 4)?; // bass, tonality
            c.skip(1)?; // add
            ch.name = c.byte_string_fixed(22)?;
            c.skip(3)?; // fifth, ninth, eleventh
            ch.first_fret = c.i32()?;
            for _ in 0..7 {
                ch.frets.push(c.i32()?);
            }
            c.skip(1 + 5 * 3)?; // barre count, frets, starts, ends
            c.skip(7 + 1)?; // omissions, blank
            c.skip(7 + 1)?; // fingering, show fingering
        }
        1 => return c.fail("a GP3 chord diagram in the new layout, which is not verified"),
        h => return c.fail(format!("chord diagram header {h}")),
    }
    Ok(ch)
}

fn beat_effects(c: &mut Reader, v: GpVersion, b: &mut RawBeat) -> Res<()> {
    let f1 = c.u8()?;
    let f2 = if v == GpVersion::Gp3 { 0 } else { c.u8()? };
    let mut simple = vec![
        (0x01, Feature::Vibrato),
        (0x02, Feature::WideVibrato),
        (0x10, Feature::FadeIn),
    ];
    if v == GpVersion::Gp3 {
        simple.extend([(0x04, Feature::Harmonic), (0x08, Feature::Harmonic)]);
    }
    for (bit, f) in simple {
        if f1 & bit != 0 {
            b.dropped.push(f);
        }
    }
    if v == GpVersion::Gp3 {
        if f1 & 0x20 != 0 {
            let kind = c.u8()?;
            // (bytes) an Int follows for every kind, not only for the tremolo bar.
            c.i32()?;
            b.dropped.push(slap(kind));
        }
    } else {
        if f1 & 0x20 != 0 {
            let kind = c.u8()?;
            b.dropped.push(slap(kind));
        }
        if f2 & 0x04 != 0 {
            bend(c)?;
            b.dropped.push(Feature::TremoloBar);
        }
        if f2 & 0x01 != 0 {
            b.dropped.push(Feature::Rasgueado);
        }
    }
    if f1 & 0x40 != 0 {
        let first = c.u8()?;
        let second = c.u8()?;
        // The description orders the bytes (up, down) in GP3 and GP4, (down, up) in GP5.
        b.brush = if v == GpVersion::Gp5 {
            (second, first)
        } else {
            (first, second)
        };
    }
    if f2 & 0x02 != 0 {
        b.pick_stroke = c.i8()?;
    }
    Ok(())
}

fn slap(kind: u8) -> Feature {
    match kind {
        0 => Feature::TremoloBar,
        1 => Feature::Tap,
        2 => Feature::Slap,
        _ => Feature::Pop,
    }
}

fn bend(c: &mut Reader) -> Res<()> {
    c.i8()?; // type
    c.i32()?; // value
    let n = c.count(MAX_BEND_POINTS, "bend points")?;
    c.skip(n * 9)
}

fn mix_table(c: &mut Reader, v: GpVersion, b: &mut RawBeat) -> Res<()> {
    let instrument = c.i8()?;
    if v == GpVersion::Gp5 {
        c.skip(16)?; // RSE instrument
    }
    let mut values = [0i8; 6]; // volume, pan, chorus, reverb, phaser, tremolo
    for x in values.iter_mut() {
        *x = c.i8()?;
    }
    if v == GpVersion::Gp5 {
        c.int_byte_string()?; // tempo name
    }
    let tempo = c.i32()?;
    for x in values {
        if x != -1 {
            c.i8()?; // transition duration
        }
    }
    if tempo != -1 {
        c.i8()?; // transition duration
        b.tempo = Some(tempo);
    }
    if v != GpVersion::Gp3 {
        c.u8()?; // "all tracks" flags
    }
    if v == GpVersion::Gp5 {
        c.i8()?; // wah
    }
    b.mix_other = instrument != -1 || values.iter().any(|x| *x != -1);
    Ok(())
}

fn note(c: &mut Reader, v: GpVersion, string: u8) -> Res<RawNote> {
    let mut n = RawNote {
        string,
        ..RawNote::default()
    };
    let flags = c.u8()?;
    n.heavy_accent = flags & 0x02 != 0;
    n.ghost = flags & 0x04 != 0;
    n.accent = flags & 0x40 != 0;
    if flags & 0x20 != 0 {
        n.kind = match c.u8()? {
            2 => NoteType::Tie,
            3 => NoteType::Dead,
            _ => NoteType::Normal,
        };
    }
    if v != GpVersion::Gp5 && flags & 0x01 != 0 {
        c.skip(2)?;
        n.dropped.push(Feature::TimeIndependentDuration);
    }
    if flags & 0x10 != 0 {
        n.velocity = Some(c.i8()?);
    }
    if flags & 0x20 != 0 {
        n.fret = c.i8()?;
    }
    if flags & 0x80 != 0 {
        c.skip(2)?;
        n.dropped.push(Feature::Fingering);
    }
    if v == GpVersion::Gp5 {
        if flags & 0x01 != 0 {
            c.skip(8)?; // duration percent (a Double)
            n.dropped.push(Feature::TimeIndependentDuration);
        }
        c.u8()?; // second flags
    }
    if flags & 0x08 != 0 {
        note_effects(c, v, &mut n)?;
    }
    Ok(n)
}

fn note_effects(c: &mut Reader, v: GpVersion, n: &mut RawNote) -> Res<()> {
    let f1 = c.u8()?;
    let f2 = if v == GpVersion::Gp3 { 0 } else { c.u8()? };
    if f1 & 0x01 != 0 {
        bend(c)?;
        n.dropped.push(Feature::Bend);
    }
    if f1 & 0x02 != 0 {
        n.dropped.push(Feature::HammerOnPullOff);
    }
    if v == GpVersion::Gp3 && f1 & 0x04 != 0 {
        n.dropped.push(Feature::Slide);
    }
    n.let_ring = f1 & 0x08 != 0;
    if f1 & 0x10 != 0 {
        c.skip(if v == GpVersion::Gp5 { 5 } else { 4 })?;
        n.dropped.push(Feature::GraceNote);
    }
    if v == GpVersion::Gp3 {
        return Ok(());
    }
    n.staccato = f2 & 0x01 != 0;
    n.palm_mute = f2 & 0x02 != 0;
    if f2 & 0x04 != 0 {
        c.i8()?;
        n.dropped.push(Feature::TremoloPicking);
    }
    if f2 & 0x08 != 0 {
        c.u8()?;
        n.dropped.push(Feature::Slide);
    }
    if f2 & 0x10 != 0 {
        let kind = c.u8()?;
        if v == GpVersion::Gp5 {
            match kind {
                2 => c.skip(3)?,
                3 => c.skip(1)?,
                _ => {}
            }
        }
        n.dropped.push(Feature::Harmonic);
    }
    if f2 & 0x20 != 0 {
        c.skip(2)?;
        n.dropped.push(Feature::Trill);
    }
    if f2 & 0x40 != 0 {
        n.dropped.push(Feature::Vibrato);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytes::MAX_STRING;

    #[test]
    fn g5_bend_points_and_text_lengths_are_bounded() {
        let mut data = vec![1u8];
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(MAX_BEND_POINTS as i32 + 1).to_le_bytes());
        assert!(
            bend(&mut Reader::new(&data))
                .unwrap_err()
                .what
                .contains("bend points")
        );

        let data = (MAX_STRING as i32 + 1).to_le_bytes();
        assert!(
            Reader::new(&data)
                .int_string()
                .unwrap_err()
                .what
                .contains("text length")
        );
    }
}
