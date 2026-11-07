//! Document API: one opaque handle per open document (interop-architecture section 6).
//!
//! Functions without `#[frb(sync)]` run on the bridge's worker threads and return a
//! `Future` in Dart. The session is guarded by a mutex that is held only for the
//! duration of one call; real-time threads never touch it (ADR-007).

use crate::api::types::*;
use crate::frb_generated::StreamSink;
use flutter_rust_bridge::frb;
use mc_app::infra::{FsProjectStore, SystemClock, SystemEntropy, cleanup_temp_files};
use mc_app::ports::RecoveryStore;
use mc_app::types::{AppInfo, DiagnosticKind, DomainEvent, MasterBar, TimeSignature};
use mc_app::{
    AppEvent, Command, CommandEnvelope, CommandOutcome, DocumentId, DocumentSession, EventEnvelope,
    EventPayload, EventSink, GuitarCommand, Revision, ScoreCommand, SessionConfig, TrackCommand,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// The newest autosave of one project, found on startup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryEntryDto {
    pub project_id: String,
    /// Path of the recovery file (internal app data).
    pub path: String,
    /// Original file of the document; `None` for a document that was never saved.
    pub source: Option<String>,
    /// Unix millis when the autosave was written.
    pub timestamp: i64,
}

fn recovery_entry_dto(e: mc_app::ports::RecoveryEntry) -> RecoveryEntryDto {
    RecoveryEntryDto {
        project_id: e.project_id,
        path: e.path.to_string_lossy().into_owned(),
        source: e.source.map(|p| p.to_string_lossy().into_owned()),
        timestamp: e.written_unix_millis as i64,
    }
}

static NEXT_DOCUMENT: AtomicU64 = AtomicU64::new(1);

fn app_info() -> AppInfo {
    AppInfo {
        app: "GuitarInspireAI".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// An open document. Cloning the Dart handle shares the same session.
#[frb(opaque)]
#[derive(Clone)]
pub struct DocumentHandle {
    session: Arc<Mutex<DocumentSession>>,
    next_command: Arc<AtomicU64>,
    /// Recovery directory this document autosaves to (set by `write_autosave` and
    /// `open_from_recovery`). A successful save discards the document's recovery files
    /// there (project-format section 12). Lock order: `session` before this.
    recovery_dir: Arc<Mutex<Option<PathBuf>>>,
}

impl std::fmt::Debug for DocumentHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentHandle").finish_non_exhaustive()
    }
}

/// Publishes session events to a Dart stream. Dropping it (unsubscribe or a new
/// subscription) closes the Dart stream (ADR-005 amendment 1: Rust owns the lifecycle).
struct DartEventSink(StreamSink<EventDto>);

impl EventSink for DartEventSink {
    fn publish(&mut self, events: &[EventEnvelope]) {
        for e in events {
            // A closed Dart side is not an error for the document (interop section 4).
            let _ = self.0.add(event_dto(e));
        }
    }
}

impl DocumentHandle {
    fn wrap(session: DocumentSession) -> Self {
        DocumentHandle {
            session: Arc::new(Mutex::new(session)),
            next_command: Arc::new(AtomicU64::new(1)),
            recovery_dir: Arc::new(Mutex::new(None)),
        }
    }

    /// Wraps a score built by an adapter (for example MIDI import, ADR-023) as a new,
    /// unsaved document. The validated `Score` already satisfies every hard invariant.
    pub(crate) fn from_imported_score(score: mc_app::types::Score) -> Self {
        Self::wrap(DocumentSession::new(
            DocumentId(NEXT_DOCUMENT.fetch_add(1, Ordering::Relaxed)),
            score,
            SessionConfig::default(),
            &SystemClock,
            &SystemEntropy,
        ))
    }

    /// A cheap immutable snapshot of the current score, for background work such as MIDI
    /// export (ADR-023 item 3 compiles it without a mute set).
    pub(crate) fn score_snapshot(&self) -> mc_app::types::Score {
        self.lock().snapshot()
    }

