//! The pattern engine in a session (ADR-034 section 7): `ApplyPattern` binds a
//! pattern of the session library to one voicing, places it over whole bars and
//! replaces voices 1 and 2 of the track, behind the overwrite safeguard. The library
//! lives in the session, is not saved before format 1.2 (6a-5) and is not part of undo.

use mc_guitar::Voicing;
use mc_pattern::{BindFailure, PatternLibrary, PatternMeta, PatternParts, Strike, place};
use mc_score::{DomainError, DomainEvent, FragmentBeat, Score};
use mc_theory::fretted::Fret;
use mc_theory::{BaseValue, Rational, chord_symbol};

use crate::command::ApplyPattern;
use crate::error::CommandError;

/// Pattern types re-exported for adapters (mc-ffi depends on mc-app only).
pub use mc_pattern::{
    BindError, Fit, LaneId, Lanes, PatternId, PlaceError, PlaceReport, PlacedBar, StepChange,
};
pub use mc_theory::NoteValue;

/// Why `ApplyPattern` was refused. On every error nothing changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyError {
    UnknownPattern(PatternId),
    Bind(Vec<BindError>),
    Place(PlaceError),
    /// A voice the pattern writes holds beats in these bars (absolute indices) and
    /// `overwrite` was false. The UI shows the preview and asks before resending.
    WouldOverwrite {
        bars: Vec<usize>,
    },
}

impl ApplyError {
    /// The rule id for callers: `PE-07` overwrite safeguard, `PE-08` range and fit,
    /// `PE-04` bar lines and lengths, `PT-02` binding, `PT-06` unknown pattern.
    pub fn rule(&self) -> &'static str {
        match self {
            ApplyError::WouldOverwrite { .. } => "PE-07",
            ApplyError::UnknownPattern(_) => "PT-06",
            ApplyError::Bind(_) => "PT-02",
            ApplyError::Place(
                PlaceError::EmptyRange
                | PlaceError::PickupInRange { .. }
                | PlaceError::MeterMismatch { .. }
                | PlaceError::FitRequired { .. },
            ) => "PE-08",
            ApplyError::Place(_) => "PE-04",
        }
    }
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bars = |b: &[usize]| {
            b.iter()
                .map(|x| (x + 1).to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        match self {
            ApplyError::WouldOverwrite { bars: b } => {
                write!(
                    f,
                    "bars {} already have content in the voices the pattern writes",
                    bars(b)
                )
            }
            ApplyError::UnknownPattern(id) => write!(f, "no pattern {}", id.raw()),
            ApplyError::Bind(errors) => {
                let first = errors.first().map(|e| format!("{e:?}")).unwrap_or_default();
                write!(
                    f,
                    "the pattern does not fit this voicing ({} problem(s), first: {first})",
                    errors.len()
                )
            }
            ApplyError::Place(PlaceError::FitRequired { range, pattern }) => write!(
                f,
                "the range ({range} whole notes) is not a whole number of repetitions of the pattern ({pattern}); choose how to fit it"
            ),
            ApplyError::Place(PlaceError::PickupInRange { bar }) => {
                write!(f, "the range contains a pickup bar (range bar {})", bar + 1)
            }
            ApplyError::Place(PlaceError::MeterMismatch { bar, meter, found }) => write!(
                f,
                "the pattern is in {}/{} but range bar {} is in {}/{}",
                meter.numerator(),
                meter.denominator(),
                bar + 1,
                found.numerator(),
                found.denominator()
            ),
            ApplyError::Place(e) => write!(f, "the pattern cannot be placed: {e:?}"),
        }
    }
}

/// What `ApplyPattern` wrote, or would write (preview).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternReport {
    pub first: usize,
    pub last: usize,
    /// Voices written: 1 (Strum) or 2 (Thumb and Fingers).
    pub voices: usize,
    /// Bars of the range whose written voices held beats before (absolute indices).
    pub overwritten: Vec<usize>,
    pub place: PlaceReport,
}

/// A preview of `ApplyPattern`: the content of the written voices per bar of the range,
/// and the report. Nothing is changed by computing it.
///
/// `revision` is the document revision the preview was computed at, read under the same
/// lock. A caller that asks the user to confirm an overwrite sends it back as the
/// command's `expected_revision`: if the score changed in between, the core refuses
/// with `StaleRevision` (E-01) and the caller must preview and confirm again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternPreview {
    pub revision: crate::command::Revision,
    pub bars: Vec<PlacedBar>,
    pub report: PatternReport,
}

