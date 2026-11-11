//! Format 1.2: the pattern library (ADR-034 section 8), stored as one entry,
//! `patterns/library.json`.
//!
//! The DTOs follow the rules of `dto`: every struct captures unknown keys in `extra`
//! (U1), defaults are omitted, field order is fixed and nothing is a float, so the
//! output is deterministic. Loading checks the shape here (P-05, with JSON paths) and
//! then rebuilds the library through `PatternLibrary::from_parts`, which checks every
//! library rule (PT-01, PT-03 to PT-08): imported data never bypasses validation
//! (GR-09).

use crate::convert::{Ctx, value_dto, value_from};
use crate::dto::{Extra, StrokeDto, UnknownContent, ValueDto, extra_paths, is_false, list};
use mc_pattern::{
    Action, Direction, GestureId, GestureParts, Lane, LaneId, Lanes, LibraryParts, Origin,
    PatternId, PatternLibrary, PatternMeta, PatternParts, Step, Strike, StringSelector, Technique,
    TechniqueSet, TechniqueSpan,
};
use mc_score::Violation;
use mc_theory::TimeSignature;
use mc_theory::fretted::StringIndex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct LibraryDto {
    /// The next id to allocate. Every gesture and pattern id is below it.
    pub next_id: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gestures: Vec<GestureDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patterns: Vec<PatternDto>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct GestureDto {
    pub id: u64,
    pub name: String,
    pub strike: StrikeDto,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct PatternDto {
    pub id: u64,
    pub name: String,
    /// In whole notes, as [numerator, denominator].
    pub length: [i64; 2],
    /// [numerator, denominator] of the declared meter, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meter: Option<[u8; 2]>,
    pub lanes: LanesDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spans: Vec<SpanDto>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<OriginDto>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Either `strum`, or both `thumb` and `fingers`.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct LanesDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strum: Option<LaneDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb: Option<LaneDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingers: Option<LaneDto>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct LaneDto {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<StepDto>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct StepDto {
    /// From the pattern start, in whole notes, as [numerator, denominator].
    pub onset: [i64; 2],
    pub value: ValueDto,
    pub action: ActionDto,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionKindDto {
    Strike,
    Gesture,
    Hold,
    Rest,
}

/// `strike` carries `strike`; `gesture` carries the gesture id; `hold` carries
/// `strings` when it holds only some strings; `rest` carries nothing.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct ActionDto {
    pub kind: ActionKindDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strike: Option<StrikeDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gesture: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strings: Option<Vec<SelectorDto>>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct StrikeDto {
    pub targets: Vec<SelectorDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<StrokeDto>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub accent: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub palm_mute: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub dead: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub let_ring: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub string_percussive: bool,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectorKindDto {
    Bass,
    Alt,
    Treble,
    String,
    Strum,
    StrumRange,
}

/// `treble` carries `count`; `string` carries `string`; `strum_range` carries `from`
/// and `to`; the others carry nothing.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct SelectorDto {
    pub kind: SelectorKindDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<u8>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LaneIdDto {
    Strum,
    Thumb,
    Fingers,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TechniqueDto {
    Accent,
    PalmMute,
    Dead,
    LetRing,
    StringPercussive,
}

/// One technique over the steps `first` to `last` of a lane, inclusive.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct SpanDto {
    pub lane: LaneIdDto,
    pub first: u64,
    pub last: u64,
    pub technique: TechniqueDto,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OriginDto {
    Grid,
    Text,
    Tap,
}

// ------------------------------------------------------------------- U1

macro_rules! leaf {
    ($t:ty) => {
        impl UnknownContent for $t {
            fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
                extra_paths(&self.extra, path, out);
            }
        }
    };
}

leaf!(SpanDto);
leaf!(SelectorDto);

impl UnknownContent for StrikeDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        list(&self.targets, path, "targets", out);
    }
}

impl UnknownContent for ActionDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        if let Some(s) = &self.strike {
            s.unknown_paths(&format!("{path}.strike"), out);
        }
        if let Some(s) = &self.strings {
            list(s, path, "strings", out);
        }
    }
}

impl UnknownContent for StepDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        self.value.unknown_paths(&format!("{path}.value"), out);
        self.action.unknown_paths(&format!("{path}.action"), out);
    }
}

