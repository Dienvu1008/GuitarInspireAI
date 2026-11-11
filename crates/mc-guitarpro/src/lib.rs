//! Guitar Pro interchange adapter (ADR-036): GP3, GP4 and GP5 (5.00) import into a
//! validated score with an import report, and export to the same three versions with
//! an export report (ADR-012).
//!
//! Own clean-room implementation. The format knowledge comes from a public
//! description of the format (facts only) and from byte analysis of files written by
//! TuxGuitar 2.0.1 for this project (spike S10). No other reader's code was used.
//!
//! "Guitar Pro" is a trademark of Arobas Music. This crate is not affiliated with or
//! endorsed by Arobas Music; the name only describes file compatibility.
//!
//! Reading and writing go through a raw model of the file (`raw`); `import` and
//! `export` translate between it and `mc_score::Score`. The score is built only with
//! `Score::from_parts` (GR-09). No file I/O: callers pass and receive bytes.

mod bytes;
mod export;
mod import;
mod raw;
mod read;
mod write;

pub use export::{GpExportError, GpExportIssue, GpExportIssueKind, GpExportReport, export_gp};
pub use import::{
    GpImportError, GpImportIssue, GpImportIssueKind, GpImportReport, MAX_FILE_BYTES, import_gp,
};
pub use raw::GpVersion;

/// Guitar Pro content the score does not hold, named in the reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Feature {
    SongInformation,
    Lyrics,
    TripletFeel,
    DoubleBar,
    ChordDiagram,
    BeatText,
    MixTable,
    Brush,
    HeavyAccent,
    GhostNote,
    Staccato,
    Dynamics,
    Bend,
    HammerOnPullOff,
    Slide,
    Harmonic,
    Vibrato,
    WideVibrato,
    GraceNote,
    TremoloPicking,
    Trill,
    Tap,
    Slap,
    Pop,
    TremoloBar,
    FadeIn,
    Rasgueado,
    Fingering,
    TimeIndependentDuration,
}

impl Feature {
    pub fn name(self) -> &'static str {
        match self {
            Feature::SongInformation => "song information",
            Feature::Lyrics => "lyrics",
            Feature::TripletFeel => "triplet feel",
            Feature::DoubleBar => "double bar line",
            Feature::ChordDiagram => "chord diagram",
            Feature::BeatText => "beat text",
            Feature::MixTable => "mix table change",
            Feature::Brush => "brush stroke",
            Feature::HeavyAccent => "heavy accent",
            Feature::GhostNote => "ghost note",
            Feature::Staccato => "staccato",
            Feature::Dynamics => "dynamics",
            Feature::Bend => "bend",
            Feature::HammerOnPullOff => "hammer-on or pull-off",
            Feature::Slide => "slide",
            Feature::Harmonic => "harmonic",
            Feature::Vibrato => "vibrato",
            Feature::WideVibrato => "wide vibrato",
            Feature::GraceNote => "grace note",
            Feature::TremoloPicking => "tremolo picking",
            Feature::Trill => "trill",
            Feature::Tap => "tap",
            Feature::Slap => "slap",
            Feature::Pop => "pop",
            Feature::TremoloBar => "tremolo bar",
            Feature::FadeIn => "fade in",
            Feature::Rasgueado => "rasgueado",
            Feature::Fingering => "fingering",
            Feature::TimeIndependentDuration => "time-independent duration",
        }
    }
}

/// Collects issues by kind: a count and the first bar (0-based), in kind order.
#[derive(Debug)]
pub(crate) struct Tally<K: Ord + Copy> {
    map: std::collections::BTreeMap<K, (usize, usize)>,
}

impl<K: Ord + Copy> Default for Tally<K> {
    fn default() -> Self {
        Tally {
            map: Default::default(),
        }
    }
}

impl<K: Ord + Copy> Tally<K> {
    pub fn add(&mut self, kind: K, bar: usize) {
        self.add_n(kind, bar, 1);
    }

    pub fn add_n(&mut self, kind: K, bar: usize, n: usize) {
        if n == 0 {
            return;
        }
        let e = self.map.entry(kind).or_insert((0, bar));
        e.0 += n;
        e.1 = e.1.min(bar);
    }

    pub fn into_vec<T>(self, make: impl Fn(K, usize, usize) -> T) -> Vec<T> {
        self.map
            .into_iter()
            .map(|(k, (count, bar))| make(k, count, bar))
            .collect()
    }
}
