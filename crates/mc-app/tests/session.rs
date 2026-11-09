mod common;
use common::*;
use mc_app::types::{AppInfo, DomainEvent, MasterBar};
use mc_app::{
    AppEvent, CollectingSink, Command, CommandEnvelope, CommandError, DocumentId, DocumentSession,
    EventPayload, HistoryLimits, InstrumentSpecInput, Revision, SaveError, ScoreCommand,
    SessionConfig, TrackCommand,
};
use std::path::Path;

fn app() -> AppInfo {
    AppInfo {
        app: "GuitarInspireAI".into(),
        version: "0.1.0".into(),
    }
}

fn set_note(beat: mc_app::BeatAddress, string: u8, fret: u8) -> Command {
    Command::Score(ScoreCommand::SetNote { beat, string, fret })
}

/// P-04: the session revision only increases (commands, undo and redo), and each save
/// records an increasing saved_revision in the file.
#[test]
fn p04_revision_is_monotonic_and_saved_revision_is_recorded() {
    let store = MemStore::default();
    let mut s = session(2);
    let t = first_track(&s);
    let mut last = s.revision();
    let mut check = |s: &DocumentSession| {
        assert!(s.revision() > last, "{:?} after {:?}", s.revision(), last);
        last = s.revision();
    };
    let b = add_beat(&mut s, t, 0);
    check(&s);
    s.execute(cmd(1, set_note(b, 1, 1))).unwrap();
    check(&s);
    s.undo().unwrap();
    check(&s);
    s.redo().unwrap();
    check(&s);
    assert!(s.execute(cmd(2, set_note(b, 9, 1))).is_err());
    assert_eq!(
        s.revision(),
        last,
        "rejected commands do not change the revision"
    );

    s.save_as(&store, Path::new("p"), &app()).unwrap();
    s.save(&store, &app()).unwrap();
    let first = mc_project::load(&store.read_raw("p"))
        .unwrap()
        .saved_revision;
    s.execute(cmd(3, set_note(b, 1, 2))).unwrap();
    s.save(&store, &app()).unwrap();
    let second = mc_project::load(&store.read_raw("p"))
        .unwrap()
        .saved_revision;
    assert_eq!((first, second), (2, 3));
}

#[test]
fn e01_stale_revision_is_rejected() {
    let mut s = session(2);
    let t = first_track(&s);
    let b = add_beat(&mut s, t, 0);
    let env = CommandEnvelope {
        expected_revision: Some(Revision(0)),
        ..cmd(1, set_note(b, 1, 3))
    };
    assert_eq!(
        s.execute(env).unwrap_err(),
        CommandError::StaleRevision {
            expected: Revision(0),
            actual: Revision(1)
        }
    );
    let env = CommandEnvelope {
        expected_revision: Some(Revision(1)),
        ..cmd(2, set_note(b, 1, 3))
    };
    assert_eq!(s.execute(env).unwrap().revision, Revision(2));
}

#[test]
fn t06_t07_c01_configurable_constraints_reject_user_input() {
    let mut s = session(1);
    let t = first_track(&s);
    let tempo = |bpm| {
        cmd(
            1,
            Command::Score(ScoreCommand::SetTempo {
                bar: 0,
                offset: (0, 1),
                bpm: (bpm, 1),
            }),
        )
    };
    assert_eq!(s.execute(tempo(10)).unwrap_err().rule(), "T-06");
    assert_eq!(s.execute(tempo(401)).unwrap_err().rule(), "T-06");
    assert!(s.execute(tempo(400)).is_ok());
    let thirteen = InstrumentSpecInput {
        tuning: vec!["E4".into(); 13],
        fret_count: 24,
        capo: 0,
    };
    assert_eq!(
        s.execute(cmd(
            2,
            Command::Track(TrackCommand::SetInstrument {
                track: t,
                instrument: thirteen
            })
        ))
        .unwrap_err()
        .rule(),
        "T-07"
    );
    let frets = InstrumentSpecInput {
        fret_count: 37,
        ..InstrumentSpecInput::standard_guitar()
    };
    assert_eq!(
        s.execute(cmd(
            3,
            Command::Track(TrackCommand::SetInstrument {
                track: t,
                instrument: frets
            })
        ))
        .unwrap_err()
        .rule(),
        "C-01"
    );
    assert_eq!(
        s.execute(cmd(
            4,
            Command::Track(TrackCommand::Rename {
                track: t,
                name: "  ".into()
            })
        ))
        .unwrap_err()
        .rule(),
        "C-02"
    );
}