impl UnknownContent for LaneDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        list(&self.steps, path, "steps", out);
    }
}

impl UnknownContent for LanesDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        for (name, lane) in [
            ("strum", &self.strum),
            ("thumb", &self.thumb),
            ("fingers", &self.fingers),
        ] {
            if let Some(l) = lane {
                l.unknown_paths(&format!("{path}.{name}"), out);
            }
        }
    }
}

impl UnknownContent for PatternDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        self.lanes.unknown_paths(&format!("{path}.lanes"), out);
        list(&self.spans, path, "spans", out);
    }
}

impl UnknownContent for GestureDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        self.strike.unknown_paths(&format!("{path}.strike"), out);
    }
}

impl UnknownContent for LibraryDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        list(&self.gestures, path, "gestures", out);
        list(&self.patterns, path, "patterns", out);
    }
}

// -------------------------------------------------------------- to DTO

pub fn library_to_dto(lib: &PatternLibrary) -> LibraryDto {
    let parts = lib.to_parts();
    LibraryDto {
        next_id: parts.next_id,
        gestures: parts
            .gestures
            .iter()
            .map(|g| GestureDto {
                id: g.id.raw(),
                name: g.name.clone(),
                strike: strike_dto(&g.strike),
                extra: Extra::new(),
            })
            .collect(),
        patterns: parts
            .patterns
            .iter()
            .map(|(id, p)| pattern_dto(*id, p))
            .collect(),
        extra: Extra::new(),
    }
}

fn pattern_dto(id: PatternId, p: &PatternParts) -> PatternDto {
    let lane = |l: &Lane| LaneDto {
        steps: l.steps.iter().map(step_dto).collect(),
        extra: Extra::new(),
    };
    let lanes = match &p.lanes {
        Lanes::Strum(l) => LanesDto {
            strum: Some(lane(l)),
            thumb: None,
            fingers: None,
            extra: Extra::new(),
        },
        Lanes::ThumbFingers { thumb, fingers } => LanesDto {
            strum: None,
            thumb: Some(lane(thumb)),
            fingers: Some(lane(fingers)),
            extra: Extra::new(),
        },
    };
    PatternDto {
        id: id.raw(),
        name: p.name.clone(),
        length: [p.length.numerator(), p.length.denominator()],
        meter: p.meter.map(|m| [m.numerator(), m.denominator()]),
        lanes,
        spans: p
            .spans
            .iter()
            .map(|s| SpanDto {
                lane: match s.lane {
                    LaneId::Strum => LaneIdDto::Strum,
                    LaneId::Thumb => LaneIdDto::Thumb,
                    LaneId::Fingers => LaneIdDto::Fingers,
                },
                first: s.first as u64,
                last: s.last as u64,
                technique: match s.technique {
                    Technique::Accent => TechniqueDto::Accent,
                    Technique::PalmMute => TechniqueDto::PalmMute,
                    Technique::Dead => TechniqueDto::Dead,
                    Technique::LetRing => TechniqueDto::LetRing,
                    Technique::StringPercussive => TechniqueDto::StringPercussive,
                },
                extra: Extra::new(),
            })
            .collect(),
        description: p.meta.description.clone(),
        tags: p.meta.tags.clone(),
        origin: p.meta.origin.map(|o| match o {
            Origin::Grid => OriginDto::Grid,
            Origin::Text => OriginDto::Text,
            Origin::Tap => OriginDto::Tap,
        }),
        extra: Extra::new(),
    }
}