    fn lock(&self) -> MutexGuard<'_, DocumentSession> {
        // A panic while holding the lock leaves the session as it was before the
        // failed call (commands check before mutating), so recovering it is safe.
        self.session.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Locks the session for a transport call driven by the engine (ADR-028). Same lock
    /// as [`Self::lock`]; a distinct name documents the audio entry point.
    pub(crate) fn lock_for_transport(&self) -> MutexGuard<'_, DocumentSession> {
        self.lock()
    }

    fn recovery_dir(&self) -> MutexGuard<'_, Option<PathBuf>> {
        self.recovery_dir.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// After a successful save the recovery files of this project are obsolete. A
    /// failure to delete them is not a save failure: the next startup offers them
    /// again, which is safe.
    fn discard_own_recovery(&self, s: &DocumentSession) {
        if let Some(dir) = self.recovery_dir().as_ref() {
            let _ = mc_app::infra::FsRecoveryStore::new(dir.clone()).discard(s.project_id());
        }
    }

    /// A new, unsaved document with one standard-guitar track and `bars` empty 4/4 bars.
    #[frb(sync)]
    pub fn create(bars: u32) -> Result<DocumentHandle, ApiError> {
        let score = mc_app::new_score("Guitar", bars.max(1) as usize)
            .map_err(|e| ApiError::new(e.rule(), e.to_string()))?;
        Ok(Self::wrap(DocumentSession::new(
            DocumentId(NEXT_DOCUMENT.fetch_add(1, Ordering::Relaxed)),
            score,
            SessionConfig::default(),
            &SystemClock,
            &SystemEntropy,
        )))
    }

    /// Opens a project file. Unknown content is reported in the summary, not an error.
    pub fn open(path: String) -> Result<DocumentHandle, ApiError> {
        let (session, _report) = DocumentSession::open(
            DocumentId(NEXT_DOCUMENT.fetch_add(1, Ordering::Relaxed)),
            &FsProjectStore::default(),
            Path::new(&path),
            SessionConfig::default(),
            &SystemClock,
        )?;
        Ok(Self::wrap(session))
    }

    /// Applies one command. On error the document is unchanged.
    pub fn execute(
        &self,
        command: CommandDto,
        coalesce_key: Option<i64>,
        expected_revision: Option<i64>,
    ) -> Result<OutcomeDto, ApiError> {
        let mut s = self.lock();
        let command = to_command(&s, command)?;
        let env = CommandEnvelope {
            expected_revision: expected_revision.map(|r| Revision(r as u64)),
            coalesce_key: coalesce_key.map(|k| k as u64),
            ..CommandEnvelope::new(self.next_command.fetch_add(1, Ordering::Relaxed), command)
        };
        Ok(outcome_dto(&s.execute(env)?))
    }

    pub fn undo(&self) -> Result<OutcomeDto, ApiError> {
        Ok(outcome_dto(&self.lock().undo()?))
    }

    pub fn redo(&self) -> Result<OutcomeDto, ApiError> {
        Ok(outcome_dto(&self.lock().redo()?))
    }

    /// Saves to the current location (refused with rule `U2` for unknown content).
    pub fn save(&self) -> Result<OutcomeDto, ApiError> {
        let mut s = self.lock();
        if let Some(path) = s.location().map(Path::to_path_buf) {
            cleanup_temp_files(&path);
        }
        let outcome = s.save(&FsProjectStore::default(), &app_info())?;
        self.discard_own_recovery(&s);
        Ok(outcome_dto(&outcome))
    }

    pub fn save_as(&self, path: String) -> Result<OutcomeDto, ApiError> {
        let path = Path::new(&path);
        cleanup_temp_files(path);
        let mut s = self.lock();
        let outcome = s.save_as(&FsProjectStore::default(), path, &app_info())?;
        self.discard_own_recovery(&s);
        Ok(outcome_dto(&outcome))
    }

    /// Streams every event of this document to Dart. A second call replaces (and
    /// closes) the previous stream.
    /// True when the document is dirty and the autosave interval has passed.
    #[frb(sync)]
    pub fn autosave_due(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.lock().autosave_due(now)
    }

    /// Writes an autosave snapshot to `dir` (per-user app data). The recovery file
    /// stores the project id and the original file location, so the app can offer
    /// recovery on the next startup.
    pub fn write_autosave(&self, dir: String) -> Result<(), ApiError> {
        let mut s = self.lock();
        let bytes = s.snapshot_bytes(&app_info());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let recovery = mc_app::infra::FsRecoveryStore::new(&dir);
        recovery
            .write_autosave(s.project_id(), s.location(), &bytes, now)
            .map_err(|e| ApiError::new("autosave", e.to_string()))?;
        s.mark_autosaved(now);
        *self.recovery_dir() = Some(PathBuf::from(dir));
        Ok(())
    }

    /// Lists the newest autosave of every project in `dir`, unfiltered. Startup uses
    /// [`Self::offered_recovery_entries`] instead.
    #[frb(sync)]
    pub fn list_recovery_entries(dir: String) -> Vec<RecoveryEntryDto> {
        let recovery = mc_app::infra::FsRecoveryStore::new(dir);
        match recovery.list() {
            Ok(entries) => entries.into_iter().map(recovery_entry_dto).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// The autosaves to offer on startup (ADR-030 section 2): none after a clean
    /// session; otherwise those newer than their source file, untitled documents, and
    /// documents whose source is gone. The rule is `mc_app::recovery::offered`.
    #[frb(sync)]
    pub fn offered_recovery_entries(dir: String, unclean: bool) -> Vec<RecoveryEntryDto> {
        let recovery = mc_app::infra::FsRecoveryStore::new(dir);
        let Ok(entries) = recovery.list() else {
            return Vec::new();
        };
        mc_app::recovery::offered(unclean, entries, mc_app::infra::modified_unix_millis)
            .into_iter()
            .map(recovery_entry_dto)
            .collect()
    }

    /// Starts an app session: writes the session marker in `dir` and returns true if
    /// the previous session did not end cleanly (its marker was still there).
    #[frb(sync)]
    pub fn begin_session(dir: String) -> Result<bool, ApiError> {
        mc_app::infra::FsRecoveryStore::new(dir)
            .begin_session()
            .map_err(|e| ApiError::new("session", e.to_string()))
    }

    /// Ends the app session cleanly: removes the session marker in `dir`. Called only
    /// after every document was saved or discarded (ADR-030 section 3).
    #[frb(sync)]
    pub fn end_session(dir: String) -> Result<(), ApiError> {
        mc_app::infra::FsRecoveryStore::new(dir)
            .end_session()
            .map_err(|e| ApiError::new("session", e.to_string()))
    }

    /// Opens the newest autosave of `project_id` in `dir`. The document keeps the
    /// original location (Save writes back there) and starts unsaved, so it keeps
    /// autosaving; its recovery files are removed by the next successful save or by
    /// `discard_recovery`.
    pub fn open_from_recovery(dir: String, project_id: String) -> Result<DocumentHandle, ApiError> {
        let recovery = mc_app::infra::FsRecoveryStore::new(&dir);
        let entry = recovery
            .list()
            .map_err(|e| ApiError::new("recovery", e.to_string()))?
            .into_iter()
            .find(|e| e.project_id == project_id)
            .ok_or_else(|| ApiError::new("not-found", format!("recovery for {project_id}")))?;
        let bytes = recovery
            .read(&entry)
            .map_err(|e| ApiError::new("recovery", e.to_string()))?;
        let (session, _) = DocumentSession::recover(
            DocumentId(NEXT_DOCUMENT.fetch_add(1, Ordering::Relaxed)),
            &bytes,
            entry.source,
            SessionConfig::default(),
            &SystemClock,
        )?;
        let handle = Self::wrap(session);
        *handle.recovery_dir() = Some(PathBuf::from(dir));
        Ok(handle)
    }

    /// Deletes this document's own autosaves (the user chose "Don't save" when closing,
    /// ADR-030 section 3). No-op if it never autosaved.
    #[frb(sync)]
    pub fn discard_autosaves(&self) {
        let s = self.lock();
        self.discard_own_recovery(&s);
    }

    /// Deletes all autosaves of `project_id` in `dir` (the user chose Discard).
    #[frb(sync)]
    pub fn discard_recovery(dir: String, project_id: String) {
        let _ = mc_app::infra::FsRecoveryStore::new(dir).discard(&project_id);
    }

    pub fn subscribe_events(&self, sink: StreamSink<EventDto>) {
        self.lock().set_sink(Box::new(DartEventSink(sink)));
    }

    /// Closes the event stream (explicit unsubscribe, ADR-005 amendment 1).
    #[frb(sync)]
    pub fn unsubscribe_events(&self) {
        drop(self.lock().take_sink());
    }

    #[frb(sync)]
    pub fn summary(&self) -> DocumentSummaryDto {
        summary_dto(&self.lock())
    }

    /// Copies the beats of one voice from (`from_bar`, `from_index`) to (`to_bar`,
    /// `to_index`), both inclusive, possibly across bars, as plain values for the
    /// in-app clipboard (ADR-030). A query: nothing changes.
    #[frb(sync)]
    pub fn copy_beats(
        &self,
        track: i64,
        voice: u32,
        from_bar: u32,
        from_index: u32,
        to_bar: u32,
        to_index: u32,
    ) -> Result<Vec<ClipBeatDto>, ApiError> {
        Ok(self
            .lock()
            .copy_beats(
                track_id(track),
                voice as usize,
                (from_bar as usize, from_index as usize),
                (to_bar as usize, to_index as usize),
            )?
            .into_iter()
            .map(ClipBeatDto::from_app)
            .collect())
    }

    /// Chord voicings for `track` using its tuning and capo (ADR-029 T6/T8). `max` caps
    /// the results (the UI shows a handful). Returns the canonical symbol, the spelled
    /// tones and the voicings; a bad chord symbol is an `ApiError`, an unknown track is
    /// `not-found`. The voicings' frets feed ordinary `SetNote` commands when the user
    /// inserts one (no new music rule, GR-05).
    #[frb(sync)]
    pub fn chord_voicings(
        &self,
        track: i64,
        symbol: String,
        max: u32,
    ) -> Result<ChordResultDto, ApiError> {
        let s = self.lock();
        let lookup = s
            .chord_voicings(track_id(track), &symbol, max.max(1) as usize)
            .ok_or_else(|| ApiError::new("not-found", format!("track {track}")))?
            .map_err(|e| ApiError::new("chord", e.to_string()))?;
        Ok(chord_result_dto(&lookup))
    }

    /// Played-bar layout for the playback cursor (ADR-028): each bar's score index and
    /// tick range after repeat and volta expansion. The UI maps a playback tick to an x
    /// on the TAB from this. Independent of mute (PB-04).
    #[frb(sync)]
    pub fn played_bars(&self) -> Vec<PlayedBarDto> {
        self.lock()
            .played_bars()
            .into_iter()
            .map(|b| PlayedBarDto {
                order: b.order as i64,
                master_bar: b.master_bar as i64,
                start_tick: b.start_tick,
                length_ticks: b.length_ticks,
            })
            .collect()
    }

    /// Beats of one voice of one bar, in order.
    #[frb(sync)]
    pub fn beats(&self, track: i64, bar: u32, voice: u32) -> Result<Vec<BeatDto>, ApiError> {
        let s = self.lock();
        let t = s
            .score()
            .track(track_id(track))
            .ok_or_else(|| ApiError::new("not-found", format!("track {track}")))?;
        let v = t
            .bar(bar as usize)
            .and_then(|b| b.voices.get(voice as usize))
            .ok_or_else(|| ApiError::new("not-found", format!("bar {bar}, voice {voice}")))?;
        Ok(v.beats
            .iter()
            .map(|b| BeatDto {
                id: b.id.raw() as i64,
                duration: DurationDto {
                    base: b.value.base().exponent(),
                    dots: b.value.dots(),
                    tuplet: b.value.tuplet().map(|t| TupletDto {
                        actual: t.actual(),
                        normal: t.normal(),
                    }),
                },
                notes: b
                    .notes
                    .iter()
                    .map(|n| NoteDto {
                        id: n.id.raw() as i64,
                        string: n.position().string.get(),
                        fret: n.position().fret.0,
                        tie: n.tie,
                        let_ring: n.effects.let_ring,
                        dead: n.effects.dead,
                    })
                    .collect(),
            })
            .collect())
    }

    /// Number of voices in a bar of a track.
    #[frb(sync)]
    pub fn voice_count(&self, track: i64, bar: u32) -> u32 {
        self.lock()
            .score()
            .track(track_id(track))
            .and_then(|t| t.bar(bar as usize))
            .map_or(0, |b| b.voices.len() as u32)
    }

    /// Locks the session for a layout query that may update its view cache (the
    /// multi-track outline, ADR-031). Same lock as every other call.
    pub(crate) fn lock_for_layout(&self) -> MutexGuard<'_, DocumentSession> {
        self.lock()
    }

    pub(crate) fn with_session<R>(&self, f: impl FnOnce(&DocumentSession) -> R) -> R {
        f(&self.lock())
    }
}