#[test]
fn events_are_invalidation_hints_with_ranges() {
    let mut s = session(4);
    s.set_sink(Box::new(CollectingSink::default()));
    let t = first_track(&s);
    let b = add_beat(&mut s, t, 2);
    let out = s.execute(cmd(9, set_note(b, 6, 3))).unwrap();
    let domain: Vec<_> = out
        .events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::Domain(d) => Some(d.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        domain,
        [DomainEvent::BarContentChanged {
            track: t,
            bars: 2..3
        }]
    );
    // The first command made the document dirty and undo available; the second changes neither.
    assert!(
        !out.events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::App(AppEvent::DirtyChanged { .. })))
    );
}

#[test]
fn undo_redo_emit_change_ranges_and_availability() {
    let mut s = session(4);
    let t = first_track(&s);
    let b = add_beat(&mut s, t, 3);
    s.execute(cmd(2, set_note(b, 1, 1))).unwrap();
    let out = s.undo().unwrap();
    assert!(out.events.iter().any(|e| e.payload
        == EventPayload::Domain(DomainEvent::BarContentChanged {
            track: t,
            bars: 3..4
        })));
    assert!(
        out.events
            .iter()
            .any(|e| e.payload == EventPayload::App(AppEvent::HistoryApplied))
    );
    assert!(out.events.iter().any(|e| e.payload
        == EventPayload::App(AppEvent::UndoAvailabilityChanged {
            can_undo: true,
            can_redo: true
        })));
    assert!(out.events.iter().all(|e| e.correlation_id.is_none()));
    s.undo().unwrap();
    assert!(!s.can_undo());
    assert_eq!(s.undo().unwrap_err(), CommandError::NothingToUndo);
    s.redo().unwrap();
    s.redo().unwrap();
    assert_eq!(s.redo().unwrap_err(), CommandError::NothingToRedo);
}

#[test]
fn a_new_command_clears_redo() {
    let mut s = session(2);
    let t = first_track(&s);
    let b = add_beat(&mut s, t, 0);
    s.undo().unwrap();
    assert!(s.can_redo());
    let _ = add_beat(&mut s, t, 1);
    assert!(!s.can_redo());
    let _ = b;
}

#[test]
fn coalesced_commands_are_one_undo_step() {
    let mut s = session(1);
    let t = first_track(&s);
    let b = add_beat(&mut s, t, 0);
    let depth = s.undo_depth();
    for (i, fret) in [1u8, 12].into_iter().enumerate() {
        let env = CommandEnvelope {
            coalesce_key: Some(77),
            ..cmd(10 + i as u64, set_note(b, 2, fret))
        };
        s.execute(env).unwrap();
    }
    assert_eq!(s.undo_depth(), depth + 1);
    s.undo().unwrap();
    assert!(
        s.score().track(t).unwrap().bar(0).unwrap().voices[0].beats[0]
            .notes
            .is_empty()
    );
}

#[test]
fn history_is_bounded_by_steps_and_bytes() {
    let config = SessionConfig {
        history: HistoryLimits {
            max_steps: 5,
            max_bytes: usize::MAX,
        },
        ..SessionConfig::default()
    };
    let mut s = DocumentSession::new(
        DocumentId(2),
        mc_app::new_score("t", 2).unwrap(),
        config,
        &clock(),
        &FixedEntropy,
    );
    let t = first_track(&s);
    for _ in 0..12 {
        add_beat(&mut s, t, 0);
    }
    assert_eq!(s.undo_depth(), 5);

    // Byte budget with small steps spread over many bars.
    let config = SessionConfig {
        history: HistoryLimits {
            max_steps: 1000,
            max_bytes: 4096,
        },
        ..SessionConfig::default()
    };
    let mut s = DocumentSession::new(
        DocumentId(3),
        mc_app::new_score("t", 50).unwrap(),
        config,
        &clock(),
        &FixedEntropy,
    );
    let t = first_track(&s);
    for i in 0..200 {
        add_beat(&mut s, t, i % 50);
    }
    assert!(s.history_bytes() <= 4096, "{}", s.history_bytes());
    assert!(s.undo_depth() >= 1 && s.undo_depth() < 200);

    // A single step larger than the whole budget is still kept, so the last edit can be undone.
    let config = SessionConfig {
        history: HistoryLimits {
            max_steps: 1000,
            max_bytes: 16,
        },
        ..SessionConfig::default()
    };
    let mut s = DocumentSession::new(
        DocumentId(4),
        mc_app::new_score("t", 1).unwrap(),
        config,
        &clock(),
        &FixedEntropy,
    );
    let t = first_track(&s);
    add_beat(&mut s, t, 0);
    add_beat(&mut s, t, 0);
    assert_eq!(s.undo_depth(), 1);
    assert!(s.history_bytes() > 16);
    s.undo().unwrap();
}

