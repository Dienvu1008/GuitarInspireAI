//! ADR-034 PE-13 (session part): the pattern library is saved with the document from
//! format 1.2, reopens unchanged, is part of the dirty check, and keeps the U2 rule.
//! A document whose library is still the built-in sample library writes none.

mod common;
use common::*;
use mc_app::types::AppInfo;
use mc_app::{DocumentId, DocumentSession, SaveError, SessionConfig};
use mc_pattern::{
    Lane, Lanes, PatternLibrary, PatternMeta, PatternParts, Step, Strike, StringSelector,
};
use mc_project::LIBRARY;
use mc_theory::{BaseValue, NoteValue, Rational};
use std::path::Path;

fn app() -> AppInfo {
    AppInfo {
        app: "GuitarInspireAI".into(),
        version: "0.1.0".into(),
    }
}

fn raw(store: &MemStore, key: &str) -> Vec<u8> {
    store.0.borrow().get(Path::new(key)).cloned().unwrap()
}

fn open(store: &MemStore, key: &str) -> DocumentSession {
    DocumentSession::open(
        DocumentId(9),
        store,
        Path::new(key),
        SessionConfig::default(),
        &clock(),
    )
    .unwrap()
    .0
}

fn entry_names(bytes: &[u8]) -> Vec<String> {
    mc_project::read_entries(bytes, &mc_project::Limits::default())
        .unwrap()
        .into_iter()
        .map(|(n, _)| n)
        .collect()
}

/// A one-step pattern named `name`.
fn tiny(name: &str) -> PatternParts {
    PatternParts {
        name: name.into(),
        length: Rational::new(1, 4).unwrap(),
        meter: None,
        lanes: Lanes::Strum(Lane {
            steps: vec![Step {
                onset: Rational::ZERO,
                duration: NoteValue::plain(BaseValue::QUARTER),
                action: mc_pattern::Action::Strike(Strike::on(vec![StringSelector::Bass])),
            }],
        }),
        spans: Vec::new(),
        meta: PatternMeta::default(),
    }
}

#[test]
fn pe13_a_document_with_only_the_sample_library_writes_no_library() {
    let store = MemStore::default();
    let mut s = session(2);
    s.save_as(&store, Path::new("p"), &app()).unwrap();
    assert_eq!(
        entry_names(&raw(&store, "p")),
        ["manifest.json", "score.json"]
    );
    let opened = open(&store, "p");
    assert_eq!(opened.patterns(), s.patterns(), "the sample library again");
    assert!(opened.sample_pattern().is_some());
    assert!(!opened.is_dirty());
}

#[test]
fn pe13_the_library_is_saved_reopened_and_saved_again_byte_for_byte() {
    let store = MemStore::default();
    let mut s = session(2);
    s.save_as(&store, Path::new("p"), &app()).unwrap();
    assert!(!s.is_dirty());
    s.patterns_mut().define_pattern(tiny("Tiny")).unwrap();
    assert!(s.is_dirty(), "a library change makes the document dirty");
    s.save(&store, &app()).unwrap();
    assert!(!s.is_dirty());
    let bytes = raw(&store, "p");
    assert_eq!(
        entry_names(&bytes),
        ["manifest.json", "score.json", LIBRARY]
    );
    let mut opened = open(&store, "p");
    assert_eq!(opened.patterns(), s.patterns());
    assert!(!opened.is_dirty());
    assert!(
        opened.sample_pattern().is_some(),
        "the sample is still there"
    );
    // Saving the reopened document unchanged writes the library entry again, byte for
    // byte (only the saved revision in the manifest moves on).
    opened.save_as(&store, Path::new("q"), &app()).unwrap();
    let again =
        mc_project::read_entries(&raw(&store, "q"), &mc_project::Limits::default()).unwrap();
    let first = mc_project::read_entries(&bytes, &mc_project::Limits::default()).unwrap();
    assert_eq!(again[2], first[2], "library.json identical");
    assert_eq!(again[1], first[1], "score.json identical");
}

#[test]
fn pe13_undoing_a_library_change_by_hand_makes_the_document_clean_again() {
    let store = MemStore::default();
    let mut s = session(1);
    s.save_as(&store, Path::new("p"), &app()).unwrap();
    let id = s.patterns_mut().define_pattern(tiny("Tiny")).unwrap();
    assert!(s.is_dirty());
    s.patterns_mut().remove_pattern(id).unwrap();
    // The id counter moved on, so the library is not what was saved: still dirty.
    assert!(s.is_dirty());
    s.save(&store, &app()).unwrap();
    assert!(!s.is_dirty());
}

