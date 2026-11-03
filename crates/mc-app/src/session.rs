//! Document session (score-model-and-undo section 1): one open project, single writer.
//! Owns the score, history, revision, event numbering and save state.

use crate::command::{Command, CommandEnvelope, Revision};
use crate::events::{AppEvent, DocumentId, EventEnvelope, EventPayload, EventSink};
use crate::handlers::{guitar_handlers, score_handlers, track_handlers};
use crate::history::{History, HistoryLimits};
use crate::ports::{Clock, Entropy, ProjectStore};
use crate::{CommandError, CommandId, Constraints, OpenError, SaveError, diff};
use mc_project::{AppInfo, LoadWarning, ProjectId, SaveInfo};
use mc_score::{Diagnostic, DomainEvent, Score};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    pub revision: Revision,
    pub events: Vec<EventEnvelope>,
}

/// What the user should be told after opening a file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpenReport {
    /// U1 paths; non-empty means Save over the original is disabled (U2).
    pub unknown_content: Vec<String>,
    pub newer_minor_version: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionConfig {
    pub constraints: Constraints,
    pub history: HistoryLimits,
    /// Autosave interval when dirty (default 2 minutes).
    pub autosave_interval_millis: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfig {
            constraints: Constraints::default(),
            history: HistoryLimits::default(),
            autosave_interval_millis: 120_000,
        }
    }
}

pub struct DocumentSession {
    id: DocumentId,
    score: Score,
    history: History,
    config: SessionConfig,
    revision: Revision,
    saved_revision: Option<Revision>,
    /// Score as last saved or opened. Dirty means the content differs from it, so undoing
    /// back to the saved state is clean again. Shared Arcs make the comparison cheap.
    saved_score: Option<Score>,
    seq: u64,
    location: Option<PathBuf>,
    project_id: ProjectId,
    created_by: Option<AppInfo>,
    file_revision: u64,
    /// U2: set when the opened file had content this build cannot write back.
    unknown_content: Vec<String>,
    last_autosave_millis: u64,
    sink: Option<Box<dyn EventSink>>,
    /// D-01 diagnostics of `score`, kept current incrementally on every change (a full
    /// scan of a large score costs several ms, too much per keystroke).
    diagnostics: Vec<Diagnostic>,
}

/// Full D-01 scan; overflow (not reachable with validated durations) reports nothing,
/// as before.
fn full_diagnostics(score: &Score) -> Vec<Diagnostic> {
    score
        .diagnostics(0..score.master_bar_count())
        .unwrap_or_default()
}

impl std::fmt::Debug for DocumentSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentSession")
            .field("id", &self.id)
            .field("revision", &self.revision)
            .field("saved_revision", &self.saved_revision)
            .field("location", &self.location)
            .finish_non_exhaustive()
    }
}

impl DocumentSession {
    /// A new, unsaved document.
    pub fn new(
        id: DocumentId,
        score: Score,
        config: SessionConfig,
        clock: &dyn Clock,
        entropy: &dyn Entropy,
    ) -> Self {
        let now = clock.now_unix_millis();
        DocumentSession {
            id,
            diagnostics: full_diagnostics(&score),
            score,
            history: History::new(config.history),
            config,
            revision: Revision(0),
            saved_revision: None,
            saved_score: None,
            seq: 0,
            location: None,
            project_id: ProjectId::from_parts(now, entropy.random_u128()),
            created_by: None,
            file_revision: 0,
            unknown_content: Vec::new(),
            last_autosave_millis: now,
            sink: None,
        }
    }

    /// Opens a project. Load errors are typed; unknown content is reported, not fatal.
    pub fn open(
        id: DocumentId,
        store: &dyn ProjectStore,
        path: &Path,
        config: SessionConfig,
        clock: &dyn Clock,
    ) -> Result<(Self, OpenReport), OpenError> {
        let bytes = store.read(path).map_err(OpenError::Store)?;
        Self::from_bytes(id, &bytes, Some(path.to_path_buf()), config, clock)
    }

    /// Opens an autosave snapshot after a crash (project-format section 12). The
    /// document keeps the original `location` (if it had one) so that Save writes back
    /// to it, but it starts unsaved: its content is not what is on disk there, so it is
    /// dirty and keeps autosaving until the user saves.
    pub fn recover(
        id: DocumentId,
        bytes: &[u8],
        location: Option<PathBuf>,
        config: SessionConfig,
        clock: &dyn Clock,
    ) -> Result<(Self, OpenReport), OpenError> {
        let (mut session, report) = Self::from_bytes(id, bytes, location, config, clock)?;
        session.saved_score = None;
        session.saved_revision = None;
        Ok((session, report))
    }