fn master_bar_dto(m: &MasterBar) -> MasterBarDto {
    MasterBarDto {
        numerator: m.time_signature.numerator(),
        denominator: m.time_signature.denominator(),
        pickup: m.pickup,
        repeat_start: m.repeat_start,
        repeat_end: m.repeat_end,
        alternate_endings: m.alternate_endings.clone(),
        marker: m.marker.clone(),
    }
}

fn summary_dto(s: &DocumentSession) -> DocumentSummaryDto {
    let score = s.score();
    DocumentSummaryDto {
        revision: s.revision().0 as i64,
        dirty: s.is_dirty(),
        unsaved_work: s.has_unsaved_work(),
        can_undo: s.can_undo(),
        can_redo: s.can_redo(),
        location: s.location().map(|p| p.to_string_lossy().into_owned()),
        unknown_content: s.unknown_content().to_vec(),
        tracks: score
            .tracks()
            .map(|t| {
                let f = t.fretted();
                TrackSummaryDto {
                    id: t.id().raw() as i64,
                    name: t.name().to_string(),
                    instrument: InstrumentDto {
                        tuning: f.tuning.strings().iter().map(|p| p.to_string()).collect(),
                        fret_count: f.fret_count,
                        capo: f.capo.0,
                    },
                }
            })
            .collect(),
        master_bars: score.master_bars().iter().map(master_bar_dto).collect(),
        diagnostics: s
            .diagnostics()
            .into_iter()
            .map(|d| DiagnosticDto {
                rule: d.rule.to_string(),
                track: d.track.raw() as i64,
                bar: d.bar as u32,
                voice: d.voice as u32,
                kind: match d.kind {
                    DiagnosticKind::Underfull { .. } => "underfull".into(),
                    DiagnosticKind::Overfull { .. } => "overfull".into(),
                },
            })
            .collect(),
    }
}