#[test]
fn pe13_removing_the_sample_is_saved_and_kept() {
    let store = MemStore::default();
    let mut s = session(1);
    let sample = s.sample_pattern().unwrap();
    s.patterns_mut().remove_pattern(sample).unwrap();
    assert_eq!(s.sample_pattern(), None);
    s.save_as(&store, Path::new("p"), &app()).unwrap();
    let opened = open(&store, "p");
    assert_eq!(opened.patterns().patterns().count(), 0);
    assert_eq!(opened.sample_pattern(), None);
}

#[test]
fn pe13_a_new_document_with_a_library_change_has_unsaved_work() {
    let mut s = session(1);
    assert!(!s.has_unsaved_work());
    s.patterns_mut().define_pattern(tiny("Tiny")).unwrap();
    assert!(s.has_unsaved_work());
}

#[test]
fn pe13_autosave_snapshots_carry_the_library() {
    let mut s = session(1);
    s.patterns_mut().define_pattern(tiny("Tiny")).unwrap();
    let (recovered, _) = DocumentSession::recover(
        DocumentId(3),
        &s.snapshot_bytes(&app()),
        None,
        SessionConfig::default(),
        &clock(),
    )
    .unwrap();
    assert_eq!(recovered.patterns(), s.patterns());
    assert!(recovered.is_dirty(), "a recovered document starts unsaved");
}

#[test]
fn pe13_u2_unknown_library_content_blocks_save_over_the_original() {
    // A package written by a later build: the library has a field this build does not
    // know. Save over it is refused; Save As works and writes the known content.
    let store = MemStore::default();
    let lib: PatternLibrary = mc_pattern::fixtures::library();
    let info = mc_project::SaveInfo {
        project_id: mc_project::ProjectId::from_parts(1, 2),
        created_by: app(),
        saved_by: app(),
        saved_revision: 1,
    };
    let base = mc_project::save_with_library(&mc_score::fixtures::techniques(), Some(&lib), &info);
    let entries = mc_project::read_entries(&base, &mc_project::Limits::default()).unwrap();
    let (old_lib, old_manifest) = (&entries[2].1, &entries[0].1);
    // Add an unknown key to the first pattern, then fix its hash and size.
    let text = String::from_utf8(old_lib.clone()).unwrap();
    let library = text
        .replacen(
            "\"name\":\"Travis\"",
            "\"name\":\"Travis\",\"x_future\":true",
            1,
        )
        .into_bytes();
    assert_ne!(&library, old_lib);
    let manifest = String::from_utf8(old_manifest.clone())
        .unwrap()
        .replace(
            &mc_project::sha256_hex(old_lib),
            &mc_project::sha256_hex(&library),
        )
        .replace(
            &format!("\"size\": {}", old_lib.len()),
            &format!("\"size\": {}", library.len()),
        );
    let future = mc_project::write_entries(&[
        ("manifest.json", manifest.as_bytes()),
        ("score.json", &entries[1].1),
        (LIBRARY, &library),
    ]);
    store.0.borrow_mut().insert("orig".into(), future.clone());
    let (mut opened, report) = DocumentSession::open(
        DocumentId(4),
        &store,
        Path::new("orig"),
        SessionConfig::default(),
        &clock(),
    )
    .unwrap();
    assert_eq!(report.unknown_content, ["library.patterns[0].x_future"]);
    assert_eq!(opened.patterns(), &lib, "known content loads");
    assert!(matches!(
        opened.save(&store, &app()),
        Err(SaveError::WouldDropUnknownContent { .. })
    ));
    assert_eq!(raw(&store, "orig"), future, "the original is untouched");
    opened.save_as(&store, Path::new("copy"), &app()).unwrap();
    assert_eq!(open(&store, "copy").patterns(), &lib);
}

// ------------------------------------------------- 6a-5b: library commands

mod commands {
    use super::*;
    use mc_app::library::{LibraryError, LibraryOutcome};
    use mc_app::{AppEvent, Command, CommandError, EventPayload, LaneLayout, PatternCommand};
    use mc_pattern::grid::{Cell, Edit, GridError};
    use mc_pattern::{Action, LaneId, PatternError, PatternId, Technique};
    use proptest::prelude::*;

    fn lib_cmd(c: PatternCommand) -> Command {
        Command::Pattern(c)
    }

    fn new_pattern(s: &mut DocumentSession) -> PatternId {
        s.execute(cmd(
            1,
            lib_cmd(PatternCommand::NewPattern {
                name: "Grid".into(),
                layout: LaneLayout::Strum,
                length: Rational::ONE,
                meter: Some(mc_theory::TimeSignature::COMMON),
            }),
        ))
        .unwrap()
        .library
        .unwrap()
        .pattern
        .unwrap()
    }