    /// Opens package bytes as a saved document at `location` (see [`Self::recover`]
    /// for crash recovery).
    pub fn from_bytes(
        id: DocumentId,
        bytes: &[u8],
        location: Option<PathBuf>,
        config: SessionConfig,
        clock: &dyn Clock,
    ) -> Result<(Self, OpenReport), OpenError> {
        let loaded = mc_project::load(bytes).map_err(OpenError::Load)?;
        let report = OpenReport {
            unknown_content: loaded.unknown_content().to_vec(),
            newer_minor_version: loaded
                .warnings
                .iter()
                .any(|w| matches!(w, LoadWarning::NewerMinorVersion { .. })),
        };
        let session = DocumentSession {
            id,
            diagnostics: full_diagnostics(&loaded.score),
            saved_score: Some(loaded.score.clone()),
            score: loaded.score,
            history: History::new(config.history),
            config,
            revision: Revision(0),
            saved_revision: Some(Revision(0)),
            seq: 0,
            location,
            project_id: ProjectId::from_loaded(loaded.project_id),
            created_by: Some(loaded.created_by),
            file_revision: loaded.saved_revision,
            unknown_content: report.unknown_content.clone(),
            last_autosave_millis: clock.now_unix_millis(),
            sink: None,
        };
        Ok((session, report))
    }

    pub fn set_sink(&mut self, sink: Box<dyn EventSink>) {
        self.sink = Some(sink);
    }

    pub fn take_sink(&mut self) -> Option<Box<dyn EventSink>> {
        self.sink.take()
    }

    // ------------------------------------------------------------ commands

    /// Applies one command. On error nothing changes: not the score, the revision,
    /// the history nor the event sequence.
    pub fn execute(&mut self, env: CommandEnvelope) -> Result<CommandOutcome, CommandError> {
        if let Some(expected) = env.expected_revision
            && expected != self.revision
        {
            return Err(CommandError::StaleRevision {
                expected,
                actual: self.revision,
            });
        }
        let mut next = self.score.clone(); // structural-sharing snapshot (ADR-016)
        let constraints = self.config.constraints;
        let domain = match env.command {
            Command::Score(c) => score_handlers::handle(&mut next, c, &constraints)?,
            Command::Track(c) => track_handlers::handle(&mut next, c, &constraints)?,
            Command::Guitar(c) => guitar_handlers::handle(&mut next, c, &constraints)?,
        };
        let before = std::mem::replace(&mut self.score, next);
        self.update_diagnostics(&before);
        let could_undo = self.history.can_undo();
        let could_redo = self.history.can_redo();
        self.history.record(before, &self.score, env.coalesce_key);
        Ok(self.commit(Some(env.id), domain, None, could_undo, could_redo))
    }

    pub fn undo(&mut self) -> Result<CommandOutcome, CommandError> {
        let (could_undo, could_redo) = (self.history.can_undo(), self.history.can_redo());
        let restored = self
            .history
            .undo(self.score.clone())
            .ok_or(CommandError::NothingToUndo)?;
        let events = diff::changes(&self.score, &restored);
        let before = std::mem::replace(&mut self.score, restored);
        self.update_diagnostics(&before);
        Ok(self.commit(
            None,
            events,
            Some(AppEvent::HistoryApplied),
            could_undo,
            could_redo,
        ))
    }

    pub fn redo(&mut self) -> Result<CommandOutcome, CommandError> {
        let (could_undo, could_redo) = (self.history.can_undo(), self.history.can_redo());
        let restored = self
            .history
            .redo(self.score.clone())
            .ok_or(CommandError::NothingToRedo)?;
        let events = diff::changes(&self.score, &restored);
        let before = std::mem::replace(&mut self.score, restored);
        self.update_diagnostics(&before);
        Ok(self.commit(
            None,
            events,
            Some(AppEvent::HistoryApplied),
            could_undo,
            could_redo,
        ))
    }

    fn commit(
        &mut self,
        cause: Option<CommandId>,
        domain: Vec<DomainEvent>,
        extra: Option<AppEvent>,
        could_undo: bool,
        could_redo: bool,
    ) -> CommandOutcome {
        let was_dirty = self.is_dirty();
        self.revision = Revision(self.revision.0 + 1);
        let mut payloads: Vec<EventPayload> =
            domain.into_iter().map(EventPayload::Domain).collect();
        payloads.extend(extra.map(EventPayload::App));
        if self.is_dirty() != was_dirty {
            payloads.push(EventPayload::App(AppEvent::DirtyChanged {
                dirty: self.is_dirty(),
            }));
        }
        let (can_undo, can_redo) = (self.history.can_undo(), self.history.can_redo());
        if (can_undo, can_redo) != (could_undo, could_redo) {
            payloads.push(EventPayload::App(AppEvent::UndoAvailabilityChanged {
                can_undo,
                can_redo,
            }));
        }
        self.emit(cause, payloads)
    }

