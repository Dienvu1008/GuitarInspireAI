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
