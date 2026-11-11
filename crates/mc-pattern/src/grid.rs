//! The grid editor front end (ADR-034 section 9, slice 6a-5b).
//!
//! Every grid edit is a pure function from a pattern to a new pattern, or a typed
//! error naming the step, span or cell. The library then validates the result as for
//! any other front end (PT-01, PT-03 to PT-08), so an edit that would break a rule
//! changes nothing. The grid's music rules live here and not in the UI (GR-05):
//!
//! - **Beats and snap.** A beat is one unit of the pattern's meter (1/4 without a
//!   meter). Each beat is split into `parts` cells: 1, 2, 3, 4, 6 or 8. Three or six
//!   parts give triplet values.
//! - **Values.** A step placed or resized over `cells` cells gets exactly one written
//!   value of that length (plain, dotted up to three dots, or triplet). A length that
//!   no single value writes is refused (`UnwritableLength`), never rounded.
//! - **Spans follow their steps.** Inserting a step inside a span widens it; deleting
//!   steps narrows or removes the spans over them; indices after a change move with it.
//! - **Order.** Moving a step never passes another step of its lane (PT-03 is about
//!   order, and a span is a run of steps in order).

use crate::{
    Action, Direction, GestureId, Lane, LaneId, Lanes, PatternLibrary, PatternMeta, PatternParts,
    Step, StringSelector, Technique, TechniqueSpan,
};
use mc_theory::{BaseValue, NoteValue, Rational, TimeSignature, Tuplet};

/// The allowed numbers of cells per beat.
pub const SNAPS: [u8; 6] = [1, 2, 3, 4, 6, 8];

/// A cell of the grid: `cell` of `parts` cells of beat `beat` (from 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cell {
    pub beat: u32,
    pub parts: u8,
    pub cell: u8,
}

/// A grid edit. Indices are step indices of the lane, from 0, in onset order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    /// Inserts a step at `at`, `cells` cells long in the snap of `at`.
    Place {
        lane: LaneId,
        at: Cell,
        cells: u8,
        action: Action,
    },
    /// Moves a step to start at `to`, keeping its value.
    Move {
        lane: LaneId,
        index: usize,
        to: Cell,
    },
    /// Gives a step the value of `cells` cells of `parts` per beat.
    Resize {
        lane: LaneId,
        index: usize,
        parts: u8,
        cells: u8,
    },
    /// Deletes the steps `first..=last` of a lane.
    Delete {
        lane: LaneId,
        first: usize,
        last: usize,
    },
    /// Replaces what a step does.
    SetAction {
        lane: LaneId,
        index: usize,
        action: Action,
    },
    /// Adds the selector to a strike's targets, or removes it if it is there.
    ToggleTarget {
        lane: LaneId,
        index: usize,
        selector: StringSelector,
    },
    /// Sets the stroke direction of every strike in `first..=last`.
    SetDirection {
        lane: LaneId,
        first: usize,
        last: usize,
        direction: Option<Direction>,
    },
    /// Sets or clears one technique flag of a strike.
    SetTechnique {
        lane: LaneId,
        index: usize,
        technique: Technique,
        on: bool,
    },
    /// A technique over a run of steps (a span, PT-07).
    AddSpan {
        lane: LaneId,
        first: usize,
        last: usize,
        technique: Technique,
    },
    /// Removes the span at `index` of the pattern's span list.
    RemoveSpan {
        index: usize,
    },
    SetLength {
        length: Rational,
    },
    SetMeter {
        meter: Option<TimeSignature>,
    },
    SetName {
        name: String,
    },
    SetMeta {
        meta: PatternMeta,
    },
}