    fn emit(&mut self, cause: Option<CommandId>, payloads: Vec<EventPayload>) -> CommandOutcome {
        let events: Vec<EventEnvelope> = payloads
            .into_iter()
            .map(|payload| {
                self.seq += 1;
                EventEnvelope {
                    seq: self.seq,
                    document_id: self.id,
                    revision: self.revision,
                    correlation_id: cause,
                    causation: None,
                    payload,
                }
            })
            .collect();
        if let Some(sink) = self.sink.as_mut()
            && !events.is_empty()
        {
            sink.publish(&events);
        }
        CommandOutcome {
            revision: self.revision,
            events,
        }
    }

    // --------------------------------------------------------------- saving

    /// Saves to the current location. Refused when the opened file had unknown content (U2).
    pub fn save(
        &mut self,
        store: &dyn ProjectStore,
        app: &AppInfo,
    ) -> Result<CommandOutcome, SaveError> {
        let path = self.location.clone().ok_or(SaveError::NoLocation)?;
        if !self.unknown_content.is_empty() {
            return Err(SaveError::WouldDropUnknownContent {
                paths: self.unknown_content.clone(),
            });
        }
        self.write_to(store, &path, app)
    }

    /// Saves to another location. Allowed with unknown content: the original file stays
    /// untouched, and the new file simply does not contain what this build cannot write.
    pub fn save_as(
        &mut self,
        store: &dyn ProjectStore,
        path: &Path,
        app: &AppInfo,
    ) -> Result<CommandOutcome, SaveError> {
        if self.location.as_deref() == Some(path) && !self.unknown_content.is_empty() {
            return Err(SaveError::WouldDropUnknownContent {
                paths: self.unknown_content.clone(),
            });
        }
        let out = self.write_to(store, path, app)?;
        self.location = Some(path.to_path_buf());
        self.unknown_content.clear();
        Ok(out)
    }

    fn write_to(
        &mut self,
        store: &dyn ProjectStore,
        path: &Path,
        app: &AppInfo,
    ) -> Result<CommandOutcome, SaveError> {
        let info = SaveInfo {
            project_id: self.project_id.clone(),
            created_by: self.created_by.clone().unwrap_or_else(|| app.clone()),
            saved_by: app.clone(),
            saved_revision: self.file_revision + 1,
        };
        let bytes = mc_project::save(&self.score, &info);
        store.write_atomic(path, &bytes).map_err(SaveError::Store)?;
        let was_dirty = self.is_dirty();
        self.file_revision += 1;
        self.created_by = Some(info.created_by);
        self.saved_revision = Some(self.revision);
        self.saved_score = Some(self.score.clone());
        self.history.break_coalescing();
        let mut payloads = vec![EventPayload::App(AppEvent::DocumentSaved {
            revision: self.revision,
        })];
        if was_dirty {
            payloads.push(EventPayload::App(AppEvent::DirtyChanged { dirty: false }));
        }
        Ok(self.emit(None, payloads))
    }

    /// Package bytes of the current state, for autosave (does not change save state).
    pub fn snapshot_bytes(&self, app: &AppInfo) -> Vec<u8> {
        let info = SaveInfo {
            project_id: self.project_id.clone(),
            created_by: self.created_by.clone().unwrap_or_else(|| app.clone()),
            saved_by: app.clone(),
            saved_revision: self.file_revision,
        };
        mc_project::save(&self.score, &info)
    }

    /// True when the document is dirty and the autosave interval has passed.
    pub fn autosave_due(&self, now_unix_millis: u64) -> bool {
        self.is_dirty()
            && now_unix_millis.saturating_sub(self.last_autosave_millis)
                >= self.config.autosave_interval_millis
    }

    pub fn mark_autosaved(&mut self, now_unix_millis: u64) {
        self.last_autosave_millis = now_unix_millis;
    }

    // -------------------------------------------------------------- queries

    pub fn score(&self) -> &Score {
        &self.score
    }

    /// Cheap immutable snapshot for background work (save, playback compile).
    pub fn snapshot(&self) -> Score {
        self.score.clone()
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn is_dirty(&self) -> bool {
        self.saved_score.as_ref() != Some(&self.score)
    }

    /// Revision at the last save or open, if any.
    pub fn saved_revision(&self) -> Option<Revision> {
        self.saved_revision
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn undo_depth(&self) -> usize {
        self.history.undo_len()
    }

    pub fn history_bytes(&self) -> usize {
        self.history.estimated_bytes()
    }

    pub fn location(&self) -> Option<&Path> {
        self.location.as_deref()
    }

    pub fn project_id(&self) -> &str {
        self.project_id.as_str()
    }

    /// U2 state: non-empty means Save over the original is refused.
    pub fn unknown_content(&self) -> &[String] {
        &self.unknown_content
    }

    /// D-01 diagnostics of the whole score (kept current on every change).
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics.clone()
    }

    /// `self.score` just replaced `before`; recheck only the bars that changed.
    fn update_diagnostics(&mut self, before: &Score) {
        self.diagnostics = self
            .score
            .diagnostics_since(before, &self.diagnostics)
            .unwrap_or_else(|_| full_diagnostics(&self.score));
    }
}