#[test]
fn save_open_roundtrip_and_dirty_tracking() {
    let store = MemStore::default();
    let mut s = session(3);
    assert!(s.is_dirty(), "a new document is unsaved");
    assert_eq!(s.save(&store, &app()).unwrap_err(), SaveError::NoLocation);
    let t = first_track(&s);
    let b = add_beat(&mut s, t, 1);
    s.execute(cmd(1, set_note(b, 3, 5))).unwrap();
    let out = s.save_as(&store, Path::new("song"), &app()).unwrap();
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::App(AppEvent::DocumentSaved { .. })))
    );
    assert!(!s.is_dirty());
    s.execute(cmd(2, set_note(b, 3, 7))).unwrap();
    assert!(s.is_dirty());
    s.undo().unwrap();
    assert!(!s.is_dirty(), "undo back to the saved revision is clean");
    s.redo().unwrap();
    s.save(&store, &app()).unwrap();

    let (opened, report) = DocumentSession::open(
        DocumentId(5),
        &store,
        Path::new("song"),
        SessionConfig::default(),
        &clock(),
    )
    .unwrap();
    assert!(report.unknown_content.is_empty());
    assert_eq!(opened.score(), s.score());
    assert_eq!(
        opened.project_id(),
        s.project_id(),
        "project id survives saves"
    );
    assert!(!opened.is_dirty());
}

/// Round-trip item 4b: a document with unknown content (U1) cannot be saved over its
/// original (U2), but Save As to another location works and the original is untouched.
#[test]
fn rt4b_unknown_content_blocks_save_but_allows_save_as() {
    let store = MemStore::default();
    let mut s = session(2);
    s.save_as(&store, Path::new("orig"), &app()).unwrap();
    let original = store.read_raw("orig");
    let future = with_unknown_field(&original);
    store.0.borrow_mut().insert("orig".into(), future.clone());

    let (mut opened, report) = DocumentSession::open(
        DocumentId(6),
        &store,
        Path::new("orig"),
        SessionConfig::default(),
        &clock(),
    )
    .unwrap();
    assert_eq!(report.unknown_content, ["$.x_future_field"]);
    let t = first_track(&opened);
    add_beat(&mut opened, t, 0);
    match opened.save(&store, &app()) {
        Err(SaveError::WouldDropUnknownContent { paths }) => {
            assert_eq!(paths, ["$.x_future_field"])
        }
        other => panic!("expected U2 refusal, got {other:?}"),
    }
    assert!(matches!(
        opened.save_as(&store, Path::new("orig"), &app()),
        Err(SaveError::WouldDropUnknownContent { .. })
    ));
    assert_eq!(store.read_raw("orig"), future, "original file untouched");
    opened.save_as(&store, Path::new("copy"), &app()).unwrap();
    assert!(opened.unknown_content().is_empty());
    opened.save(&store, &app()).unwrap(); // now saving to "copy" is fine
    assert_eq!(store.read_raw("orig"), future);
}

trait ReadRaw {
    fn read_raw(&self, key: &str) -> Vec<u8>;
}

impl ReadRaw for MemStore {
    fn read_raw(&self, key: &str) -> Vec<u8> {
        self.0.borrow().get(Path::new(key)).cloned().unwrap()
    }
}

/// Simulates a file from a newer minor version: one unknown top-level field in score.json,
/// with the manifest hash and size updated. Plain text edits keep this test free of a
/// JSON dependency (mc-app's dev-dependencies are fixed by the FF-01 matrix).
fn with_unknown_field(pkg: &[u8]) -> Vec<u8> {
    let e = mc_project::read_entries(pkg, &mc_project::Limits::default()).unwrap();
    let old_score = String::from_utf8(e[1].1.clone()).unwrap();
    let score = old_score.replacen('{', "{\"x_future_field\":{\"anything\":1},", 1);
    let manifest = String::from_utf8(e[0].1.clone())
        .unwrap()
        .replace(
            &mc_project::sha256_hex(old_score.as_bytes()),
            &mc_project::sha256_hex(score.as_bytes()),
        )
        .replace(
            &format!("\"size\": {}", old_score.len()),
            &format!("\"size\": {}", score.len()),
        );
    mc_project::write_entries(&[
        ("manifest.json", manifest.as_bytes()),
        ("score.json", score.as_bytes()),
    ])
}