/// Why a grid edit was refused. Nothing changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridError {
    /// `parts` is not one of [`SNAPS`].
    BadSnap {
        parts: u8,
    },
    /// The cell index is not below `parts`, or a length of 0 cells.
    BadCell {
        cell: Cell,
    },
    /// The position does not fit the rational range.
    TimeOverflow,
    /// No single written value has this length.
    UnwritableLength {
        length: Rational,
    },
    NoSuchLane {
        lane: LaneId,
    },
    NoSuchStep {
        lane: LaneId,
        index: usize,
    },
    /// The run `first..=last` is empty or reversed.
    BadRun {
        first: usize,
        last: usize,
    },
    /// The step is not a strike, and the edit needs one.
    NotAStrike {
        lane: LaneId,
        index: usize,
    },
    /// Moving the step would pass another step of the lane.
    WouldReorder {
        lane: LaneId,
        index: usize,
    },
    /// Removing this target would leave the strike without one.
    LastTarget {
        lane: LaneId,
        index: usize,
    },
    NoSuchSpan {
        index: usize,
    },
    /// The run has no strike, so a direction changes nothing.
    NoStrikeInRun {
        lane: LaneId,
        first: usize,
        last: usize,
    },
}

impl std::fmt::Display for GridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridError::BadSnap { parts } => {
                write!(
                    f,
                    "a beat can be split into 1, 2, 3, 4, 6 or 8 cells, not {parts}"
                )
            }
            GridError::BadCell { cell } => write!(
                f,
                "no cell {} of {} in beat {}",
                cell.cell + 1,
                cell.parts,
                cell.beat + 1
            ),
            GridError::TimeOverflow => write!(f, "the position is out of range"),
            GridError::UnwritableLength { length } => {
                write!(f, "no single note value is {length} of a whole note long")
            }
            GridError::NoSuchLane { lane } => write!(f, "the pattern has no {lane:?} lane"),
            GridError::NoSuchStep { lane, index } => {
                write!(f, "the {lane:?} lane has no step {}", index + 1)
            }
            GridError::BadRun { first, last } => {
                write!(f, "steps {} to {} are not a run", first + 1, last + 1)
            }
            GridError::NotAStrike { lane, index } => {
                write!(f, "step {} of the {lane:?} lane is not a strike", index + 1)
            }
            GridError::WouldReorder { lane, index } => write!(
                f,
                "step {} of the {lane:?} lane cannot pass another step",
                index + 1
            ),
            GridError::LastTarget { lane, index } => write!(
                f,
                "step {} of the {lane:?} lane needs at least one target",
                index + 1
            ),
            GridError::NoSuchSpan { index } => write!(f, "no span {}", index + 1),
            GridError::NoStrikeInRun { lane, first, last } => write!(
                f,
                "steps {} to {} of the {lane:?} lane have no strike",
                first + 1,
                last + 1
            ),
        }
    }
}

type Res<T> = Result<T, GridError>;

/// The length of one beat: one unit of the meter, or a quarter without a meter.
pub fn beat_length(meter: Option<TimeSignature>) -> Rational {
    let d = meter.map_or(4, |m| m.denominator());
    Rational::new(1, i64::from(d)).expect("non-zero")
}

fn check_snap(parts: u8) -> Res<()> {
    if SNAPS.contains(&parts) {
        Ok(())
    } else {
        Err(GridError::BadSnap { parts })
    }
}

/// The length of `cells` cells of `parts` per beat.
pub fn cells_length(meter: Option<TimeSignature>, parts: u8, cells: u8) -> Res<Rational> {
    check_snap(parts)?;
    beat_length(meter)
        .checked_mul(Rational::new(i64::from(cells), i64::from(parts)).expect("non-zero"))
        .ok_or(GridError::TimeOverflow)
}

/// Where a cell starts, from the pattern start.
pub fn cell_onset(meter: Option<TimeSignature>, at: Cell) -> Res<Rational> {
    check_snap(at.parts)?;
    if at.cell >= at.parts {
        return Err(GridError::BadCell { cell: at });
    }
    let beats = Rational::from_int(i64::from(at.beat))
        .checked_add(Rational::new(i64::from(at.cell), i64::from(at.parts)).expect("non-zero"))
        .ok_or(GridError::TimeOverflow)?;
    beat_length(meter)
        .checked_mul(beats)
        .ok_or(GridError::TimeOverflow)
}

