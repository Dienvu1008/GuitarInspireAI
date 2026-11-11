//! Library commands (ADR-034 section 22, slice 6a-5b): pattern and gesture edits as
//! undoable session commands. Every rule is `mc-pattern`'s: the grid edit is
//! `mc_pattern::grid::edit`, and the library validates the result (PT-01, PT-03 to
//! PT-08). This module only routes the command and reports what was created.

use crate::command::{LaneLayout, PatternCommand};
use crate::error::CommandError;
use mc_pattern::grid::{Edit, GridError};
use mc_pattern::{
    Action, GestureId, Lane, Lanes, PatternError, PatternId, PatternLibrary, PatternMeta,
    PatternParts,
};

/// Why a library command was refused. Nothing changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryError {
    /// The grid edit itself is impossible (a cell, a step, a length).
    Grid(GridError),
    /// The result breaks a library rule, or names an unknown pattern or gesture.
    Rules(Vec<PatternError>),
}

impl LibraryError {
    /// `PE-14` for grid edits, `PT` for library rules.
    pub fn rule(&self) -> &'static str {
        match self {
            LibraryError::Grid(_) => "PE-14",
            LibraryError::Rules(_) => "PT",
        }
    }
}

impl std::fmt::Display for LibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LibraryError::Grid(e) => write!(f, "{e}"),
            LibraryError::Rules(errors) => {
                let first = errors.first().map(|e| e.to_string()).unwrap_or_default();
                if errors.len() > 1 {
                    write!(f, "{first} (and {} more)", errors.len() - 1)
                } else {
                    write!(f, "{first}")
                }
            }
        }
    }
}

impl From<Vec<PatternError>> for CommandError {
    fn from(e: Vec<PatternError>) -> Self {
        CommandError::Library(LibraryError::Rules(e))
    }
}

impl From<GridError> for CommandError {
    fn from(e: GridError) -> Self {
        CommandError::Library(LibraryError::Grid(e))
    }
}

/// What a library command created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LibraryOutcome {
    pub pattern: Option<PatternId>,
    pub gesture: Option<GestureId>,
}

/// True for the commands this module handles.
pub(crate) fn is_library_command(c: &PatternCommand) -> bool {
    !matches!(c, PatternCommand::ApplyPattern(_))
}

fn parts_of(lib: &PatternLibrary, id: PatternId) -> Result<PatternParts, CommandError> {
    lib.pattern(id)
        .map(|p| p.parts().clone())
        .ok_or_else(|| vec![PatternError::UnknownPattern(id)].into())
}

/// Applies a library command to a copy of the library. On success the caller replaces
/// the session library with the result.
pub(crate) fn handle(
    lib: &PatternLibrary,
    c: PatternCommand,
) -> Result<(PatternLibrary, LibraryOutcome), CommandError> {
    let mut next = lib.clone();
    let mut out = LibraryOutcome::default();
    match c {
        PatternCommand::ApplyPattern(_) => unreachable!("routed to crate::pattern::apply"),
        PatternCommand::NewPattern {
            name,
            layout,
            length,
            meter,
        } => {
            let lanes = match layout {
                LaneLayout::Strum => Lanes::Strum(Lane::default()),
                LaneLayout::ThumbFingers => Lanes::ThumbFingers {
                    thumb: Lane::default(),
                    fingers: Lane::default(),
                },
            };
            out.pattern = Some(next.define_pattern(PatternParts {
                name,
                length,
                meter,
                lanes,
                spans: Vec::new(),
                meta: PatternMeta {
                    origin: Some(mc_pattern::Origin::Grid),
                    ..PatternMeta::default()
                },
            })?);
        }
        PatternCommand::DefinePattern { parts } => {
            out.pattern = Some(next.define_pattern(parts)?);
        }
        PatternCommand::RemovePattern { pattern } => {
            next.remove_pattern(pattern)?;
        }
        PatternCommand::EditPattern { pattern, edit } => {
            let parts = parts_of(&next, pattern)?;
            let edited = mc_pattern::grid::edit(&parts, &edit)?;
            next.replace_pattern(pattern, edited)?;
        }
        PatternCommand::DefineGesture { name, strike } => {
            out.gesture = Some(next.define_gesture(&name, strike)?);
        }
        PatternCommand::GestureFromStep {
            pattern,
            lane,
            index,
            name,
        } => {
            let parts = parts_of(&next, pattern)?;
            let step = parts
                .lanes
                .get(lane)
                .ok_or(GridError::NoSuchLane { lane })?
                .steps
                .get(index)
                .ok_or(GridError::NoSuchStep { lane, index })?;
            let Action::Strike(strike) = &step.action else {
                return Err(GridError::NotAStrike { lane, index }.into());
            };
            let id = next.define_gesture(&name, strike.clone())?;
            let edited = mc_pattern::grid::edit(
                &parts,
                &Edit::SetAction {
                    lane,
                    index,
                    action: Action::Gesture(id),
                },
            )?;
            next.replace_pattern(pattern, edited)?;
            out.gesture = Some(id);
        }
        PatternCommand::RenameGesture { gesture, name } => {
            next.rename_gesture(gesture, &name)?;
        }
        PatternCommand::ChangeGesture { gesture, strike } => {
            next.change_gesture(gesture, strike)?;
        }
        PatternCommand::RemoveGesture { gesture } => {
            next.remove_gesture(gesture)?;
        }
    }
    Ok((next, out))
}
