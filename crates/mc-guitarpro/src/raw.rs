//! The file as Guitar Pro stores it, before any mapping to the score. Reader and writer
//! work on this model only; `import` and `export` translate between it and `Score`.
//! Values keep the file's meaning (for example the repeat count as stored), and the
//! version rules are applied in `import` and `export`.

/// The format versions this adapter reads and writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpVersion {
    Gp3,
    Gp4,
    Gp5,
}

impl GpVersion {
    /// The version string written by `export` (TuxGuitar writes the same strings).
    pub fn header(self) -> &'static str {
        match self {
            GpVersion::Gp3 => "FICHIER GUITAR PRO v3.00",
            GpVersion::Gp4 => "FICHIER GUITAR PRO v4.06",
            GpVersion::Gp5 => "FICHIER GUITAR PRO v5.00",
        }
    }

    /// The version for a file extension (`gp3`, `gp4`, `gp5`, any case, with or
    /// without the dot). The app chooses the export version with this.
    pub fn from_extension(ext: &str) -> Option<GpVersion> {
        match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
            "gp3" => Some(GpVersion::Gp3),
            "gp4" => Some(GpVersion::Gp4),
            "gp5" => Some(GpVersion::Gp5),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            GpVersion::Gp3 => "gp3",
            GpVersion::Gp4 => "gp4",
            GpVersion::Gp5 => "gp5",
        }
    }

    /// Voices per measure and track.
    pub(crate) fn voices(self) -> usize {
        if self == GpVersion::Gp5 { 2 } else { 1 }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RawSong {
    /// Title, subtitle, artist, album, words, (music, GP5 only), copyright, tab, instructions.
    pub info: Vec<String>,
    pub notice: Vec<String>,
    /// GP3 and GP4 only.
    pub triplet_feel: bool,
    /// GP4 and GP5: lyrics track (1-based as stored) and five (start bar, text) lines.
    pub lyrics_track: i32,
    pub lyrics: Vec<(i32, String)>,
    pub tempo: i32,
    pub key: i32,
    pub headers: Vec<RawHeader>,
    pub tracks: Vec<RawTrack>,
    /// `[measure][track]`, as stored.
    pub bars: Vec<Vec<RawBar>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RawHeader {
    /// Present only when the bar changes it.
    pub numerator: Option<u8>,
    pub denominator: Option<u8>,
    pub repeat_start: bool,
    /// As stored: GP5 the total plays, GP3 and GP4 the number of repeats.
    pub repeat_close: Option<u8>,
    /// Bitmask of passes (bit 0 = pass 1), 0 for none.
    pub alternative: u8,
    /// Name and colour.
    pub marker: Option<(String, [u8; 4])>,
    /// Fifths and the minor flag, present when the key changes.
    pub key: Option<(i8, i8)>,
    pub double_bar: bool,
    /// GP5 only.
    pub triplet_feel: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RawTrack {
    pub percussion: bool,
    pub name: String,
    /// MIDI numbers, string 1 (highest) first.
    pub tuning: Vec<i32>,
    pub port: i32,
    pub channel: i32,
    pub effect_channel: i32,
    pub frets: i32,
    pub capo: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RawBar {
    pub voices: Vec<Vec<RawBeat>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BeatStatus {
    /// Status 0: the beat takes no time and has no content.
    Empty,
    #[default]
    Normal,
    Rest,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RawBeat {
    pub status: BeatStatus,
    /// -2 whole, -1 half, 0 quarter ... 4 sixty-fourth.
    pub duration: i8,
    pub dotted: bool,
    /// Tuplet "enters", for example 3 for a triplet.
    pub tuplet: Option<i32>,
    pub chord: Option<RawChord>,
    pub text: Option<String>,
    /// Pick stroke as stored: 1 up, 2 down (GP4 and GP5).
    pub pick_stroke: i8,
    /// Brush bytes in the order of the format description: (up, down).
    pub brush: (u8, u8),
    pub tempo: Option<i32>,
    /// A mix table that changes more than the tempo.
    pub mix_other: bool,
    pub notes: Vec<RawNote>,
    /// Beat effects the score does not hold, in reading order.
    pub dropped: Vec<crate::Feature>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RawChord {
    pub name: String,
    pub first_fret: i32,
    pub frets: Vec<i32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NoteType {
    #[default]
    Normal,
    Tie,
    Dead,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RawNote {
    /// 1 = highest string.
    pub string: u8,
    pub kind: NoteType,
    pub fret: i8,
    pub accent: bool,
    pub heavy_accent: bool,
    pub ghost: bool,
    pub let_ring: bool,
    pub palm_mute: bool,
    pub staccato: bool,
    pub velocity: Option<i8>,
    /// Note effects the score does not hold, in reading order.
    pub dropped: Vec<crate::Feature>,
}