    fn edit(pattern: PatternId, edit: Edit) -> Command {
        lib_cmd(PatternCommand::EditPattern { pattern, edit })
    }

    fn place(beat: u32, action: Action) -> Edit {
        Edit::Place {
            lane: LaneId::Strum,
            at: Cell {
                beat,
                parts: 2,
                cell: 0,
            },
            cells: 2,
            action,
        }
    }

    fn bass() -> Action {
        Action::Strike(Strike::on(vec![StringSelector::Bass]))
    }

    fn has_library_event(out: &mc_app::CommandOutcome) -> bool {
        out.events
            .iter()
            .any(|e| e.payload == EventPayload::App(AppEvent::PatternLibraryChanged))
    }

    #[test]
    fn pe14_a_library_command_is_one_revision_and_one_undo_step() {
        let mut s = session(1);
        let (rev, depth, score) = (s.revision(), s.undo_depth(), s.snapshot());
        let p = new_pattern(&mut s);
        assert_eq!(s.revision().0, rev.0 + 1);
        assert_eq!(s.undo_depth(), depth + 1);
        assert_eq!(s.score(), &score, "the score does not change");
        let out = s.execute(cmd(2, edit(p, place(0, bass())))).unwrap();
        assert!(has_library_event(&out));
        assert_eq!(out.library, Some(LibraryOutcome::default()));
        let steps = s
            .patterns()
            .pattern(p)
            .unwrap()
            .parts()
            .lanes
            .get(LaneId::Strum)
            .unwrap()
            .steps
            .len();
        assert_eq!(steps, 1);
        assert_eq!(
            s.patterns().pattern(p).unwrap().parts().meta.origin,
            Some(mc_pattern::Origin::Grid)
        );
    }

    #[test]
    fn pe14_undo_and_redo_restore_the_library_and_announce_it() {
        let mut s = session(1);
        let start = s.patterns().clone();
        let p = new_pattern(&mut s);
        s.execute(cmd(2, edit(p, place(0, bass())))).unwrap();
        let after = s.patterns().clone();
        let out = s.undo().unwrap();
        assert!(has_library_event(&out));
        s.undo().unwrap();
        assert_eq!(s.patterns(), &start);
        s.redo().unwrap();
        let out = s.redo().unwrap();
        assert!(has_library_event(&out));
        assert_eq!(s.patterns(), &after);
    }

    #[test]
    fn pe14_score_steps_and_library_steps_undo_independently() {
        let mut s = session(2);
        let t = first_track(&s);
        let p = new_pattern(&mut s);
        let lib = s.patterns().clone();
        add_beat(&mut s, t, 0);
        // Undoing the score step leaves the library alone and says nothing about it.
        let out = s.undo().unwrap();
        assert!(!has_library_event(&out));
        assert_eq!(s.patterns(), &lib);
        assert!(s.patterns().pattern(p).is_some());
    }

    #[test]
    fn pe14_a_refused_command_changes_nothing() {
        let mut s = session(1);
        let p = new_pattern(&mut s);
        s.execute(cmd(2, edit(p, place(0, bass())))).unwrap();
        let (rev, depth, lib) = (s.revision(), s.undo_depth(), s.patterns().clone());
        // A grid error: a snap of 5 cells.
        let e = s
            .execute(cmd(
                3,
                edit(
                    p,
                    Edit::Place {
                        lane: LaneId::Strum,
                        at: Cell {
                            beat: 1,
                            parts: 5,
                            cell: 0,
                        },
                        cells: 1,
                        action: bass(),
                    },
                ),
            ))
            .unwrap_err();
        assert_eq!(
            e,
            CommandError::Library(LibraryError::Grid(GridError::BadSnap { parts: 5 }))
        );
        assert_eq!(e.rule(), "PE-14");
        // A rule error: placing over the first step (PT-03).
        let e = s.execute(cmd(4, edit(p, place(0, bass())))).unwrap_err();
        let CommandError::Library(LibraryError::Rules(errors)) = &e else {
            panic!("{e:?}")
        };
        assert!(
            errors
                .iter()
                .any(|x| matches!(x, PatternError::Overlap { .. }))
        );
        assert_eq!(e.rule(), "PT");
        // An unknown pattern.
        assert!(
            s.execute(cmd(5, edit(PatternId::from_raw(999), place(1, bass()))))
                .is_err()
        );
        assert_eq!((s.revision(), s.undo_depth()), (rev, depth));
        assert_eq!(s.patterns(), &lib);
    }

