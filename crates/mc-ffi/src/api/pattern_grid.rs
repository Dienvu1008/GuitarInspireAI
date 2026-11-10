//! The pattern grid editor and the gesture palette (ADR-034, slice 6a-5c). A thin
//! bridge over `mc-app`: the grid's music rules (the value of a cell, spans following
//! their steps, step order) and every library rule are the core's (GR-05). Dart draws
//! the view model of [`pattern_grid`] and sends `CommandDto::EditPattern` and the
//! other library commands.

use flutter_rust_bridge::frb;

use crate::api::document::DocumentHandle;
use crate::api::pattern::duration;
use crate::api::types::{ApiError, DurationDto, StrokeDto};
use mc_app::pattern::grid::{Cell, CellKind, Edit, GridView, Row};
use mc_app::pattern::{
    Action, GestureId, LaneId, PatternId, Strike, StringSelector, Technique, TechniqueSet,
};
use mc_app::types::{Rational, TimeSignature};

/// A lane of a pattern: one strum lane, or thumb and fingers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternLaneDto {
    Strum,
    Thumb,
    Fingers,
}

impl PatternLaneDto {
    pub(crate) fn to_app(self) -> LaneId {
        match self {
            PatternLaneDto::Strum => LaneId::Strum,
            PatternLaneDto::Thumb => LaneId::Thumb,
            PatternLaneDto::Fingers => LaneId::Fingers,
        }
    }

    fn from_app(l: LaneId) -> Self {
        match l {
            LaneId::Strum => PatternLaneDto::Strum,
            LaneId::Thumb => PatternLaneDto::Thumb,
            LaneId::Fingers => PatternLaneDto::Fingers,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectorKindDto {
    Bass,
    Alt,
    /// Up to `count` highest sounded strings above the bass (plain Treble is 3).
    Treble,
    /// The physical string `string` (1 is the highest).
    String,
    Strum,
    /// The sounded strings from `from` to `to`.
    StrumRange,
}

/// What a strike targets. Only the fields of its kind are read; the others are 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridSelectorDto {
    pub kind: SelectorKindDto,
    pub count: u8,
    pub string: u8,
    pub from: u8,
    pub to: u8,
}

fn string_index(n: u8) -> Result<mc_app::pattern::StringIndex, ApiError> {
    mc_app::pattern::StringIndex::new(n)
        .map_err(|_| ApiError::new("PE-14", format!("string {n} does not exist")))
}

impl GridSelectorDto {
    pub(crate) fn to_app(self) -> Result<StringSelector, ApiError> {
        Ok(match self.kind {
            SelectorKindDto::Bass => StringSelector::Bass,
            SelectorKindDto::Alt => StringSelector::Alt,
            SelectorKindDto::Treble => StringSelector::Treble(self.count),
            SelectorKindDto::String => StringSelector::Absolute(string_index(self.string)?),
            SelectorKindDto::Strum => StringSelector::Strum,
            SelectorKindDto::StrumRange => {
                StringSelector::StrumRange(string_index(self.from)?, string_index(self.to)?)
            }
        })
    }

    pub(crate) fn from_app(s: StringSelector) -> Self {
        let plain = |kind| GridSelectorDto {
            kind,
            count: 0,
            string: 0,
            from: 0,
            to: 0,
        };
        match s {
            StringSelector::Bass => plain(SelectorKindDto::Bass),
            StringSelector::Alt => plain(SelectorKindDto::Alt),
            StringSelector::Treble(n) => GridSelectorDto {
                count: n,
                ..plain(SelectorKindDto::Treble)
            },
            StringSelector::Absolute(i) => GridSelectorDto {
                string: i.get(),
                ..plain(SelectorKindDto::String)
            },
            StringSelector::Strum => plain(SelectorKindDto::Strum),
            StringSelector::StrumRange(a, b) => GridSelectorDto {
                from: a.get(),
                to: b.get(),
                ..plain(SelectorKindDto::StrumRange)
            },
        }
    }
}

/// The five techniques of ADR-035.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TechniquesDto {
    pub accent: bool,
    pub palm_mute: bool,
    pub dead: bool,
    pub let_ring: bool,
    pub string_percussive: bool,
}

impl TechniquesDto {
    fn to_app(self) -> TechniqueSet {
        TechniqueSet {
            accent: self.accent,
            palm_mute: self.palm_mute,
            dead: self.dead,
            let_ring: self.let_ring,
            string_percussive: self.string_percussive,
        }
    }

