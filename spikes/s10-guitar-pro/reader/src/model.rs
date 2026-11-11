//! The spike's test model: what the reader understood, nothing more.

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Song {
    pub version: String,
    pub info: Vec<(String, String)>,
    pub tempo: i32,
    pub key: i32,
    /// Track the lyrics belong to (as stored, GP4/GP5 only).
    pub lyrics_track: i32,
    /// Five lyric lines: (start bar, text), as stored.
    pub lyrics: Vec<(i32, String)>,
    pub headers: Vec<Header>,
    pub tracks: Vec<Track>,
    /// [track][measure]
    pub measures: Vec<Vec<Measure>>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Header {
    pub numerator: u8,
    pub denominator: u8,
    pub repeat_start: bool,
    /// Total plays of the repeated section, 0 when the bar ends no repeat.
    pub repeat_end: u8,
    /// Alternate endings as a bitmask of passes (bit 0 = pass 1), 0 for none
    /// (bytes: f17 stores 3 for "passes 1 and 2").
    pub alternative: u8,
    pub marker: Option<String>,
    pub key: Option<(i8, i8)>,
    pub double_bar: bool,
    pub triplet_feel: u8,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Track {
    pub name: String,
    pub percussion: bool,
    /// MIDI pitches, string 1 (highest) first.
    pub tuning: Vec<i32>,
    pub frets: i32,
    pub capo: i32,
    pub channel: i32,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Measure {
    pub voices: Vec<Vec<Beat>>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Beat {
    /// 0 empty, 1 normal, 2 rest.
    pub status: u8,
    /// -2 whole ... 3 thirty-second (and below).
    pub duration: i8,
    pub dotted: bool,
    /// Tuplet "enters", for example 3 for a triplet.
    pub tuplet: Option<i32>,
    pub notes: Vec<Note>,
    /// Brush: (down speed, up speed), 0 for none.
    pub stroke: (u8, u8),
    /// Pick stroke: -1 down... as stored (0 none).
    pub pick_stroke: i8,
    pub text: Option<String>,
    pub chord: Option<Chord>,
    pub tempo_change: Option<i32>,
    /// Effects the reader skipped, by name.
    pub unsupported: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Note {
    /// 1 = highest string.
    pub string: u8,
    pub fret: i8,
    pub tie: bool,
    pub dead: bool,
    pub accent: bool,
    pub heavy_accent: bool,
    pub ghost: bool,
    pub palm_mute: bool,
    pub let_ring: bool,
    pub staccato: bool,
    pub velocity: Option<i8>,
    pub unsupported: Vec<String>,
}

/// A chord diagram: what a score model could keep. Root, type, alterations,
/// barres and fingering are read and dropped.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Chord {
    pub name: String,
    pub first_fret: i32,
    /// Fret per string, string 1 first, -1 for a string not played.
    pub frets: Vec<i32>,
}