    #[test]
    fn pe14_a_gesture_is_made_from_a_strike_in_one_step() {
        let mut s = session(1);
        let p = new_pattern(&mut s);
        s.execute(cmd(2, edit(p, place(0, bass())))).unwrap();
        s.execute(cmd(
            3,
            edit(
                p,
                Edit::SetTechnique {
                    lane: LaneId::Strum,
                    index: 0,
                    technique: Technique::PalmMute,
                    on: true,
                },
            ),
        ))
        .unwrap();
        let depth = s.undo_depth();
        let out = s
            .execute(cmd(
                4,
                lib_cmd(PatternCommand::GestureFromStep {
                    pattern: p,
                    lane: LaneId::Strum,
                    index: 0,
                    name: "Bùm".into(),
                }),
            ))
            .unwrap();
        let g = out.library.unwrap().gesture.unwrap();
        assert_eq!(s.undo_depth(), depth + 1);
        let def = s.patterns().gesture(g).unwrap();
        assert_eq!(def.name(), "Bùm");
        assert!(def.strike().techniques.palm_mute);
        let step = &s
            .patterns()
            .pattern(p)
            .unwrap()
            .parts()
            .lanes
            .get(LaneId::Strum)
            .unwrap()
            .steps[0];
        assert_eq!(step.action, Action::Gesture(g));
        // A second gesture with the same name is refused, and nothing changes.
        s.execute(cmd(5, edit(p, place(1, bass())))).unwrap();
        let lib = s.patterns().clone();
        assert!(
            s.execute(cmd(
                6,
                lib_cmd(PatternCommand::GestureFromStep {
                    pattern: p,
                    lane: LaneId::Strum,
                    index: 1,
                    name: "BÙM".into(),
                }),
            ))
            .is_err()
        );
        assert_eq!(s.patterns(), &lib);
        // A step that is not a strike.
        assert!(matches!(
            s.execute(cmd(
                7,
                lib_cmd(PatternCommand::GestureFromStep {
                    pattern: p,
                    lane: LaneId::Strum,
                    index: 0,
                    name: "Again".into(),
                }),
            )),
            Err(CommandError::Library(LibraryError::Grid(
                GridError::NotAStrike { .. }
            )))
        ));
        // One undo puts the strike back and removes the gesture.
        s.undo().unwrap(); // the second place
        s.undo().unwrap();
        assert!(s.patterns().gesture(g).is_none());
        let step = &s
            .patterns()
            .pattern(p)
            .unwrap()
            .parts()
            .lanes
            .get(LaneId::Strum)
            .unwrap()
            .steps[0];
        assert!(matches!(step.action, Action::Strike(_)));
    }

    #[test]
    fn pe14_gesture_commands_keep_every_pattern_valid() {
        let mut s = session(1);
        let out = s
            .execute(cmd(
                1,
                lib_cmd(PatternCommand::DefineGesture {
                    name: "XL".into(),
                    strike: Strike::on(vec![StringSelector::Bass, StringSelector::TREBLE]),
                }),
            ))
            .unwrap();
        let g = out.library.unwrap().gesture.unwrap();
        let p = new_pattern(&mut s);
        s.execute(cmd(2, edit(p, place(0, Action::Gesture(g)))))
            .unwrap();
        s.execute(cmd(
            3,
            edit(
                p,
                place(1, Action::Hold(Some(vec![StringSelector::TREBLE]))),
            ),
        ))
        .unwrap(); // a hold of the gesture's treble strings (PT-05 through the gesture)
        s.execute(cmd(
            4,
            lib_cmd(PatternCommand::RenameGesture {
                gesture: g,
                name: "Chuck".into(),
            }),
        ))
        .unwrap();
        assert_eq!(s.patterns().gesture(g).unwrap().name(), "Chuck");
        // Removing a gesture in use is refused.
        assert!(
            s.execute(cmd(
                5,
                lib_cmd(PatternCommand::RemoveGesture { gesture: g })
            ))
            .is_err()
        );
        s.execute(cmd(
            6,
            lib_cmd(PatternCommand::RemovePattern { pattern: p }),
        ))
        .unwrap();
        s.execute(cmd(
            7,
            lib_cmd(PatternCommand::RemoveGesture { gesture: g }),
        ))
        .unwrap();
        assert!(s.patterns().gesture(g).is_none());
    }

