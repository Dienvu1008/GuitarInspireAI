//! Document session (score-model-and-undo section 1): one open project, single writer.
//! Owns the score, history, revision, event numbering and save state.

use crate::command::{Command, CommandEnvelope, Revision};
use crate::events::{AppEvent, DocumentId, EventEnvelope, EventPayload, EventSink};
use crate::handlers::{guitar_handlers, score_handlers, track_handlers};
use crate::history::{History, HistoryLimits};
use crate::ports::{Clock, Entropy, ProjectStore};
use crate::transport::{
    AudioPort, PlaybackState, PlayedBarInfo, TransportCommand, TransportError, TransportState,
};
use crate::{CommandError, CommandId, Constraints, OpenError, SaveError, diff};
use mc_playback::{CompileOptions, PerformanceTimeline, TempoFactor, Tick, compile};
use mc_project::{AppInfo, LoadWarning, ProjectId, SaveInfo};
use mc_score::{Diagnostic, DomainEvent, Score, TrackId};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    pub revision: Revision,
    pub events: Vec<EventEnvelope>,
}

/// A tuner reading mapped to a track's strings (ADR-029 T8): the nearest open string
/// (1-based), its target frequency, and how many cents the measured note is off
/// (positive is sharp). Produced by [`DocumentSession::tuner_match`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TunerReference {
    pub string: u8,
    pub target_hz: f32,
    pub cents: f32,
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
    /// Playback transport settings: session state, not document content (Q3, Q6). Not
    /// saved to the file, not part of undo.
    transport: TransportState,
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
            transport: TransportState::default(),
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
            transport: TransportState::default(),
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

    // ------------------------------------------------------------ transport

    /// Compiles the current snapshot for playback, applying the muted set (pruned to
    /// tracks that still exist). Pure (PB-09); called when playback (re)starts and after
    /// an edit during playback. The user tempo factor is not part of the timeline; it is
    /// a render scalar sent through [`AudioPort::set_tempo_factor`].
    fn compile_timeline(&self) -> Result<PerformanceTimeline, TransportError> {
        let mut options = CompileOptions::new();
        let existing: Vec<TrackId> = self.score.tracks().map(|t| t.id()).collect();
        options.muted = self
            .transport
            .muted()
            .iter()
            .copied()
            .filter(|t| existing.contains(t))
            .collect();
        compile(&self.score, &options)
            .map(|out| out.timeline)
            .map_err(|e| TransportError::Compile(e.to_string()))
    }

    /// Carries out a transport command through `port`. Playback is session state: no
    /// command id, no revision bump, no undo, nothing written to the file (Q3, Q6).
    /// Events are published so the UI can update the transport bar and cursor.
    pub fn transport(
        &mut self,
        cmd: TransportCommand,
        port: &mut dyn AudioPort,
    ) -> Result<CommandOutcome, TransportError> {
        let mut payloads: Vec<EventPayload> = Vec::new();
        match cmd {
            TransportCommand::Play { from_master_bar } => {
                let timeline = self.compile_timeline()?;
                let start = match from_master_bar {
                    Some(bar) => timeline
                        .first_played_start_of_master_bar(bar)
                        .ok_or(TransportError::BarOutOfRange { master_bar: bar })?,
                    None => Tick(port.position().0.max(0)),
                };
                port.load_timeline(timeline)?;
                port.set_tempo_factor(self.transport.tempo_factor())?;
                port.play(start)?;
            }
            TransportCommand::Stop => port.stop()?,
            TransportCommand::Seek { tick } => port.seek(tick)?,
            TransportCommand::SetMuted { track, muted } => {
                let changed = self.set_muted_state(track, muted);
                if changed {
                    payloads.push(EventPayload::App(AppEvent::MuteChanged { track, muted }));
                    // Recompile and swap so the change is heard without restarting while
                    // playing (Q8 releases held notes on swap).
                    if port.state() == PlaybackState::Playing {
                        let timeline = self.compile_timeline()?;
                        port.load_timeline(timeline)?;
                    }
                }
            }
            TransportCommand::SetTempoFactor { factor } => {
                if factor != self.transport.tempo_factor() {
                    self.set_tempo_factor_state(factor);
                    port.set_tempo_factor(factor)?;
                    payloads.push(EventPayload::App(AppEvent::TempoFactorChanged {
                        hundredths: factor.hundredths(),
                    }));
                }
            }
        }
        payloads.extend(self.sync_playback_state(port));
        Ok(self.emit(None, payloads))
    }

    /// After an edit while playing, recompile and swap so the engine hears the change
    /// (audio-dsp section 3 "Score edits during playback"). A no-op when stopped.
    pub fn refresh_playback_after_edit(
        &mut self,
        port: &mut dyn AudioPort,
    ) -> Result<CommandOutcome, TransportError> {
        if port.state() == PlaybackState::Playing {
            let timeline = self.compile_timeline()?;
            port.load_timeline(timeline)?;
        }
        let payloads = self.sync_playback_state(port);
        Ok(self.emit(None, payloads))
    }

    fn set_muted_state(&mut self, track: TrackId, muted: bool) -> bool {
        if muted {
            self.transport.set_muted_insert(track)
        } else {
            self.transport.set_muted_remove(track)
        }
    }

    fn set_tempo_factor_state(&mut self, factor: TempoFactor) {
        self.transport.set_tempo_factor(factor);
    }

    /// Reads the engine state through the port and, if it changed, returns a
    /// `PlaybackStateChanged` payload. Keeps the session's mirror current.
    fn sync_playback_state(&mut self, port: &dyn AudioPort) -> Vec<EventPayload> {
        let state = port.state();
        if self.transport.set_state(state) {
            vec![EventPayload::App(AppEvent::PlaybackStateChanged { state })]
        } else {
            Vec::new()
        }
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

    /// Current playback transport settings (muted tracks, tempo factor, last state).
    /// Session state, not document content.
    pub fn transport_state(&self) -> &TransportState {
        &self.transport
    }

    /// Maps a measured frequency to the nearest open string of `track` and the cents off,
    /// using that track's tuning and capo (ADR-029 T8). The reference-pitch rule lives in
    /// the Guitar context (`mc_guitar`, GR-05); this reads the track and delegates. When
    /// the track changes tuning or capo, the next call reflects it with no restart.
    /// Returns `None` for an unknown track or a non-positive frequency.
    pub fn tuner_match(&self, track: TrackId, hz: f32) -> Option<TunerReference> {
        let t = self.score.track(track)?;
        let fretted = t.fretted();
        let targets = mc_guitar::tuner_targets(&fretted.tuning, fretted.capo);
        mc_guitar::nearest(&targets, hz).map(|m| TunerReference {
            string: m.string.get(),
            target_hz: m.target_hz,
            cents: m.cents,
        })
    }

    /// Played-bar layout of the current score for the playback cursor: each bar's score
    /// index, start tick and length after repeat and volta expansion (ADR-028). Timing
    /// is independent of mute (PB-04), so this compiles without a mute set. Returns an
    /// empty vector if compilation fails. The UI maps a playback tick to an x position
    /// on the TAB from this table.
    pub fn played_bars(&self) -> Vec<PlayedBarInfo> {
        match compile(&self.score, &CompileOptions::new()) {
            Ok(out) => out
                .timeline
                .played_bars()
                .iter()
                .map(|b| PlayedBarInfo {
                    order: b.order,
                    master_bar: b.master_bar,
                    start_tick: b.start.0,
                    length_ticks: b.length.0,
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// `self.score` just replaced `before`; recheck only the bars that changed.
    fn update_diagnostics(&mut self, before: &Score) {
        self.diagnostics = self
            .score
            .diagnostics_since(before, &self.diagnostics)
            .unwrap_or_else(|_| full_diagnostics(&self.score));
    }
}