/// The built-in sample pattern of 6a-3: corpus pattern 3, alternating bass
/// "boom-chick" (`Bass:4 D Treble:4 Alt:4 D Treble:4`, one 4/4 bar). It is a pattern,
/// not a gesture; no gesture is built in (ADR-034 section 6).
pub fn sample_parts() -> PatternParts {
    use mc_pattern::{Action, Direction, Lane, Step, StringSelector};
    let q = NoteValue::plain(BaseValue::QUARTER);
    let at = |k: i64| Rational::new(k, 4).expect("quarters");
    let strike = |targets: Vec<StringSelector>, down: bool| {
        Action::Strike(Strike {
            direction: down.then_some(Direction::Down),
            ..Strike::on(targets)
        })
    };
    let steps = vec![
        Step {
            onset: at(0),
            duration: q,
            action: strike(vec![StringSelector::Bass], false),
        },
        Step {
            onset: at(1),
            duration: q,
            action: strike(vec![StringSelector::TREBLE], true),
        },
        Step {
            onset: at(2),
            duration: q,
            action: strike(vec![StringSelector::Alt], false),
        },
        Step {
            onset: at(3),
            duration: q,
            action: strike(vec![StringSelector::TREBLE], true),
        },
    ];
    PatternParts {
        name: "Boom-chick".into(),
        length: Rational::ONE,
        meter: None,
        lanes: Lanes::Strum(Lane { steps }),
        spans: Vec::new(),
        meta: PatternMeta {
            description: "Alternating bass: bass, down-strum treble, alternate bass, down-strum treble (corpus pattern 3)".into(),
            tags: vec!["sample".into()],
            origin: None,
        },
    }
}

/// A library holding the sample pattern, and its id.
pub fn sample_library() -> (PatternLibrary, PatternId) {
    let mut lib = PatternLibrary::new();
    let id = lib
        .define_pattern(sample_parts())
        .expect("the sample pattern is valid");
    (lib, id)
}

struct Plan {
    content: Vec<Vec<(usize, Vec<FragmentBeat>)>>,
    preview: PatternPreview,
}

fn plan(score: &Score, lib: &PatternLibrary, a: &ApplyPattern) -> Result<Plan, CommandError> {
    let track = score
        .track(a.track)
        .ok_or_else(|| DomainError::NotFound(format!("track {}", a.track.raw())))?;
    if a.first > a.last || a.last >= score.master_bar_count() {
        return Err(DomainError::InvalidPosition(format!(
            "bar range {} to {} of {}",
            a.first,
            a.last,
            score.master_bar_count()
        ))
        .into());
    }
    let voicing = Voicing::new(a.voicing.iter().map(|f| f.map(Fret)).collect())?;
    let chord = chord_symbol::parse(&a.chord)?;
    let fragment = lib
        .bind(a.pattern, &voicing, &chord, track.fretted())
        .map_err(|f| match f {
            BindFailure::Library(_) => ApplyError::UnknownPattern(a.pattern),
            BindFailure::Bind(e) => ApplyError::Bind(e),
        })?;
    let placement = place(&fragment, &score.master_bars()[a.first..=a.last], a.fit)
        .map_err(ApplyError::Place)?;
    let voices = fragment.lanes.len();
    let overwritten = (a.first..=a.last)
        .filter(|&b| {
            track
                .bar(b)
                .is_some_and(|bar| bar.voices.iter().take(voices).any(|v| !v.beats.is_empty()))
        })
        .collect();
    let content = placement
        .bars
        .iter()
        .map(|pb| {
            pb.voices
                .iter()
                .enumerate()
                .map(|(v, (_, beats))| (v, beats.clone()))
                .collect()
        })
        .collect();
    Ok(Plan {
        content,
        preview: PatternPreview {
            revision: crate::command::Revision(0),
            bars: placement.bars,
            report: PatternReport {
                first: a.first,
                last: a.last,
                voices,
                overwritten,
                place: placement.report,
            },
        },
    })
}

/// The preview the apply dialog shows; changes nothing.
pub(crate) fn preview(
    score: &Score,
    lib: &PatternLibrary,
    a: &ApplyPattern,
    revision: crate::command::Revision,
) -> Result<PatternPreview, CommandError> {
    let mut preview = plan(score, lib, a)?.preview;
    preview.revision = revision;
    Ok(preview)
}

/// Applies the pattern to `score` (the session's working copy), or refuses.
pub(crate) fn apply(
    score: &mut Score,
    lib: &PatternLibrary,
    a: &ApplyPattern,
) -> Result<(Vec<DomainEvent>, PatternReport), CommandError> {
    let plan = plan(score, lib, a)?;
    if !a.overwrite && !plan.preview.report.overwritten.is_empty() {
        return Err(ApplyError::WouldOverwrite {
            bars: plan.preview.report.overwritten,
        }
        .into());
    }
    let events = score.replace_voices(a.track, a.first, &plan.content)?;
    Ok((events, plan.preview.report))
}