/// The one written value of this length: plain, dotted (up to three dots) or an
/// undotted triplet. A dotted triplet is a valid value but never what a grid cell
/// means (5/16 would be a triple-dotted triplet quarter), so the grid does not make
/// one. Nothing is ever rounded.
pub fn single_value(length: Rational) -> Option<NoteValue> {
    let kinds = [(None, 3), (Some(Tuplet::TRIPLET), 0)];
    for (tuplet, max_dots) in kinds {
        for dots in 0..=max_dots {
            for e in 0..=8 {
                let Ok(base) = BaseValue::from_exponent(e) else {
                    continue;
                };
                if let Ok(v) = NoteValue::new(base, dots, tuplet)
                    && v.length() == length
                {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn value_of(meter: Option<TimeSignature>, parts: u8, cells: u8) -> Res<NoteValue> {
    if cells == 0 {
        return Err(GridError::BadCell {
            cell: Cell {
                beat: 0,
                parts,
                cell: 0,
            },
        });
    }
    let length = cells_length(meter, parts, cells)?;
    single_value(length).ok_or(GridError::UnwritableLength { length })
}

fn lane_mut(parts: &mut PatternParts, lane: LaneId) -> Res<&mut Lane> {
    match (&mut parts.lanes, lane) {
        (Lanes::Strum(l), LaneId::Strum) => Ok(l),
        (Lanes::ThumbFingers { thumb, .. }, LaneId::Thumb) => Ok(thumb),
        (Lanes::ThumbFingers { fingers, .. }, LaneId::Fingers) => Ok(fingers),
        _ => Err(GridError::NoSuchLane { lane }),
    }
}

fn step_mut(parts: &mut PatternParts, lane: LaneId, index: usize) -> Res<&mut Step> {
    lane_mut(parts, lane)?
        .steps
        .get_mut(index)
        .ok_or(GridError::NoSuchStep { lane, index })
}

fn check_run(parts: &mut PatternParts, lane: LaneId, first: usize, last: usize) -> Res<()> {
    if first > last {
        return Err(GridError::BadRun { first, last });
    }
    let n = lane_mut(parts, lane)?.steps.len();
    if last >= n {
        return Err(GridError::NoSuchStep { lane, index: last });
    }
    Ok(())
}

fn strike_mut(parts: &mut PatternParts, lane: LaneId, index: usize) -> Res<&mut crate::Strike> {
    match &mut step_mut(parts, lane, index)?.action {
        Action::Strike(k) => Ok(k),
        _ => Err(GridError::NotAStrike { lane, index }),
    }
}

/// Applies one grid edit. The result is not yet validated: the library does that.
pub fn edit(parts: &PatternParts, e: &Edit) -> Res<PatternParts> {
    let mut p = parts.clone();
    let meter = p.meter;
    match e {
        Edit::Place {
            lane,
            at,
            cells,
            action,
        } => {
            let onset = cell_onset(meter, *at)?;
            let duration = value_of(meter, at.parts, *cells)?;
            let steps = &mut lane_mut(&mut p, *lane)?.steps;
            let pos = steps.partition_point(|s| s.onset <= onset);
            steps.insert(
                pos,
                Step {
                    onset,
                    duration,
                    action: action.clone(),
                },
            );
            for s in p.spans.iter_mut().filter(|s| s.lane == *lane) {
                if s.first >= pos {
                    s.first += 1;
                    s.last += 1;
                } else if s.last >= pos {
                    s.last += 1; // inserted inside the run: the span widens
                }
            }
        }
        Edit::Move { lane, index, to } => {
            let onset = cell_onset(meter, *to)?;
            let steps = &mut lane_mut(&mut p, *lane)?.steps;
            if *index >= steps.len() {
                return Err(GridError::NoSuchStep {
                    lane: *lane,
                    index: *index,
                });
            }
            let before_ok = *index == 0 || steps[*index - 1].onset < onset;
            let after_ok = steps.get(*index + 1).is_none_or(|n| onset < n.onset);
            if !before_ok || !after_ok {
                return Err(GridError::WouldReorder {
                    lane: *lane,
                    index: *index,
                });
            }
            steps[*index].onset = onset;
        }
        Edit::Resize {
            lane,
            index,
            parts,
            cells,
        } => {
            let duration = value_of(meter, *parts, *cells)?;
            step_mut(&mut p, *lane, *index)?.duration = duration;
        }
        Edit::Delete { lane, first, last } => {
            check_run(&mut p, *lane, *first, *last)?;
            let n = last - first + 1;
            lane_mut(&mut p, *lane)?.steps.drain(*first..=*last);
            p.spans.retain_mut(|s| {
                if s.lane != *lane || s.last < *first {
                    return true;
                }
                if s.first > *last {
                    s.first -= n;
                    s.last -= n;
                    return true;
                }
                // The span overlaps the deleted run: keep what is left of it.
                let keep_before = s.first < *first;
                let keep_after = s.last > *last;
                match (keep_before, keep_after) {
                    (false, false) => false,
                    (true, false) => {
                        s.last = first - 1;
                        true
                    }
                    (false, true) => {
                        s.first = *first;
                        s.last -= n;
                        true
                    }
                    (true, true) => {
                        s.last -= n;
                        true
                    }
                }
            });
        }
        Edit::SetAction {
            lane,
            index,
            action,
        } => {
            step_mut(&mut p, *lane, *index)?.action = action.clone();
        }
        Edit::ToggleTarget {
            lane,
            index,
            selector,
        } => {
            let k = strike_mut(&mut p, *lane, *index)?;
            if let Some(i) = k.targets.iter().position(|t| t == selector) {
                if k.targets.len() == 1 {
                    return Err(GridError::LastTarget {
                        lane: *lane,
                        index: *index,
                    });
                }
                k.targets.remove(i);
            } else {
                k.targets.push(*selector);
            }
        }
        Edit::SetDirection {
            lane,
            first,
            last,
            direction,
        } => {
            check_run(&mut p, *lane, *first, *last)?;
            let mut any = false;
            for s in &mut lane_mut(&mut p, *lane)?.steps[*first..=*last] {
                if let Action::Strike(k) = &mut s.action {
                    k.direction = *direction;
                    any = true;
                }
            }
            if !any {
                return Err(GridError::NoStrikeInRun {
                    lane: *lane,
                    first: *first,
                    last: *last,
                });
            }
        }
        Edit::SetTechnique {
            lane,
            index,
            technique,
            on,
        } => {
            let t = &mut strike_mut(&mut p, *lane, *index)?.techniques;
            let flag = match technique {
                Technique::Accent => &mut t.accent,
                Technique::PalmMute => &mut t.palm_mute,
                Technique::Dead => &mut t.dead,
                Technique::LetRing => &mut t.let_ring,
                Technique::StringPercussive => &mut t.string_percussive,
            };
            *flag = *on;
        }
        Edit::AddSpan {
            lane,
            first,
            last,
            technique,
        } => {
            check_run(&mut p, *lane, *first, *last)?;
            p.spans.push(TechniqueSpan {
                lane: *lane,
                first: *first,
                last: *last,
                technique: *technique,
            });
        }
        Edit::RemoveSpan { index } => {
            if *index >= p.spans.len() {
                return Err(GridError::NoSuchSpan { index: *index });
            }
            p.spans.remove(*index);
        }
        Edit::SetLength { length } => p.length = *length,
        Edit::SetMeter { meter } => p.meter = *meter,
        Edit::SetName { name } => p.name = name.clone(),
        Edit::SetMeta { meta } => p.meta = meta.clone(),
    }
    Ok(p)
}

// ------------------------------------------------------------- view model

/// A row of the grid: what a strike targets, or a user gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Row {
    Selector(StringSelector),
    Gesture(GestureId),
}

/// The label of a selector, as the text syntax writes it.
pub fn selector_label(s: &StringSelector) -> String {
    match s {
        StringSelector::Bass => "Bass".into(),
        StringSelector::Alt => "Alt".into(),
        StringSelector::Treble(3) => "Treble".into(),
        StringSelector::Treble(n) => format!("Treble({n})"),
        StringSelector::Absolute(i) => format!("String {}", i.get()),
        StringSelector::Strum => "Strum".into(),
        StringSelector::StrumRange(a, b) => format!("Strum({}..{})", a.get(), b.get()),
    }
}

/// What a step does, for the grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellKind {
    Strike,
    Gesture(GestureId),
    /// Holds every string still held (`None`), or these rows.
    Hold,
    Rest,
}

/// One step on the grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridStep {
    pub lane: LaneId,
    pub index: usize,
    pub onset: Rational,
    pub end: Rational,
    pub value: NoteValue,
    /// The cell it starts in, if it starts on the grid of its beat.
    pub cell: Option<Cell>,
    pub kind: CellKind,
    /// Indices into [`GridView::rows`] the step marks: its targets, a gesture's row,
    /// a partial hold's strings. Empty for a rest and for a full hold.
    pub rows: Vec<usize>,
    pub direction: Option<Direction>,
    /// The step's own technique flags (a gesture's come from the gesture).
    pub techniques: crate::TechniqueSet,
    /// Techniques from spans over the step.
    pub span_techniques: crate::TechniqueSet,
}

/// A span band over a run of steps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridSpan {
    pub index: usize,
    pub lane: LaneId,
    pub first: usize,
    pub last: usize,
    pub technique: Technique,
    pub start: Rational,
    pub end: Rational,
}

/// The grid of one pattern: beats with their snap, rows, lanes, steps and spans.
/// Positions are exact rationals of a whole note; a beat is `beat_length` long and the
/// UI draws time linearly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridView {
    pub length: Rational,
    pub meter: Option<TimeSignature>,
    pub beat_length: Rational,
    /// The number of cells of each beat (the snap per beat).
    pub snaps: Vec<u8>,
    pub rows: Vec<(Row, String)>,
    pub lanes: Vec<LaneId>,
    pub steps: Vec<GridStep>,
    pub spans: Vec<GridSpan>,
}

