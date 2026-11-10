//! The Pattern IR (ADR-034 section 2). Plain data: every field is public, and a
//! pattern only becomes a [`crate::PatternDefinition`] through validation
//! ([`crate::PatternLibrary`]). Time is rational everywhere (ADR-011): onsets in whole
//! notes from the pattern start, durations as written note values. No floating point.

use mc_score::{NoteEffects, Stroke};
use mc_theory::fretted::StringIndex;
use mc_theory::{NoteValue, Rational, TimeSignature};

use crate::ids::GestureId;

/// The techniques of a strike: accent, palm mute, dead, let ring, string-percussive.
/// It is the score's own set (ADR-035), so binding copies it unchanged.
pub type TechniqueSet = NoteEffects;

/// Stroke direction of a strike: the score's own `Stroke` (ADR-035).
pub type Direction = Stroke;

/// Which strings a strike targets. Resolved against a voicing at bind time (6a-3,
/// ADR-034 section 5); only sounded strings are ever selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StringSelector {
    /// The lowest sounded string of the voicing.
    Bass,
    /// The alternate bass (ADR-034 section 5).
    Alt,
    /// Up to n highest sounded strings above the bass; `Treble(3)` is plain "Treble".
    /// n is at least 1.
    Treble(u8),
    /// A physical string, 1 = highest (Q-02). It must be sounded at bind time.
    Absolute(StringIndex),
    /// Every sounded string.
    Strum,
    /// The sounded strings between the two strings, inclusive, in either order.
    StrumRange(StringIndex, StringIndex),
}

impl StringSelector {
    /// Plain "Treble" (Q-04): the three highest sounded strings above the bass.
    pub const TREBLE: StringSelector = StringSelector::Treble(3);
}

/// One attack: the union of its targets is struck together ("+").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Strike {
    pub targets: Vec<StringSelector>,
    pub direction: Option<Direction>,
    pub techniques: TechniqueSet,
}

impl Strike {
    /// A strike on the given targets, with no direction and no technique.
    pub fn on(targets: impl Into<Vec<StringSelector>>) -> Strike {
        Strike {
            targets: targets.into(),
            direction: None,
            techniques: TechniqueSet::default(),
        }
    }
}

/// What a step does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Strike(Strike),
    /// A user gesture from the library, by id, so a rename keeps every use working.
    /// Expanded at bind time (ADR-034 section 6).
    Gesture(GestureId),
    /// Continues the previous strike of the lane on the selected strings, which get
    /// tied notes; the other strings stop. `None` holds every string still held.
    Hold(Option<Vec<StringSelector>>),
    /// A true rest: no attack and no motion.
    Rest,
}

/// One step of a lane. Onset and duration are independent (Q-03): the duration is how
/// long the notes sound, the distance to the next attack comes from the onsets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// From the pattern start, in whole notes.
    pub onset: Rational,
    /// The written value: base, dots and tuplet ratio, as in the score.
    pub duration: NoteValue,
    pub action: Action,
}

/// The steps of one lane, in onset order (PT-03). Gaps are rests (PT-09).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Lane {
    pub steps: Vec<Step>,
}

/// At most two lanes (ADR-034 decision 13.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lanes {
    /// One strum lane, written to voice 1.
    Strum(Lane),
    /// The thumb lane (voice 1) and the fingers lane (voice 2). Steps of different
    /// lanes may overlap.
    ThumbFingers { thumb: Lane, fingers: Lane },
}

/// Names a lane in errors and technique spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LaneId {
    Strum,
    Thumb,
    Fingers,
}

impl Lanes {
    /// The lanes with their ids, in voice order.
    pub fn iter(&self) -> impl Iterator<Item = (LaneId, &Lane)> {
        let pair: [Option<(LaneId, &Lane)>; 2] = match self {
            Lanes::Strum(l) => [Some((LaneId::Strum, l)), None],
            Lanes::ThumbFingers { thumb, fingers } => [
                Some((LaneId::Thumb, thumb)),
                Some((LaneId::Fingers, fingers)),
            ],
        };
        pair.into_iter().flatten()
    }

    /// The lane with this id, if the pattern has it.
    pub fn get(&self, id: LaneId) -> Option<&Lane> {
        self.iter().find(|(l, _)| *l == id).map(|(_, lane)| lane)
    }
}

/// One technique flag, for a span.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Technique {
    Accent,
    PalmMute,
    Dead,
    LetRing,
    StringPercussive,
}

/// One technique over a run of steps of one lane, by step index, inclusive (PT-07).
/// Spans combine with the steps' own techniques; nothing is rejected, the sound
/// follows the ADR-035 precedence and every flag is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TechniqueSpan {
    pub lane: LaneId,
    pub first: usize,
    pub last: usize,
    pub technique: Technique,
}

/// How a pattern was made. Information only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Grid,
    Text,
    Tap,
}

/// Free information about a pattern; never affects validation or binding.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PatternMeta {
    pub description: String,
    pub tags: Vec<String>,
    pub origin: Option<Origin>,
}

/// A pattern as plain data, before validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternParts {
    pub name: String,
    /// In whole notes, greater than 0; may span several bars.
    pub length: Rational,
    /// If set, every target bar must use it (placement, ADR-034 section 7).
    pub meter: Option<TimeSignature>,
    pub lanes: Lanes,
    pub spans: Vec<TechniqueSpan>,
    pub meta: PatternMeta,
}
