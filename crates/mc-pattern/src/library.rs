//! The pattern library aggregate (bounded-contexts section 4): patterns and user
//! gestures, in memory (6a-2). Every pattern in it is valid against its gestures, and
//! every operation is atomic: on an error nothing changes.

use std::collections::BTreeMap;

use mc_theory::{NoteValue, Rational, TimeSignature};

use crate::error::{PatternError, StrikeAt};
use crate::ids::{GestureId, PatternId};
use crate::model::{Action, LaneId, PatternParts, Strike, StringSelector, TechniqueSpan};
use crate::names::{check_name, name_key};
use crate::validate::{Gestures, check_strike, validate};

/// A named strike template (ADR-034 section 6). Gestures never reference gestures in
/// 6a (PT-06): the template is a strike, so a cycle cannot be written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GestureDefinition {
    id: GestureId,
    name: String,
    strike: Strike,
}

impl GestureDefinition {
    pub fn id(&self) -> GestureId {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn strike(&self) -> &Strike {
        &self.strike
    }
}

/// A validated pattern of a library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternDefinition {
    id: PatternId,
    parts: PatternParts,
}

impl PatternDefinition {
    pub fn id(&self) -> PatternId {
        self.id
    }
    /// The pattern as plain data, exactly as it was defined.
    pub fn parts(&self) -> &PatternParts {
        &self.parts
    }
    pub fn name(&self) -> &str {
        &self.parts.name
    }
    /// The gestures the pattern uses, in first-use order, each once.
    pub fn gestures_used(&self) -> Vec<GestureId> {
        let mut out = Vec::new();
        for (_, lane) in self.parts.lanes.iter() {
            for step in &lane.steps {
                if let Action::Gesture(g) = step.action
                    && !out.contains(&g)
                {
                    out.push(g);
                }
            }
        }
        out
    }
}

/// A gesture as plain data (for storage and front ends).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GestureParts {
    pub id: GestureId,
    pub name: String,
    pub strike: Strike,
}

/// A whole library as plain data. [`PatternLibrary::to_parts`] lists gestures and
/// patterns in ascending id order; [`PatternLibrary::from_parts`] validates
/// everything and gives back the same parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryParts {
    /// The next id to allocate; every id in the parts is below it.
    pub next_id: u64,
    pub gestures: Vec<GestureParts>,
    pub patterns: Vec<(PatternId, PatternParts)>,
}

/// What a step does after gesture expansion (the input of binding, 6a-3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedAction {
    Strike(Strike),
    Hold(Option<Vec<StringSelector>>),
    Rest,
}

/// A step after gesture expansion; `gesture` records where a strike came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedStep {
    pub onset: Rational,
    pub duration: NoteValue,
    pub action: ResolvedAction,
    pub gesture: Option<GestureId>,
}

/// A pattern with every gesture replaced by its strike. Spans are kept as they are;
/// binding applies them (6a-3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedPattern {
    pub id: PatternId,
    pub length: Rational,
    pub meter: Option<TimeSignature>,
    pub lanes: Vec<(LaneId, Vec<ResolvedStep>)>,
    pub spans: Vec<TechniqueSpan>,
}

/// Patterns and gestures, with one id counter for both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternLibrary {
    next_id: u64,
    gestures: BTreeMap<GestureId, GestureDefinition>,
    patterns: BTreeMap<PatternId, PatternDefinition>,
}

impl Default for PatternLibrary {
    fn default() -> Self {
        PatternLibrary::new()
    }
}

impl Gestures for BTreeMap<GestureId, GestureDefinition> {
    fn strike(&self, id: GestureId) -> Option<&Strike> {
        self.get(&id).map(|g| &g.strike)
    }
}

type Res<T> = Result<T, Vec<PatternError>>;

impl PatternLibrary {
    pub fn new() -> Self {
        PatternLibrary {
            next_id: 1,
            gestures: BTreeMap::new(),
            patterns: BTreeMap::new(),
        }
    }

    pub fn gesture(&self, id: GestureId) -> Option<&GestureDefinition> {
        self.gestures.get(&id)
    }

    /// The gesture with this name, compared without case (PT-08).
    pub fn gesture_by_name(&self, name: &str) -> Option<&GestureDefinition> {
        let key = name_key(name);
        self.gestures.values().find(|g| name_key(&g.name) == key)
    }