fn step_dto(s: &Step) -> StepDto {
    let none = ActionDto {
        kind: ActionKindDto::Rest,
        strike: None,
        gesture: None,
        strings: None,
        extra: Extra::new(),
    };
    let action = match &s.action {
        Action::Strike(k) => ActionDto {
            kind: ActionKindDto::Strike,
            strike: Some(strike_dto(k)),
            ..none
        },
        Action::Gesture(g) => ActionDto {
            kind: ActionKindDto::Gesture,
            gesture: Some(g.raw()),
            ..none
        },
        Action::Hold(sel) => ActionDto {
            kind: ActionKindDto::Hold,
            strings: sel.as_ref().map(|v| v.iter().map(selector_dto).collect()),
            ..none
        },
        Action::Rest => none,
    };
    StepDto {
        onset: [s.onset.numerator(), s.onset.denominator()],
        value: value_dto(s.duration),
        action,
        extra: Extra::new(),
    }
}

fn strike_dto(k: &Strike) -> StrikeDto {
    let t = &k.techniques;
    StrikeDto {
        targets: k.targets.iter().map(selector_dto).collect(),
        direction: k.direction.map(|d| match d {
            Direction::Down => StrokeDto::Down,
            Direction::Up => StrokeDto::Up,
        }),
        accent: t.accent,
        palm_mute: t.palm_mute,
        dead: t.dead,
        let_ring: t.let_ring,
        string_percussive: t.string_percussive,
        extra: Extra::new(),
    }
}

fn selector_dto(s: &StringSelector) -> SelectorDto {
    let plain = |kind| SelectorDto {
        kind,
        count: None,
        string: None,
        from: None,
        to: None,
        extra: Extra::new(),
    };
    match *s {
        StringSelector::Bass => plain(SelectorKindDto::Bass),
        StringSelector::Alt => plain(SelectorKindDto::Alt),
        StringSelector::Treble(n) => SelectorDto {
            count: Some(n),
            ..plain(SelectorKindDto::Treble)
        },
        StringSelector::Absolute(i) => SelectorDto {
            string: Some(i.get()),
            ..plain(SelectorKindDto::String)
        },
        StringSelector::Strum => plain(SelectorKindDto::Strum),
        StringSelector::StrumRange(a, b) => SelectorDto {
            from: Some(a.get()),
            to: Some(b.get()),
            ..plain(SelectorKindDto::StrumRange)
        },
    }
}

// ------------------------------------------------------------ from DTO

/// Why a stored library was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryLoadError {
    /// The stored shape is wrong (P-05), with JSON paths.
    Shape(Vec<Violation>),
    /// The shape is right, but the library breaks a library rule.
    Invalid(Vec<mc_pattern::PatternError>),
}

/// Rebuilds the library: shape first (all problems, with paths), then every library
/// rule through `PatternLibrary::from_parts`.
pub fn library_from_dto(dto: &LibraryDto, root: &str) -> Result<PatternLibrary, LibraryLoadError> {
    let mut cx = Ctx {
        violations: Vec::new(),
    };
    let gestures: Vec<GestureParts> = dto
        .gestures
        .iter()
        .enumerate()
        .filter_map(|(i, g)| {
            let path = format!("{root}.gestures[{i}]");
            Some(GestureParts {
                id: GestureId::from_raw(g.id),
                name: g.name.clone(),
                strike: strike_from(&mut cx, &format!("{path}.strike"), &g.strike)?,
            })
        })
        .collect();
    let patterns: Vec<(PatternId, PatternParts)> = dto
        .patterns
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let path = format!("{root}.patterns[{i}]");
            Some((PatternId::from_raw(p.id), pattern_from(&mut cx, &path, p)?))
        })
        .collect();
    if !cx.violations.is_empty() {
        return Err(LibraryLoadError::Shape(cx.violations));
    }
    PatternLibrary::from_parts(LibraryParts {
        next_id: dto.next_id,
        gestures,
        patterns,
    })
    .map_err(LibraryLoadError::Invalid)
}