    fn from_app(t: TechniqueSet) -> Self {
        TechniquesDto {
            accent: t.accent,
            palm_mute: t.palm_mute,
            dead: t.dead,
            let_ring: t.let_ring,
            string_percussive: t.string_percussive,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TechniqueKindDto {
    Accent,
    PalmMute,
    Dead,
    LetRing,
    StringPercussive,
}

impl TechniqueKindDto {
    fn to_app(self) -> Technique {
        match self {
            TechniqueKindDto::Accent => Technique::Accent,
            TechniqueKindDto::PalmMute => Technique::PalmMute,
            TechniqueKindDto::Dead => Technique::Dead,
            TechniqueKindDto::LetRing => Technique::LetRing,
            TechniqueKindDto::StringPercussive => Technique::StringPercussive,
        }
    }

    fn from_app(t: Technique) -> Self {
        match t {
            Technique::Accent => TechniqueKindDto::Accent,
            Technique::PalmMute => TechniqueKindDto::PalmMute,
            Technique::Dead => TechniqueKindDto::Dead,
            Technique::LetRing => TechniqueKindDto::LetRing,
            Technique::StringPercussive => TechniqueKindDto::StringPercussive,
        }
    }
}

/// One attack: the union of its targets, struck together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridStrikeDto {
    pub targets: Vec<GridSelectorDto>,
    pub direction: Option<StrokeDto>,
    pub techniques: TechniquesDto,
}

impl GridStrikeDto {
    pub(crate) fn to_app(&self) -> Result<Strike, ApiError> {
        Ok(Strike {
            targets: self
                .targets
                .iter()
                .map(|t| t.to_app())
                .collect::<Result<_, _>>()?,
            direction: self.direction.map(StrokeDto::to_app),
            techniques: self.techniques.to_app(),
        })
    }

    fn from_app(k: &Strike) -> Self {
        GridStrikeDto {
            targets: k
                .targets
                .iter()
                .copied()
                .map(GridSelectorDto::from_app)
                .collect(),
            direction: k.direction.map(StrokeDto::from_app),
            techniques: TechniquesDto::from_app(k.techniques),
        }
    }
}

/// What a step does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridActionDto {
    Strike {
        strike: GridStrikeDto,
    },
    Gesture {
        gesture: i64,
    },
    /// Holds every string still held (`strings` is `None`), or these strings.
    Hold {
        strings: Option<Vec<GridSelectorDto>>,
    },
    Rest,
}

impl GridActionDto {
    fn to_app(&self) -> Result<Action, ApiError> {
        Ok(match self {
            GridActionDto::Strike { strike } => Action::Strike(strike.to_app()?),
            GridActionDto::Gesture { gesture } => {
                Action::Gesture(GestureId::from_raw(*gesture as u64))
            }
            GridActionDto::Hold { strings } => Action::Hold(
                strings
                    .as_ref()
                    .map(|v| v.iter().map(|s| s.to_app()).collect::<Result<_, _>>())
                    .transpose()?,
            ),
            GridActionDto::Rest => Action::Rest,
        })
    }
}

/// A cell of the grid: `cell` of `parts` cells of beat `beat` (all from 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridCellDto {
    pub beat: u32,
    pub parts: u8,
    pub cell: u8,
}

impl GridCellDto {
    fn to_app(self) -> Cell {
        Cell {
            beat: self.beat,
            parts: self.parts,
            cell: self.cell,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeterDto {
    pub numerator: u8,
    pub denominator: u8,
}

impl MeterDto {
    pub(crate) fn to_app(self) -> Result<TimeSignature, ApiError> {
        TimeSignature::new(self.numerator, self.denominator)
            .map_err(|e| ApiError::new("T", e.to_string()))
    }
}

/// One grid edit (`mc_pattern::grid::Edit`). Step indices are per lane, from 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridEditDto {
    Place {
        lane: PatternLaneDto,
        at: GridCellDto,
        cells: u8,
        action: GridActionDto,
    },
    Move {
        lane: PatternLaneDto,
        index: u32,
        to: GridCellDto,
    },
    Resize {
        lane: PatternLaneDto,
        index: u32,
        parts: u8,
        cells: u8,
    },
    Delete {
        lane: PatternLaneDto,
        first: u32,
        last: u32,
    },
    SetAction {
        lane: PatternLaneDto,
        index: u32,
        action: GridActionDto,
    },
    ToggleTarget {
        lane: PatternLaneDto,
        index: u32,
        selector: GridSelectorDto,
    },
    SetDirection {
        lane: PatternLaneDto,
        first: u32,
        last: u32,
        direction: Option<StrokeDto>,
    },
    SetTechnique {
        lane: PatternLaneDto,
        index: u32,
        technique: TechniqueKindDto,
        on: bool,
    },
    AddSpan {
        lane: PatternLaneDto,
        first: u32,
        last: u32,
        technique: TechniqueKindDto,
    },
    RemoveSpan {
        index: u32,
    },
    SetLength {
        numerator: i64,
        denominator: i64,
    },
    SetMeter {
        meter: Option<MeterDto>,
    },
    SetName {
        name: String,
    },
    /// The description and tags; the origin is kept.
    SetMeta {
        description: String,
        tags: Vec<String>,
    },
}

impl GridEditDto {
    /// `origin` is the pattern's current origin, kept by `SetMeta`.
    pub(crate) fn to_app(&self, origin: Option<mc_app::pattern::Origin>) -> Result<Edit, ApiError> {
        let i = |x: u32| x as usize;
        Ok(match self {
            GridEditDto::Place {
                lane,
                at,
                cells,
                action,
            } => Edit::Place {
                lane: lane.to_app(),
                at: at.to_app(),
                cells: *cells,
                action: action.to_app()?,
            },
            GridEditDto::Move { lane, index, to } => Edit::Move {
                lane: lane.to_app(),
                index: i(*index),
                to: to.to_app(),
            },
            GridEditDto::Resize {
                lane,
                index,
                parts,
                cells,
            } => Edit::Resize {
                lane: lane.to_app(),
                index: i(*index),
                parts: *parts,
                cells: *cells,
            },
            GridEditDto::Delete { lane, first, last } => Edit::Delete {
                lane: lane.to_app(),
                first: i(*first),
                last: i(*last),
            },
            GridEditDto::SetAction {
                lane,
                index,
                action,
            } => Edit::SetAction {
                lane: lane.to_app(),
                index: i(*index),
                action: action.to_app()?,
            },
            GridEditDto::ToggleTarget {
                lane,
                index,
                selector,
            } => Edit::ToggleTarget {
                lane: lane.to_app(),
                index: i(*index),
                selector: selector.to_app()?,
            },
            GridEditDto::SetDirection {
                lane,
                first,
                last,
                direction,
            } => Edit::SetDirection {
                lane: lane.to_app(),
                first: i(*first),
                last: i(*last),
                direction: direction.map(StrokeDto::to_app),
            },
            GridEditDto::SetTechnique {
                lane,
                index,
                technique,
                on,
            } => Edit::SetTechnique {
                lane: lane.to_app(),
                index: i(*index),
                technique: technique.to_app(),
                on: *on,
            },
            GridEditDto::AddSpan {
                lane,
                first,
                last,
                technique,
            } => Edit::AddSpan {
                lane: lane.to_app(),
                first: i(*first),
                last: i(*last),
                technique: technique.to_app(),
            },
            GridEditDto::RemoveSpan { index } => Edit::RemoveSpan { index: i(*index) },
            GridEditDto::SetLength {
                numerator,
                denominator,
            } => Edit::SetLength {
                length: Rational::new(*numerator, *denominator)
                    .map_err(|e| ApiError::new("T", e.to_string()))?,
            },
            GridEditDto::SetMeter { meter } => Edit::SetMeter {
                meter: meter.map(MeterDto::to_app).transpose()?,
            },
            GridEditDto::SetName { name } => Edit::SetName { name: name.clone() },
            GridEditDto::SetMeta { description, tags } => Edit::SetMeta {
                meta: mc_app::pattern::PatternMeta {
                    description: description.clone(),
                    tags: tags.clone(),
                    origin,
                },
            },
        })
    }
}

// ------------------------------------------------------------- view model

/// A row of the grid: a selector, or a user gesture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridRowDto {
    pub label: String,
    pub selector: Option<GridSelectorDto>,
    pub gesture: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridStepKindDto {
    Strike,
    Gesture,
    Hold,
    Rest,
}

/// One step on the grid. Times are fractions of a whole note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridStepDto {
    pub lane: PatternLaneDto,
    pub index: u32,
    pub onset_numerator: i64,
    pub onset_denominator: i64,
    pub end_numerator: i64,
    pub end_denominator: i64,
    pub value: DurationDto,
    /// The cell it starts in, if it starts on the grid of its beat.
    pub cell: Option<GridCellDto>,
    pub kind: GridStepKindDto,
    pub gesture: Option<i64>,
    /// Indices into [`GridViewDto::rows`] the step marks.
    pub rows: Vec<u32>,
    pub direction: Option<StrokeDto>,
    pub techniques: TechniquesDto,
    pub span_techniques: TechniquesDto,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridSpanDto {
    pub index: u32,
    pub lane: PatternLaneDto,
    pub first: u32,
    pub last: u32,
    pub technique: TechniqueKindDto,
    pub start_numerator: i64,
    pub start_denominator: i64,
    pub end_numerator: i64,
    pub end_denominator: i64,
}

/// The grid of one pattern (`mc_pattern::grid::view`). Draw time linearly: a beat is
/// `beat_numerator / beat_denominator` of a whole note and has `snaps[beat]` cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridViewDto {
    pub name: String,
    pub length_numerator: i64,
    pub length_denominator: i64,
    pub meter: Option<MeterDto>,
    pub beat_numerator: i64,
    pub beat_denominator: i64,
    pub snaps: Vec<u8>,
    pub rows: Vec<GridRowDto>,
    pub lanes: Vec<PatternLaneDto>,
    pub steps: Vec<GridStepDto>,
    pub spans: Vec<GridSpanDto>,
}

fn view_dto(name: &str, v: &GridView) -> GridViewDto {
    GridViewDto {
        name: name.to_string(),
        length_numerator: v.length.numerator(),
        length_denominator: v.length.denominator(),
        meter: v.meter.map(|m| MeterDto {
            numerator: m.numerator(),
            denominator: m.denominator(),
        }),
        beat_numerator: v.beat_length.numerator(),
        beat_denominator: v.beat_length.denominator(),
        snaps: v.snaps.clone(),
        rows: v
            .rows
            .iter()
            .map(|(row, label)| GridRowDto {
                label: label.clone(),
                selector: match row {
                    Row::Selector(s) => Some(GridSelectorDto::from_app(*s)),
                    Row::Gesture(_) => None,
                },
                gesture: match row {
                    Row::Gesture(g) => Some(g.raw() as i64),
                    Row::Selector(_) => None,
                },
            })
            .collect(),
        lanes: v
            .lanes
            .iter()
            .copied()
            .map(PatternLaneDto::from_app)
            .collect(),
        steps: v
            .steps
            .iter()
            .map(|s| GridStepDto {
                lane: PatternLaneDto::from_app(s.lane),
                index: s.index as u32,
                onset_numerator: s.onset.numerator(),
                onset_denominator: s.onset.denominator(),
                end_numerator: s.end.numerator(),
                end_denominator: s.end.denominator(),
                value: duration(s.value),
                cell: s.cell.map(|c| GridCellDto {
                    beat: c.beat,
                    parts: c.parts,
                    cell: c.cell,
                }),
                kind: match s.kind {
                    CellKind::Strike => GridStepKindDto::Strike,
                    CellKind::Gesture(_) => GridStepKindDto::Gesture,
                    CellKind::Hold => GridStepKindDto::Hold,
                    CellKind::Rest => GridStepKindDto::Rest,
                },
                gesture: match s.kind {
                    CellKind::Gesture(g) => Some(g.raw() as i64),
                    _ => None,
                },
                rows: s.rows.iter().map(|r| *r as u32).collect(),
                direction: s.direction.map(StrokeDto::from_app),
                techniques: TechniquesDto::from_app(s.techniques),
                span_techniques: TechniquesDto::from_app(s.span_techniques),
            })
            .collect(),
        spans: v
            .spans
            .iter()
            .map(|s| GridSpanDto {
                index: s.index as u32,
                lane: PatternLaneDto::from_app(s.lane),
                first: s.first as u32,
                last: s.last as u32,
                technique: TechniqueKindDto::from_app(s.technique),
                start_numerator: s.start.numerator(),
                start_denominator: s.start.denominator(),
                end_numerator: s.end.numerator(),
                end_denominator: s.end.denominator(),
            })
            .collect(),
    }
}

/// A user gesture of the library (the palette).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GestureInfoDto {
    pub id: i64,
    pub name: String,
    pub strike: GridStrikeDto,
}

/// The grid of `pattern` with `snaps` cells per beat (beat by beat; later beats repeat
/// the last entry, and an empty list means 2). Refused with `PE-14` for a bad snap and
/// `PT` for an unknown pattern.
#[frb(sync)]
pub fn pattern_grid(
    document: &DocumentHandle,
    pattern: i64,
    snaps: Vec<u8>,
) -> Result<GridViewDto, ApiError> {
    let s = document.lock_for_pattern();
    let id = PatternId::from_raw(pattern as u64);
    let v = s.pattern_grid(id, &snaps)?;
    let name = s
        .patterns()
        .pattern(id)
        .map(|p| p.name().to_string())
        .unwrap_or_default();
    Ok(view_dto(&name, &v))
}

/// The gestures of the session library, by id.
#[frb(sync)]
pub fn gestures(document: &DocumentHandle) -> Vec<GestureInfoDto> {
    let s = document.lock_for_pattern();
    s.patterns()
        .gestures()
        .map(|g| GestureInfoDto {
            id: g.id().raw() as i64,
            name: g.name().to_string(),
            strike: GridStrikeDto::from_app(g.strike()),
        })
        .collect()
}