#[test]
fn autosave_is_due_only_when_dirty_after_the_interval() {
    let c = clock();
    let mut s = DocumentSession::new(
        DocumentId(7),
        mc_app::new_score("t", 1).unwrap(),
        SessionConfig::default(),
        &c,
        &FixedEntropy,
    );
    let start = c.now();
    assert!(!s.autosave_due(start + 1000));
    assert!(s.autosave_due(start + 120_000), "new documents are dirty");
    s.mark_autosaved(start + 120_000);
    assert!(!s.autosave_due(start + 150_000));
    let store = MemStore::default();
    s.save_as(&store, Path::new("x"), &app()).unwrap();
    assert!(
        !s.autosave_due(start + 999_999),
        "clean documents are not autosaved"
    );
    let restored = DocumentSession::from_bytes(
        DocumentId(8),
        &s.snapshot_bytes(&app()),
        Some("x".into()),
        SessionConfig::default(),
        &c,
    )
    .unwrap()
    .0;
    assert_eq!(restored.score(), s.score());
}

#[test]
fn a_recovered_document_keeps_its_location_and_starts_unsaved() {
    let c = clock();
    let mut s = session(2);
    let store = MemStore::default();
    s.save_as(&store, Path::new("song"), &app()).unwrap();
    let bytes = s.snapshot_bytes(&app());
    let (recovered, _) = DocumentSession::recover(
        DocumentId(9),
        &bytes,
        Some("song".into()),
        SessionConfig::default(),
        &c,
    )
    .unwrap();
    assert_eq!(recovered.score(), s.score());
    assert_eq!(recovered.project_id(), s.project_id());
    assert_eq!(recovered.location(), Some(Path::new("song")));
    assert!(recovered.is_dirty(), "recovered content is not on disk yet");
    assert!(
        recovered.autosave_due(c.now() + 999_999),
        "keeps autosaving until saved"
    );
}

#[test]
fn master_bar_commands_are_routed() {
    let mut s = session(2);
    s.execute(cmd(
        1,
        Command::Score(ScoreCommand::InsertMasterBars {
            at: 1,
            count: 3,
            template: MasterBar::default(),
        }),
    ))
    .unwrap();
    assert_eq!(s.score().master_bar_count(), 5);
    s.execute(cmd(
        2,
        Command::Score(ScoreCommand::RemoveMasterBars { at: 0, count: 2 }),
    ))
    .unwrap();
    assert_eq!(s.score().master_bar_count(), 3);
    s.execute(cmd(
        3,
        Command::Track(TrackCommand::Add {
            name: "Bass".into(),
            instrument: InstrumentSpecInput::standard_guitar(),
        }),
    ))
    .unwrap();
    assert_eq!(s.score().track_count(), 2);
}

trait Now {
    fn now(&self) -> u64;
}

impl Now for FixedClock {
    fn now(&self) -> u64 {
        self.0.get()
    }
}

/// ADR-030 section 3: closing asks "save changes?" only when work would be lost. A new
/// document is dirty but holds nothing until it is edited; opened and recovered
/// documents follow `is_dirty` exactly.
#[test]
fn c6_unsaved_work_ignores_an_untouched_new_document() {
    let store = MemStore::default();
    let mut s = session(2);
    assert!(s.is_dirty());
    assert!(!s.has_unsaved_work(), "untouched new document");

    let t = first_track(&s);
    let b = add_beat(&mut s, t, 0);
    assert!(s.has_unsaved_work(), "edited new document");
    s.undo().unwrap();
    assert!(!s.has_unsaved_work(), "back to how it was created");
    s.redo().unwrap();

    s.save_as(&store, Path::new("song"), &app()).unwrap();
    assert!(!s.has_unsaved_work());
    s.execute(cmd(1, set_note(b, 1, 3))).unwrap();
    assert!(s.has_unsaved_work());
    // After a save, only the saved file counts: removing everything is a change.
    s.execute(cmd(2, Command::Score(ScoreCommand::RemoveBeat { beat: b })))
        .unwrap();
    assert!(s.has_unsaved_work());

    let bytes = store.read_raw("song");
    let (recovered, _) = DocumentSession::recover(
        DocumentId(9),
        &bytes,
        Some("song".into()),
        SessionConfig::default(),
        &clock(),
    )
    .unwrap();
    assert!(
        recovered.has_unsaved_work(),
        "recovered content is not on disk"
    );
    let (opened, _) = DocumentSession::from_bytes(
        DocumentId(10),
        &bytes,
        Some("song".into()),
        SessionConfig::default(),
        &clock(),
    )
    .unwrap();
    assert!(!opened.has_unsaved_work());
}