fn pattern_from(cx: &mut Ctx, path: &str, p: &PatternDto) -> Option<PatternParts> {
    let length = cx.rational(format!("{path}.length"), p.length);
    let meter = match p.meter {
        None => Some(None),
        Some([n, d]) => TimeSignature::new(n, d)
            .map(Some)
            .map_err(|e| cx.fail(format!("{path}.meter"), e))
            .ok(),
    };
    let lp = format!("{path}.lanes");
    let lane = |cx: &mut Ctx, name: &str, l: &LaneDto| -> Option<Lane> {
        let steps: Vec<Option<Step>> = l
            .steps
            .iter()
            .enumerate()
            .map(|(k, s)| step_from(cx, &format!("{lp}.{name}.steps[{k}]"), s))
            .collect();
        steps
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .map(|steps| Lane { steps })
    };
    let lanes = match (&p.lanes.strum, &p.lanes.thumb, &p.lanes.fingers) {
        (Some(s), None, None) => lane(cx, "strum", s).map(Lanes::Strum),
        (None, Some(t), Some(f)) => {
            let thumb = lane(cx, "thumb", t);
            let fingers = lane(cx, "fingers", f);
            Some(Lanes::ThumbFingers {
                thumb: thumb?,
                fingers: fingers?,
            })
        }
        _ => {
            cx.fail(
                lp.clone(),
                "a pattern has either strum, or both thumb and fingers",
            );
            None
        }
    };
    let mut spans = Vec::new();
    for (i, s) in p.spans.iter().enumerate() {
        let (Ok(first), Ok(last)) = (usize::try_from(s.first), usize::try_from(s.last)) else {
            cx.fail(format!("{path}.spans[{i}]"), "step index out of range");
            continue;
        };
        spans.push(TechniqueSpan {
            lane: match s.lane {
                LaneIdDto::Strum => LaneId::Strum,
                LaneIdDto::Thumb => LaneId::Thumb,
                LaneIdDto::Fingers => LaneId::Fingers,
            },
            first,
            last,
            technique: match s.technique {
                TechniqueDto::Accent => Technique::Accent,
                TechniqueDto::PalmMute => Technique::PalmMute,
                TechniqueDto::Dead => Technique::Dead,
                TechniqueDto::LetRing => Technique::LetRing,
                TechniqueDto::StringPercussive => Technique::StringPercussive,
            },
        });
    }
    Some(PatternParts {
        name: p.name.clone(),
        length: length?,
        meter: meter?,
        lanes: lanes?,
        spans,
        meta: PatternMeta {
            description: p.description.clone(),
            tags: p.tags.clone(),
            origin: p.origin.map(|o| match o {
                OriginDto::Grid => Origin::Grid,
                OriginDto::Text => Origin::Text,
                OriginDto::Tap => Origin::Tap,
            }),
        },
    })
}

fn step_from(cx: &mut Ctx, path: &str, s: &StepDto) -> Option<Step> {
    let onset = cx.rational(format!("{path}.onset"), s.onset);
    let duration = value_from(cx, &format!("{path}.value"), &s.value);
    let action = action_from(cx, &format!("{path}.action"), &s.action);
    Some(Step {
        onset: onset?,
        duration: duration?,
        action: action?,
    })
}

