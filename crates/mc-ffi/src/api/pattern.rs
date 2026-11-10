//! Pattern engine API (ADR-034, slice 6a-3). Thin bridge over `mc-app`: it owns no
//! musical rule (GR-05). Selectors, binding, fit and the overwrite safeguard are all
//! decided by the core; Dart shows the preview and sends `CommandDto::ApplyPattern`.

use flutter_rust_bridge::frb;

use crate::api::document::DocumentHandle;
use crate::api::types::{ApiError, DurationDto, TupletDto};
use mc_app::pattern::{
    ApplyError, Fit, LaneId, Lanes, NoteValue, PatternId, PatternReport, PlaceError, StepChange,
};
use mc_app::{ApplyPattern, CommandError};

/// A pattern of the session library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternInfoDto {
    pub id: i64,
    pub name: String,
    pub description: String,
    /// Pattern length in whole notes, as a fraction.
    pub length_numerator: i64,
    pub length_denominator: i64,
    /// True for Thumb and Fingers (voices 1 and 2), false for one Strum lane (voice 1).
    pub two_lanes: bool,
}

/// How to fit a range that is not a whole number of repetitions (ADR-034 section 7).
/// There is no default: `None` in [`ApplyPatternDto::fit`] asks the core to refuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitDto {
    RepeatAndCut,
    WholeRepetitionsOnly,
}

/// The arguments of `ApplyPattern`. `frets` is per string, string 1 first, as in
/// `VoicingDto`: `-1` muted, `0` open, `>0` fretted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyPatternDto {
    pub track: i64,
    /// First and last master bar, 0-based and inclusive.
    pub first: u32,
    pub last: u32,
    pub pattern: i64,
    pub frets: Vec<i32>,
    pub chord: String,
    pub fit: Option<FitDto>,
    pub overwrite: bool,
}

/// A step that `RepeatAndCut` dropped (`now` empty) or shortened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepChangeDto {
    /// "strum", "thumb" or "fingers".
    pub lane: String,
    pub repetition: u32,
    pub step: u32,
    pub onset_numerator: i64,
    pub onset_denominator: i64,
    pub written: DurationDto,
    pub now: Vec<DurationDto>,
}

/// What `ApplyPattern` wrote or would write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternReportDto {
    pub first: u32,
    pub last: u32,
    /// Voices written: 1 or 2.
    pub voices: u32,
    /// Bars (0-based) whose written voices already hold beats.
    pub overwritten: Vec<u32>,
    pub repetitions: u32,
    /// Attacked notes written.
    pub notes: u32,
    pub dropped: Vec<StepChangeDto>,
    pub shortened: Vec<StepChangeDto>,
}

/// The preview of the apply dialog. `needs_fit` is true when the range is not a whole
/// number of repetitions and no fit was given; then `report` is `None` and the dialog
/// asks for a fit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternPreviewDto {
    /// The document revision the preview was computed at. Send it as
    /// `expected_revision` with `ApplyPattern`: the core then refuses (E-01) if the
    /// score changed since, and the caller must preview and confirm again.
    pub revision: i64,
    pub needs_fit: bool,
    pub report: Option<PatternReportDto>,
}

fn duration(v: NoteValue) -> DurationDto {
    DurationDto {
        base: v.base().exponent(),
        dots: v.dots(),
        tuplet: v.tuplet().map(|t| TupletDto {
            actual: t.actual(),
            normal: t.normal(),
        }),
    }
}

fn change(c: &StepChange) -> StepChangeDto {
    StepChangeDto {
        lane: match c.lane {
            LaneId::Strum => "strum",
            LaneId::Thumb => "thumb",
            LaneId::Fingers => "fingers",
        }
        .into(),
        repetition: c.repetition as u32,
        step: c.step as u32,
        onset_numerator: c.onset.numerator(),
        onset_denominator: c.onset.denominator(),
        written: duration(c.written),
        now: c.now.iter().copied().map(duration).collect(),
    }
}

pub(crate) fn report_dto(r: &PatternReport) -> PatternReportDto {
    PatternReportDto {
        first: r.first as u32,
        last: r.last as u32,
        voices: r.voices as u32,
        overwritten: r.overwritten.iter().map(|b| *b as u32).collect(),
        repetitions: r.place.repetitions as u32,
        notes: r.place.notes as u32,
        dropped: r.place.dropped.iter().map(change).collect(),
        shortened: r.place.shortened.iter().map(change).collect(),
    }
}

impl ApplyPatternDto {
    pub(crate) fn to_app(&self) -> Result<ApplyPattern, ApiError> {
        let voicing = self
            .frets
            .iter()
            .map(|&f| match f {
                -1 => Ok(None),
                0..=255 => Ok(Some(f as u8)),
                _ => Err(ApiError::new("voicing", format!("fret {f} out of range"))),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ApplyPattern {
            track: mc_app::types::TrackId::from_raw(self.track as u64),
            first: self.first as usize,
            last: self.last as usize,
            pattern: PatternId::from_raw(self.pattern as u64),
            voicing,
            chord: self.chord.clone(),
            fit: self.fit.map(|f| match f {
                FitDto::RepeatAndCut => Fit::RepeatAndCut,
                FitDto::WholeRepetitionsOnly => Fit::WholeRepetitionsOnly,
            }),
            overwrite: self.overwrite,
        })
    }
}

/// The patterns of the session library (6a-3: the built-in sample).
#[frb(sync)]
pub fn patterns(document: &DocumentHandle) -> Vec<PatternInfoDto> {
    let s = document.lock_for_pattern();
    s.patterns()
        .patterns()
        .map(|p| {
            let parts = p.parts();
            PatternInfoDto {
                id: p.id().raw() as i64,
                name: parts.name.clone(),
                description: parts.meta.description.clone(),
                length_numerator: parts.length.numerator(),
                length_denominator: parts.length.denominator(),
                two_lanes: matches!(parts.lanes, Lanes::ThumbFingers { .. }),
            }
        })
        .collect()
}

/// The id of the built-in sample pattern.
#[frb(sync)]
pub fn sample_pattern(document: &DocumentHandle) -> i64 {
    document.lock_for_pattern().sample_pattern().raw() as i64
}

/// What `ApplyPattern` would write, without changing anything. A missing fit for a
/// range that needs one is reported as `needs_fit`; every other refusal is an
/// `ApiError` with the core's rule id.
#[frb(sync)]
pub fn preview_pattern(
    document: &DocumentHandle,
    args: ApplyPatternDto,
) -> Result<PatternPreviewDto, ApiError> {
    let a = args.to_app()?;
    // One lock for the preview and its revision, so they belong together.
    let s = document.lock_for_pattern();
    let revision = s.revision().0 as i64;
    match s.preview_pattern(&a) {
        Ok(p) => Ok(PatternPreviewDto {
            revision: p.revision.0 as i64,
            needs_fit: false,
            report: Some(report_dto(&p.report)),
        }),
        Err(CommandError::Pattern(ApplyError::Place(PlaceError::FitRequired { .. })))
            if a.fit.is_none() =>
        {
            Ok(PatternPreviewDto {
                revision,
                needs_fit: true,
                report: None,
            })
        }
        Err(e) => Err(e.into()),
    }
}