/// Maps a `ChordLookup` to its DTO (ADR-029 T6). Frets are per string, string 1 first;
/// `-1` muted, `0` open, `>0` fretted.
fn chord_result_dto(lookup: &mc_app::guitar::ChordLookup) -> ChordResultDto {
    ChordResultDto {
        symbol: lookup.symbol.clone(),
        tones: lookup.tones.clone(),
        voicings: lookup
            .voicings
            .iter()
            .map(|v| VoicingDto {
                frets: v
                    .frets()
                    .iter()
                    .map(|f| f.map_or(-1, |fret| fret.0 as i32))
                    .collect(),
                chart_text: v.to_string(),
                fingers: v.fingers() as i32,
                lowest_fret: v.lowest_fretted().map_or(-1, |f| f as i32),
                highest_fret: v.highest_fretted().map_or(-1, |f| f as i32),
            })
            .collect(),
    }
}

/// Maps the application `PlaybackState` to its DTO (ADR-028). Used by the event mapping
/// here and by the engine API.
pub(crate) fn playback_state_dto(state: mc_app::PlaybackState) -> PlaybackStateDto {
    match state {
        mc_app::PlaybackState::Stopped => PlaybackStateDto::Stopped,
        mc_app::PlaybackState::Playing => PlaybackStateDto::Playing,
        mc_app::PlaybackState::Failed => PlaybackStateDto::Failed,
    }
}