    pub fn gestures(&self) -> impl Iterator<Item = &GestureDefinition> {
        self.gestures.values()
    }

    pub fn pattern(&self, id: PatternId) -> Option<&PatternDefinition> {
        self.patterns.get(&id)
    }

    pub fn patterns(&self) -> impl Iterator<Item = &PatternDefinition> {
        self.patterns.values()
    }

    /// Validates a pattern against this library's gestures without adding it (for
    /// editors and front ends).
    pub fn validate_pattern(&self, parts: &PatternParts) -> Vec<PatternError> {
        validate(parts, &self.gestures)
    }

    fn allocate(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// PT-08 for a name given to `except` (or to a new gesture when `None`).
    fn check_gesture_name(&self, name: &str, except: Option<GestureId>) -> Res<()> {
        if let Err(problem) = check_name(name) {
            return Err(vec![PatternError::InvalidGestureName {
                name: name.to_string(),
                problem,
            }]);
        }
        let key = name_key(name);
        if let Some(g) = self
            .gestures
            .values()
            .find(|g| Some(g.id) != except && name_key(&g.name) == key)
        {
            return Err(vec![PatternError::DuplicateGestureName {
                name: name.to_string(),
                existing: g.id,
            }]);
        }
        Ok(())
    }

    fn checked_strike(name: &str, strike: &Strike) -> Res<()> {
        let mut errors = Vec::new();
        check_strike(
            strike,
            &StrikeAt::Gesture {
                name: name.to_string(),
            },
            &mut errors,
        );
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn define_gesture(&mut self, name: &str, strike: Strike) -> Res<GestureId> {
        self.check_gesture_name(name, None)?;
        Self::checked_strike(name, &strike)?;
        let id = GestureId::from_raw(self.allocate());
        self.gestures.insert(
            id,
            GestureDefinition {
                id,
                name: name.to_string(),
                strike,
            },
        );
        Ok(id)
    }

    /// Renames a gesture. Patterns refer to it by id, so none of them changes.
    pub fn rename_gesture(&mut self, id: GestureId, name: &str) -> Res<()> {
        if !self.gestures.contains_key(&id) {
            return Err(vec![PatternError::UnknownGestureId(id)]);
        }
        self.check_gesture_name(name, Some(id))?;
        self.gestures.get_mut(&id).expect("checked").name = name.to_string();
        Ok(())
    }

    /// Changes a gesture's strike. Every pattern using it must stay valid (a hold
    /// after the gesture depends on its targets, PT-05); otherwise nothing changes.
    pub fn change_gesture(&mut self, id: GestureId, strike: Strike) -> Res<()> {
        let Some(old) = self.gestures.get(&id) else {
            return Err(vec![PatternError::UnknownGestureId(id)]);
        };
        Self::checked_strike(&old.name.clone(), &strike)?;
        let mut trial = self.gestures.clone();
        trial.get_mut(&id).expect("checked").strike = strike;
        let mut errors = Vec::new();
        for p in self.patterns.values() {
            if p.gestures_used().contains(&id) {
                let e = validate(&p.parts, &trial);
                if !e.is_empty() {
                    errors.push(PatternError::InvalidPattern {
                        pattern: p.id,
                        errors: e,
                    });
                }
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        self.gestures = trial;
        Ok(())
    }

    /// Removes a gesture no pattern uses.
    pub fn remove_gesture(&mut self, id: GestureId) -> Res<GestureDefinition> {
        if !self.gestures.contains_key(&id) {
            return Err(vec![PatternError::UnknownGestureId(id)]);
        }
        let users: Vec<PatternId> = self
            .patterns
            .values()
            .filter(|p| p.gestures_used().contains(&id))
            .map(|p| p.id)
            .collect();
        if !users.is_empty() {
            return Err(vec![PatternError::GestureInUse {
                gesture: id,
                patterns: users,
            }]);
        }
        Ok(self.gestures.remove(&id).expect("checked"))
    }

    pub fn define_pattern(&mut self, parts: PatternParts) -> Res<PatternId> {
        let errors = validate(&parts, &self.gestures);
        if !errors.is_empty() {
            return Err(errors);
        }
        let id = PatternId::from_raw(self.allocate());
        self.patterns.insert(id, PatternDefinition { id, parts });
        Ok(id)
    }

    pub fn replace_pattern(&mut self, id: PatternId, parts: PatternParts) -> Res<()> {
        if !self.patterns.contains_key(&id) {
            return Err(vec![PatternError::UnknownPattern(id)]);
        }
        let errors = validate(&parts, &self.gestures);
        if !errors.is_empty() {
            return Err(errors);
        }
        self.patterns.insert(id, PatternDefinition { id, parts });
        Ok(())
    }

    pub fn remove_pattern(&mut self, id: PatternId) -> Res<PatternDefinition> {
        self.patterns
            .remove(&id)
            .ok_or_else(|| vec![PatternError::UnknownPattern(id)])
    }

    /// The pattern with every gesture expanded to its current strike (ADR-034
    /// section 6: gestures are expanded at bind time).
    pub fn resolve(&self, id: PatternId) -> Res<ResolvedPattern> {
        let p = self
            .patterns
            .get(&id)
            .ok_or_else(|| vec![PatternError::UnknownPattern(id)])?;
        let lanes = p
            .parts
            .lanes
            .iter()
            .map(|(lane_id, lane)| {
                let steps = lane
                    .steps
                    .iter()
                    .map(|s| {
                        let (action, gesture) = match &s.action {
                            Action::Strike(k) => (ResolvedAction::Strike(k.clone()), None),
                            Action::Gesture(g) => (
                                ResolvedAction::Strike(
                                    self.gestures
                                        .get(g)
                                        .expect("library patterns are valid (PT-06)")
                                        .strike
                                        .clone(),
                                ),
                                Some(*g),
                            ),
                            Action::Hold(h) => (ResolvedAction::Hold(h.clone()), None),
                            Action::Rest => (ResolvedAction::Rest, None),
                        };
                        ResolvedStep {
                            onset: s.onset,
                            duration: s.duration,
                            action,
                            gesture,
                        }
                    })
                    .collect();
                (lane_id, steps)
            })
            .collect();
        Ok(ResolvedPattern {
            id,
            length: p.parts.length,
            meter: p.parts.meter,
            lanes,
            spans: p.parts.spans.clone(),
        })
    }

    /// The library as plain data, in ascending id order.
    pub fn to_parts(&self) -> LibraryParts {
        LibraryParts {
            next_id: self.next_id,
            gestures: self
                .gestures
                .values()
                .map(|g| GestureParts {
                    id: g.id,
                    name: g.name.clone(),
                    strike: g.strike.clone(),
                })
                .collect(),
            patterns: self
                .patterns
                .values()
                .map(|p| (p.id, p.parts.clone()))
                .collect(),
        }
    }

    /// Rebuilds a library from plain data, checking everything a library guarantees:
    /// unique ids below the counter, gesture names and strikes (PT-08), and every
    /// pattern (PT-01, PT-03 to PT-07). Returns every error found.
    pub fn from_parts(parts: LibraryParts) -> Res<Self> {
        let mut errors = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        let ids = parts
            .gestures
            .iter()
            .map(|g| g.id.raw())
            .chain(parts.patterns.iter().map(|(id, _)| id.raw()));
        for id in ids {
            if !seen.insert(id) {
                errors.push(PatternError::DuplicateId(id));
            }
            if id >= parts.next_id {
                errors.push(PatternError::IdNotBelowCounter {
                    id,
                    next_id: parts.next_id,
                });
            }
        }
        let mut lib = PatternLibrary {
            next_id: parts.next_id,
            gestures: BTreeMap::new(),
            patterns: BTreeMap::new(),
        };
        for g in parts.gestures {
            match lib
                .check_gesture_name(&g.name, None)
                .and_then(|_| Self::checked_strike(&g.name, &g.strike))
            {
                Err(e) => errors.extend(e),
                Ok(()) => {
                    lib.gestures.insert(
                        g.id,
                        GestureDefinition {
                            id: g.id,
                            name: g.name,
                            strike: g.strike,
                        },
                    );
                }
            }
        }
        for (id, p) in parts.patterns {
            let e = validate(&p, &lib.gestures);
            if e.is_empty() {
                lib.patterns.insert(id, PatternDefinition { id, parts: p });
            } else {
                errors.push(PatternError::InvalidPattern {
                    pattern: id,
                    errors: e,
                });
            }
        }
        if errors.is_empty() {
            Ok(lib)
        } else {
            Err(errors)
        }
    }
}