fn action_from(cx: &mut Ctx, path: &str, a: &ActionDto) -> Option<Action> {
    let present = [
        ("strike", a.strike.is_some()),
        ("gesture", a.gesture.is_some()),
        ("strings", a.strings.is_some()),
    ];
    let allowed: &[&str] = match a.kind {
        ActionKindDto::Strike => &["strike"],
        ActionKindDto::Gesture => &["gesture"],
        ActionKindDto::Hold => &["strings"],
        ActionKindDto::Rest => &[],
    };
    let mut ok = true;
    for (field, is_set) in present {
        if is_set && !allowed.contains(&field) {
            cx.fail(
                format!("{path}.{field}"),
                "not allowed for this kind of action",
            );
            ok = false;
        }
    }
    let action = match a.kind {
        ActionKindDto::Strike => match &a.strike {
            Some(k) => strike_from(cx, &format!("{path}.strike"), k).map(Action::Strike),
            None => {
                cx.fail(format!("{path}.strike"), "a strike needs its strike");
                None
            }
        },
        ActionKindDto::Gesture => match a.gesture {
            Some(g) => Some(Action::Gesture(GestureId::from_raw(g))),
            None => {
                cx.fail(
                    format!("{path}.gesture"),
                    "a gesture step needs its gesture id",
                );
                None
            }
        },
        ActionKindDto::Hold => match &a.strings {
            None => Some(Action::Hold(None)),
            Some(list) => {
                selectors_from(cx, &format!("{path}.strings"), list).map(|v| Action::Hold(Some(v)))
            }
        },
        ActionKindDto::Rest => Some(Action::Rest),
    };
    if ok { action } else { None }
}

fn strike_from(cx: &mut Ctx, path: &str, k: &StrikeDto) -> Option<Strike> {
    let targets = selectors_from(cx, &format!("{path}.targets"), &k.targets)?;
    Some(Strike {
        targets,
        direction: k.direction.map(|d| match d {
            StrokeDto::Down => Direction::Down,
            StrokeDto::Up => Direction::Up,
        }),
        techniques: TechniqueSet {
            accent: k.accent,
            palm_mute: k.palm_mute,
            dead: k.dead,
            let_ring: k.let_ring,
            string_percussive: k.string_percussive,
        },
    })
}

fn selectors_from(cx: &mut Ctx, path: &str, list: &[SelectorDto]) -> Option<Vec<StringSelector>> {
    let out: Vec<Option<StringSelector>> = list
        .iter()
        .enumerate()
        .map(|(i, s)| selector_from(cx, &format!("{path}[{i}]"), s))
        .collect();
    out.into_iter().collect()
}

fn selector_from(cx: &mut Ctx, path: &str, s: &SelectorDto) -> Option<StringSelector> {
    let present = [
        ("count", s.count.is_some()),
        ("string", s.string.is_some()),
        ("from", s.from.is_some()),
        ("to", s.to.is_some()),
    ];
    let needed: &[&str] = match s.kind {
        SelectorKindDto::Treble => &["count"],
        SelectorKindDto::String => &["string"],
        SelectorKindDto::StrumRange => &["from", "to"],
        SelectorKindDto::Bass | SelectorKindDto::Alt | SelectorKindDto::Strum => &[],
    };
    let mut ok = true;
    for (field, is_set) in present {
        if is_set != needed.contains(&field) {
            let why = if is_set {
                "not allowed for this kind of selector"
            } else {
                "required for this kind of selector"
            };
            cx.fail(format!("{path}.{field}"), why);
            ok = false;
        }
    }
    if !ok {
        return None;
    }
    let mut string = |field: &str, n: Option<u8>| {
        StringIndex::new(n.unwrap_or(0))
            .map_err(|e| cx.fail(format!("{path}.{field}"), e))
            .ok()
    };
    Some(match s.kind {
        SelectorKindDto::Bass => StringSelector::Bass,
        SelectorKindDto::Alt => StringSelector::Alt,
        SelectorKindDto::Treble => StringSelector::Treble(s.count.unwrap_or(0)),
        SelectorKindDto::String => StringSelector::Absolute(string("string", s.string)?),
        SelectorKindDto::Strum => StringSelector::Strum,
        SelectorKindDto::StrumRange => {
            let from = string("from", s.from);
            let to = string("to", s.to);
            StringSelector::StrumRange(from?, to?)
        }
    })
}