pub(crate) fn outcome_dto(o: &CommandOutcome) -> OutcomeDto {
    OutcomeDto {
        revision: o.revision.0 as i64,
        events: o.events.iter().map(event_dto).collect(),
    }
}

fn event_dto(e: &EventEnvelope) -> EventDto {
    let t = |id: mc_app::types::TrackId| id.raw() as i64;
    let kind = match &e.payload {
        EventPayload::Domain(d) => match d {
            DomainEvent::TrackAdded { track } => EventKindDto::TrackAdded { track: t(*track) },
            DomainEvent::TrackRemoved { track } => EventKindDto::TrackRemoved { track: t(*track) },
            DomainEvent::TrackChanged { track } => EventKindDto::TrackChanged { track: t(*track) },
            DomainEvent::TracksReordered => EventKindDto::TracksReordered,
            DomainEvent::MasterBarsInserted { at, count } => EventKindDto::MasterBarsInserted {
                at: *at as u32,
                count: *count as u32,
            },
            DomainEvent::MasterBarsRemoved { at, count } => EventKindDto::MasterBarsRemoved {
                at: *at as u32,
                count: *count as u32,
            },
            DomainEvent::MasterBarsChanged { bars } => EventKindDto::MasterBarsChanged {
                start: bars.start as u32,
                end: bars.end as u32,
            },
            DomainEvent::BarContentChanged { track, bars } => EventKindDto::BarContentChanged {
                track: t(*track),
                start: bars.start as u32,
                end: bars.end as u32,
            },
            DomainEvent::TempoMapChanged => EventKindDto::TempoMapChanged,
            DomainEvent::TiesRemoved { track, notes } => EventKindDto::TiesRemoved {
                track: t(*track),
                notes: notes.iter().map(|n| n.raw() as i64).collect(),
            },
        },
        EventPayload::App(a) => match a {
            AppEvent::DocumentSaved { .. } => EventKindDto::DocumentSaved,
            AppEvent::DirtyChanged { dirty } => EventKindDto::DirtyChanged { dirty: *dirty },
            AppEvent::UndoAvailabilityChanged { can_undo, can_redo } => {
                EventKindDto::UndoAvailabilityChanged {
                    can_undo: *can_undo,
                    can_redo: *can_redo,
                }
            }
            AppEvent::HistoryApplied => EventKindDto::HistoryApplied,
            AppEvent::PlaybackStateChanged { state } => EventKindDto::PlaybackStateChanged {
                state: playback_state_dto(*state),
            },
            AppEvent::SoloChanged { track, soloed } => EventKindDto::SoloChanged {
                track: t(*track),
                soloed: *soloed,
            },
            AppEvent::MixChanged { track, mix } => EventKindDto::MixChanged {
                track: t(*track),
                volume: mix.volume.get(),
                pan: i16::from(mix.pan.get()),
            },
            AppEvent::MuteChanged { track, muted } => EventKindDto::MuteChanged {
                track: t(*track),
                muted: *muted,
            },
            AppEvent::TempoFactorChanged { hundredths } => EventKindDto::TempoFactorChanged {
                hundredths: *hundredths,
            },
        },
    };
    EventDto {
        seq: e.seq as i64,
        revision: e.revision.0 as i64,
        correlation: e.correlation_id.map(|c| c.0 as i64),
        kind,
    }
}