    #[test]
    fn pe14_library_commands_make_the_document_dirty_and_undo_cleans_it() {
        let store = MemStore::default();
        let mut s = session(1);
        s.save_as(&store, Path::new("p"), &app()).unwrap();
        let out = s
            .execute(cmd(
                1,
                lib_cmd(PatternCommand::DefinePattern {
                    parts: tiny("Tiny"),
                }),
            ))
            .unwrap();
        assert!(s.is_dirty());
        assert!(
            out.events
                .iter()
                .any(|e| e.payload == EventPayload::App(AppEvent::DirtyChanged { dirty: true }))
        );
        let out = s.undo().unwrap();
        assert!(!s.is_dirty(), "undo back to the saved library is clean");
        assert!(
            out.events
                .iter()
                .any(|e| e.payload == EventPayload::App(AppEvent::DirtyChanged { dirty: false }))
        );
    }

    #[test]
    fn pe14_coalesced_steps_undo_to_the_state_before_the_first() {
        let mut s = session(2);
        let t = first_track(&s);
        let p = new_pattern(&mut s);
        s.execute(cmd(2, edit(p, place(0, bass())))).unwrap();
        let (score0, lib0, depth) = (s.snapshot(), s.patterns().clone(), s.undo_depth());
        let keyed = |id, c| mc_app::CommandEnvelope {
            coalesce_key: Some(77),
            ..cmd(id, c)
        };
        // A score step, then two resizes of the same step (a drag), with one key.
        let index = s.score().track(t).unwrap().bar(0).unwrap().voices[0]
            .beats
            .len();
        s.execute(keyed(
            3,
            Command::Score(mc_app::ScoreCommand::InsertBeat {
                track: t,
                bar: 0,
                voice: 0,
                index,
                duration: mc_app::DurationSpec::plain(2),
            }),
        ))
        .unwrap();
        for (id, cells) in [(4, 3), (5, 4)] {
            s.execute(keyed(
                id,
                edit(
                    p,
                    Edit::Resize {
                        lane: LaneId::Strum,
                        index: 0,
                        parts: 2,
                        cells,
                    },
                ),
            ))
            .unwrap();
        }
        assert_eq!(s.undo_depth(), depth + 1, "one step");
        s.undo().unwrap();
        assert_eq!(s.score(), &score0);
        assert_eq!(
            s.patterns(),
            &lib0,
            "the library before the first library step"
        );
    }

    /// One random step: a score edit or a library edit.
    #[derive(Clone, Debug)]
    enum Op {
        Beat(usize),
        Define(u8),
        Remove(usize),
        Place(usize, u32),
        Undo,
        Redo,
    }

    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![
            (0usize..2).prop_map(Op::Beat),
            any::<u8>().prop_map(Op::Define),
            (0usize..4).prop_map(Op::Remove),
            (0usize..4, 0u32..4).prop_map(|(p, b)| Op::Place(p, b)),
            Just(Op::Undo),
            Just(Op::Redo),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 120, .. ProptestConfig::default() })]

        /// Mixed score and library steps: undoing everything gives back the first score
        /// and library, redoing everything the last ones, and every step is one revision.
        #[test]
        fn pe14_mixed_histories_undo_and_redo_exactly(ops in proptest::collection::vec(op(), 1..20)) {
            let mut s = session(2);
            let t = first_track(&s);
            let (score0, lib0) = (s.snapshot(), s.patterns().clone());
            for (i, o) in ops.iter().enumerate() {
                let id = 100 + i as u64;
                let ids: Vec<PatternId> = s.patterns().patterns().map(|p| p.id()).collect();
                let before = s.revision();
                let ok = match o {
                    Op::Beat(bar) => { add_beat(&mut s, t, *bar); true }
                    Op::Define(n) => s.execute(cmd(id, lib_cmd(PatternCommand::DefinePattern { parts: tiny(&format!("P{n}")) }))).is_ok(),
                    Op::Remove(k) => match ids.get(*k) {
                        Some(p) => s.execute(cmd(id, lib_cmd(PatternCommand::RemovePattern { pattern: *p }))).is_ok(),
                        None => false,
                    },
                    Op::Place(k, b) => match ids.get(*k) {
                        Some(p) => s.execute(cmd(id, edit(*p, place(*b, bass())))).is_ok(),
                        None => false,
                    },
                    Op::Undo => s.undo().is_ok(),
                    Op::Redo => s.redo().is_ok(),
                };
                let moved = s.revision() != before;
                prop_assert_eq!(ok, moved, "{:?}", o);
            }
            let (score1, lib1) = (s.snapshot(), s.patterns().clone());
            let mut undone = 0;
            while s.undo().is_ok() { undone += 1; }
            prop_assert_eq!(s.score(), &score0);
            prop_assert_eq!(s.patterns(), &lib0);
            for _ in 0..undone { s.redo().unwrap(); }
            prop_assert_eq!(s.score(), &score1);
            prop_assert_eq!(s.patterns(), &lib1);
        }
    }
}