fn union(a: crate::TechniqueSet, b: crate::TechniqueSet) -> crate::TechniqueSet {
    crate::TechniqueSet {
        let_ring: a.let_ring || b.let_ring,
        dead: a.dead || b.dead,
        accent: a.accent || b.accent,
        palm_mute: a.palm_mute || b.palm_mute,
        string_percussive: a.string_percussive || b.string_percussive,
    }
}

fn flag(t: Technique) -> crate::TechniqueSet {
    let mut s = crate::TechniqueSet::default();
    match t {
        Technique::Accent => s.accent = true,
        Technique::PalmMute => s.palm_mute = true,
        Technique::Dead => s.dead = true,
        Technique::LetRing => s.let_ring = true,
        Technique::StringPercussive => s.string_percussive = true,
    }
    s
}

/// The grid of `parts`. `snaps` gives the cells per beat, beat by beat; beats after its
/// end use its last entry, or 2 when it is empty. An invalid snap is refused.
pub fn view(parts: &PatternParts, lib: &PatternLibrary, snaps: &[u8]) -> Res<GridView> {
    for s in snaps {
        check_snap(*s)?;
    }
    let beat = beat_length(parts.meter);
    let beats = parts
        .length
        .checked_div(beat)
        .ok_or(GridError::TimeOverflow)?;
    // Whole beats, rounded up: a pattern may end inside a beat.
    let count = (beats.numerator() + beats.denominator() - 1) / beats.denominator();
    let count = usize::try_from(count.max(1)).unwrap_or(1);
    let fallback = snaps.last().copied().unwrap_or(2);
    let snaps: Vec<u8> = (0..count)
        .map(|i| snaps.get(i).copied().unwrap_or(fallback))
        .collect();

    // Rows: the standard targets, every other selector the pattern uses, the gestures.
    let mut rows: Vec<Row> = vec![
        Row::Selector(StringSelector::Bass),
        Row::Selector(StringSelector::Alt),
        Row::Selector(StringSelector::TREBLE),
    ];
    let max_string = parts
        .lanes
        .iter()
        .flat_map(|(_, l)| l.steps.iter())
        .flat_map(|s| match &s.action {
            Action::Strike(k) => k.targets.clone(),
            Action::Hold(Some(v)) => v.clone(),
            _ => Vec::new(),
        })
        .filter_map(|t| match t {
            StringSelector::Absolute(i) => Some(i.get()),
            StringSelector::StrumRange(a, b) => Some(a.get().max(b.get())),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        .max(6);
    for n in 1..=max_string {
        rows.push(Row::Selector(StringSelector::Absolute(
            mc_theory::fretted::StringIndex::new(n).expect("from 1"),
        )));
    }
    rows.push(Row::Selector(StringSelector::Strum));
    let mut extra: Vec<Row> = Vec::new();
    for (_, l) in parts.lanes.iter() {
        for s in &l.steps {
            let targets = match &s.action {
                Action::Strike(k) => k.targets.clone(),
                Action::Hold(Some(v)) => v.clone(),
                _ => Vec::new(),
            };
            for t in targets {
                let r = Row::Selector(t);
                if !rows.contains(&r) && !extra.contains(&r) {
                    extra.push(r);
                }
            }
        }
    }
    // Other Treble(n) and Strum ranges sit before the strings, in first-use order.
    rows.splice(3..3, extra);
    rows.extend(lib.gestures().map(|g| Row::Gesture(g.id())));
    let labels: Vec<(Row, String)> = rows
        .iter()
        .map(|r| {
            let label = match r {
                Row::Selector(s) => selector_label(s),
                Row::Gesture(g) => lib
                    .gesture(*g)
                    .map(|d| d.name().to_string())
                    .unwrap_or_default(),
            };
            (*r, label)
        })
        .collect();
    let row_of = |r: Row| rows.iter().position(|x| *x == r);

    let mut steps = Vec::new();
    for (lane, l) in parts.lanes.iter() {
        for (index, s) in l.steps.iter().enumerate() {
            let (kind, marked, direction, techniques) = match &s.action {
                Action::Strike(k) => (
                    CellKind::Strike,
                    k.targets
                        .iter()
                        .filter_map(|t| row_of(Row::Selector(*t)))
                        .collect(),
                    k.direction,
                    k.techniques,
                ),
                Action::Gesture(g) => {
                    let def = lib.gesture(*g);
                    (
                        CellKind::Gesture(*g),
                        row_of(Row::Gesture(*g)).into_iter().collect(),
                        def.and_then(|d| d.strike().direction),
                        def.map(|d| d.strike().techniques).unwrap_or_default(),
                    )
                }
                Action::Hold(sel) => (
                    CellKind::Hold,
                    sel.iter()
                        .flatten()
                        .filter_map(|t| row_of(Row::Selector(*t)))
                        .collect(),
                    None,
                    crate::TechniqueSet::default(),
                ),
                Action::Rest => (CellKind::Rest, Vec::new(), None, Default::default()),
            };
            let span_techniques = parts
                .spans
                .iter()
                .filter(|sp| sp.lane == lane && sp.first <= index && index <= sp.last)
                .fold(crate::TechniqueSet::default(), |acc, sp| {
                    union(acc, flag(sp.technique))
                });
            let end = s
                .onset
                .checked_add(s.duration.length())
                .ok_or(GridError::TimeOverflow)?;
            steps.push(GridStep {
                lane,
                index,
                onset: s.onset,
                end,
                value: s.duration,
                cell: cell_at(parts.meter, &snaps, s.onset),
                kind,
                rows: marked,
                direction,
                techniques,
                span_techniques,
            });
        }
    }
    let spans = parts
        .spans
        .iter()
        .enumerate()
        .filter_map(|(index, sp)| {
            let lane = parts.lanes.get(sp.lane)?;
            let first = lane.steps.get(sp.first)?;
            let last = lane.steps.get(sp.last)?;
            Some(GridSpan {
                index,
                lane: sp.lane,
                first: sp.first,
                last: sp.last,
                technique: sp.technique,
                start: first.onset,
                end: last.onset.checked_add(last.duration.length())?,
            })
        })
        .collect();
    Ok(GridView {
        length: parts.length,
        meter: parts.meter,
        beat_length: beat,
        snaps,
        rows: labels,
        lanes: parts.lanes.iter().map(|(id, _)| id).collect(),
        steps,
        spans,
    })
}

/// The cell `onset` starts, if it is on the grid of its beat.
fn cell_at(meter: Option<TimeSignature>, snaps: &[u8], onset: Rational) -> Option<Cell> {
    let beats = onset.checked_div(beat_length(meter))?;
    let beat = beats.numerator().checked_div(beats.denominator())?;
    let parts = *snaps.get(usize::try_from(beat).ok()?)?;
    let within = beats.checked_sub(Rational::from_int(beat))?;
    let cells = within.checked_mul(Rational::from_int(i64::from(parts)))?;
    (cells.denominator() == 1).then(|| Cell {
        beat: u32::try_from(beat).unwrap_or(u32::MAX),
        parts,
        cell: u8::try_from(cells.numerator()).unwrap_or(u8::MAX),
    })
}