/// Converts a DTO into an application command. Master bar edits start from the bar's
/// current properties (the key signature is not edited in Phase 3); new bars copy the
/// time signature and key of the bar before them. These are input conveniences; the
/// core validates the result.
fn to_command(s: &DocumentSession, c: CommandDto) -> Result<Command, ApiError> {
    let score = s.score();
    let bar_at = |i: u32| -> Result<MasterBar, ApiError> {
        score
            .master_bars()
            .get(i as usize)
            .cloned()
            .ok_or_else(|| ApiError::new("not-found", format!("bar {i}")))
    };
    let sc = |c: ScoreCommand| Ok(Command::Score(c));
    let tc = |c: TrackCommand| Ok(Command::Track(c));
    match c {
        CommandDto::InsertMasterBars { at, count } => {
            let source = at
                .saturating_sub(1)
                .min(score.master_bar_count().saturating_sub(1) as u32);
            let prev = bar_at(source)?;
            sc(ScoreCommand::InsertMasterBars {
                at: at as usize,
                count: count as usize,
                template: MasterBar::new(prev.time_signature, prev.key),
            })
        }
        CommandDto::RemoveMasterBars { at, count } => sc(ScoreCommand::RemoveMasterBars {
            at: at as usize,
            count: count as usize,
        }),
        CommandDto::SetMasterBar { index, bar } => {
            let mut m = bar_at(index)?;
            m.time_signature = TimeSignature::new(bar.numerator, bar.denominator)
                .map_err(|e| ApiError::new("T", e.to_string()))?;
            m.pickup = bar.pickup;
            m.repeat_start = bar.repeat_start;
            m.repeat_end = bar.repeat_end;
            m.alternate_endings = bar.alternate_endings;
            m.marker = bar.marker;
            sc(ScoreCommand::SetMasterBar {
                index: index as usize,
                bar: m,
            })
        }
        CommandDto::SetTempo {
            bar,
            offset_num,
            offset_den,
            bpm,
        } => sc(ScoreCommand::SetTempo {
            bar: bar as usize,
            offset: (offset_num, offset_den),
            bpm: (bpm as i64, 1),
        }),
        CommandDto::RemoveTempo {
            bar,
            offset_num,
            offset_den,
        } => sc(ScoreCommand::RemoveTempo {
            bar: bar as usize,
            offset: (offset_num, offset_den),
        }),
        CommandDto::InsertBeat {
            track,
            bar,
            voice,
            index,
            duration,
        } => sc(ScoreCommand::InsertBeat {
            track: track_id(track),
            bar: bar as usize,
            voice: voice as usize,
            index: index as usize,
            duration: duration.to_app(),
        }),
        CommandDto::RemoveBeat { beat } => sc(ScoreCommand::RemoveBeat {
            beat: beat.to_app(),
        }),
        CommandDto::SetBeatDuration { beat, duration } => sc(ScoreCommand::SetBeatDuration {
            beat: beat.to_app(),
            duration: duration.to_app(),
        }),
        CommandDto::SetNote { beat, string, fret } => sc(ScoreCommand::SetNote {
            beat: beat.to_app(),
            string,
            fret,
        }),
        CommandDto::RemoveNote { beat, note } => sc(ScoreCommand::RemoveNote {
            beat: beat.to_app(),
            note: note_id(note),
        }),
        CommandDto::SetTie { beat, note, tie } => sc(ScoreCommand::SetTie {
            beat: beat.to_app(),
            note: note_id(note),
            tie,
        }),
        CommandDto::SetNoteEffects {
            beat,
            note,
            let_ring,
            dead,
        } => sc(ScoreCommand::SetNoteEffects {
            beat: beat.to_app(),
            note: note_id(note),
            effects: mc_app::types::NoteEffects { let_ring, dead },
        }),
        CommandDto::AddVoice { track, bar } => sc(ScoreCommand::AddVoice {
            track: track_id(track),
            bar: bar as usize,
        }),
        CommandDto::RemoveVoice { track, bar, voice } => sc(ScoreCommand::RemoveVoice {
            track: track_id(track),
            bar: bar as usize,
            voice: voice as usize,
        }),
        CommandDto::ShiftFrets {
            track,
            from_bar,
            to_bar,
            delta,
        } => sc(ScoreCommand::ShiftFrets {
            track: track_id(track),
            bars: (from_bar as usize, to_bar as usize + 1),
            delta,
        }),
        CommandDto::PasteBeats {
            track,
            bar,
            voice,
            index,
            beats,
        } => sc(ScoreCommand::PasteBeats {
            track: track_id(track),
            bar: bar as usize,
            voice: voice as usize,
            index: index as usize,
            beats: beats.iter().map(ClipBeatDto::to_app).collect(),
        }),
        CommandDto::RemoveBeats {
            track,
            voice,
            from_bar,
            from_index,
            to_bar,
            to_index,
        } => sc(ScoreCommand::RemoveBeats {
            track: track_id(track),
            voice: voice as usize,
            from: (from_bar as usize, from_index as usize),
            to: (to_bar as usize, to_index as usize),
        }),
        CommandDto::AddTrack { name, instrument } => tc(TrackCommand::Add {
            name,
            instrument: instrument.to_app(),
        }),
        CommandDto::RemoveTrack { track } => tc(TrackCommand::Remove {
            track: track_id(track),
        }),
        CommandDto::RenameTrack { track, name } => tc(TrackCommand::Rename {
            track: track_id(track),
            name,
        }),
        CommandDto::SetInstrument { track, instrument } => tc(TrackCommand::SetInstrument {
            track: track_id(track),
            instrument: instrument.to_app(),
        }),
        CommandDto::MoveTrack { track, to } => tc(TrackCommand::Move {
            track: track_id(track),
            to: to as usize,
        }),
        CommandDto::Retune {
            track,
            instrument,
            keep_pitches,
        } => Ok(Command::Guitar(GuitarCommand::Retune {
            track: track_id(track),
            instrument: instrument.to_app(),
            mode: if keep_pitches {
                mc_app::guitar::RetuneMode::KeepPitches
            } else {
                mc_app::guitar::RetuneMode::KeepFrets
            },
        })),
    }
}
