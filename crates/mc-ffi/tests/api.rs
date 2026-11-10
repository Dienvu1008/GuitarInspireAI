//! Bridge API tests from Rust (the same calls Dart makes, without the transport).

use mc_ffi::api::document::DocumentHandle;
use mc_ffi::api::layout::{LINE_STRING, default_metrics};
use mc_ffi::api::types::*;

fn quarter() -> DurationDto {
    DurationDto {
        base: 2,
        dots: 0,
        tuplet: None,
    }
}

fn first_track(d: &DocumentHandle) -> i64 {
    d.summary().tracks[0].id
}

fn insert_beat(d: &DocumentHandle, track: i64, bar: u32) -> BeatAddressDto {
    let index = d.beats(track, bar, 0).unwrap().len() as u32;
    d.execute(
        CommandDto::InsertBeat {
            track,
            bar,
            voice: 0,
            index,
            duration: quarter(),
        },
        None,
        None,
    )
    .unwrap();
    let beat = d.beats(track, bar, 0).unwrap()[index as usize].id;
    BeatAddressDto {
        track,
        bar,
        voice: 0,
        beat,
    }
}

#[test]
fn commands_undo_redo_and_events() {
    let d = DocumentHandle::create(2).unwrap();
    let t = first_track(&d);
    let b = insert_beat(&d, t, 0);
    let out = d
        .execute(
            CommandDto::SetNote {
                beat: b,
                string: 6,
                fret: 3,
            },
            None,
            None,
        )
        .unwrap();
    assert_eq!(out.revision, 2);
    assert!(out.events.iter().any(|e| matches!(
        e.kind,
        EventKindDto::BarContentChanged { track, start: 0, end: 1 } if track == t
    )));
    // Sequence numbers are gapless across outcomes.
    let seqs: Vec<i64> = out.events.iter().map(|e| e.seq).collect();
    assert!(seqs.windows(2).all(|w| w[1] == w[0] + 1));
    let notes = &d.beats(t, 0, 0).unwrap()[0].notes;
    assert_eq!((notes[0].string, notes[0].fret), (6, 3));

    d.undo().unwrap();
    assert!(d.beats(t, 0, 0).unwrap()[0].notes.is_empty());
    d.redo().unwrap();
    assert_eq!(d.beats(t, 0, 0).unwrap()[0].notes.len(), 1);
    let s = d.summary();
    assert!(s.dirty && s.can_undo && !s.can_redo);
    assert_eq!(s.revision, 4);
}

#[test]
fn rejected_commands_report_the_rule_and_change_nothing() {
    let d = DocumentHandle::create(1).unwrap();
    let t = first_track(&d);
    let b = insert_beat(&d, t, 0);
    let before = d.summary();
    let err = d
        .execute(
            CommandDto::SetNote {
                beat: b,
                string: 6,
                fret: 30,
            },
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(err.rule, "H-04");
    let stale = d
        .execute(
            CommandDto::SetNote {
                beat: b,
                string: 6,
                fret: 1,
            },
            None,
            Some(0),
        )
        .unwrap_err();
    assert_eq!(stale.rule, "E-01");
    assert_eq!(d.summary(), before);
    let fresh = DocumentHandle::create(1).unwrap();
    assert_eq!(fresh.undo().unwrap_err().rule, "E-05");
    assert_eq!(fresh.redo().unwrap_err().rule, "E-05");
}

#[test]
fn coalesced_commands_are_one_undo_step() {
    let d = DocumentHandle::create(1).unwrap();
    let t = first_track(&d);
    let b = insert_beat(&d, t, 0);
    for fret in [1, 12] {
        d.execute(
            CommandDto::SetNote {
                beat: b,
                string: 1,
                fret,
            },
            Some(42),
            None,
        )
        .unwrap();
    }
    d.undo().unwrap();
    assert!(d.beats(t, 0, 0).unwrap()[0].notes.is_empty());
}

#[test]
fn layout_is_packed_consistently() {
    let d = DocumentHandle::create(3).unwrap();
    let t = first_track(&d);
    let b = insert_beat(&d, t, 1);
    d.execute(
        CommandDto::SetNote {
            beat: b,
            string: 2,
            fret: 10,
        },
        None,
        None,
    )
    .unwrap();
    let l = d.tab_layout(t, default_metrics()).unwrap();
    assert_eq!(l.strings, 6);
    let s = &l.systems[0];
    assert_eq!(s.lines.len(), s.line_kinds.len() * 4);
    assert_eq!(
        s.line_kinds.iter().filter(|k| **k == LINE_STRING).count(),
        6
    );
    assert_eq!(s.hit_rects.len(), s.hit_meta.len());
    // The beat appears once in `beats` and has a hit region on every string.
    let slot = s.beats.iter().position(|x| x.id == b.beat).unwrap() as i32;
    assert_eq!(s.beats[slot as usize].bar, 1);
    let regions = s.hit_meta.chunks(4).filter(|m| m[3] == slot).count();
    assert_eq!(regions, 6);
    assert!(s.glyphs.iter().any(|g| matches!(
        g.kind,
        mc_ffi::api::layout::GlyphKindDto::Fret { fret: 10, .. }
    )));
    let bad = mc_ffi::api::layout::MetricsDto {
        width: 10.0,
        ..default_metrics()
    };
    assert_eq!(d.tab_layout(t, bad).unwrap_err().rule, "layout");
}

#[test]
fn tracks_bars_and_retune() {
    let d = DocumentHandle::create(1).unwrap();
    let t = first_track(&d);
    let standard = d.summary().tracks[0].instrument.clone();
    d.execute(
        CommandDto::AddTrack {
            name: "Bass".into(),
            instrument: standard.clone(),
        },
        None,
        None,
    )
    .unwrap();
    d.execute(CommandDto::InsertMasterBars { at: 1, count: 2 }, None, None)
        .unwrap();
    let mut bar = d.summary().master_bars[1].clone();
    bar.numerator = 3;
    bar.repeat_start = true;
    d.execute(CommandDto::SetMasterBar { index: 1, bar }, None, None)
        .unwrap();
    let s = d.summary();
    assert_eq!(s.tracks.len(), 2);
    assert_eq!(s.master_bars.len(), 3);
    assert_eq!(
        (s.master_bars[1].numerator, s.master_bars[1].repeat_start),
        (3, true)
    );

    let b = insert_beat(&d, t, 0);
    d.execute(
        CommandDto::SetNote {
            beat: b,
            string: 6,
            fret: 3,
        },
        None,
        None,
    )
    .unwrap();
    let mut drop_d = standard;
    drop_d.tuning[5] = "D2".into();
    d.execute(
        CommandDto::Retune {
            track: t,
            instrument: drop_d,
            keep_pitches: true,
        },
        None,
        None,
    )
    .unwrap();
    assert_eq!(d.beats(t, 0, 0).unwrap()[0].notes[0].fret, 5);
    assert_eq!(d.summary().tracks[0].instrument.tuning[5], "D2");
}

#[test]
fn save_and_open_round_trip() {
    let dir = std::env::temp_dir().join(format!("mc-ffi-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("song.giproj").to_string_lossy().into_owned();
    let d = DocumentHandle::create(2).unwrap();
    let t = first_track(&d);
    let b = insert_beat(&d, t, 1);
    d.execute(
        CommandDto::SetNote {
            beat: b,
            string: 3,
            fret: 7,
        },
        None,
        None,
    )
    .unwrap();
    assert_eq!(d.save().unwrap_err().rule, "save-location");
    d.save_as(path.clone()).unwrap();
    assert!(!d.summary().dirty);
    let reopened = DocumentHandle::open(path.clone()).unwrap();
    let s = reopened.summary();
    assert_eq!(s.location.as_deref(), Some(path.as_str()));
    assert_eq!(
        reopened.beats(s.tracks[0].id, 1, 0).unwrap()[0].notes[0].fret,
        7
    );
    assert_eq!(
        DocumentHandle::open(dir.join("missing.giproj").to_string_lossy().into_owned())
            .unwrap_err()
            .rule,
        "open"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Crash recovery (project-format section 12): an autosave survives the session, the
/// recovered document keeps its location and stays unsaved, and a normal save removes
/// the recovery files.
#[test]
fn autosave_recover_and_discard_on_save() {
    let root = std::env::temp_dir().join(format!("mc-ffi-recovery-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let rec = root.join("recovery").to_string_lossy().into_owned();
    let song = root.join("song.giproj").to_string_lossy().into_owned();

    let d = DocumentHandle::create(1).unwrap();
    let t = first_track(&d);
    d.save_as(song.clone()).unwrap();
    let b = insert_beat(&d, t, 0);
    d.execute(
        CommandDto::SetNote {
            beat: b,
            string: 2,
            fret: 5,
        },
        None,
        None,
    )
    .unwrap();
    d.write_autosave(rec.clone()).unwrap();
    drop(d); // the "crash": no save

    let entries = DocumentHandle::list_recovery_entries(rec.clone());
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.source.as_deref(), Some(song.as_str()));
    assert!(entry.timestamp > 0);

    let r = DocumentHandle::open_from_recovery(rec.clone(), entry.project_id.clone()).unwrap();
    let s = r.summary();
    assert!(s.dirty, "recovered content is not on disk yet");
    assert_eq!(s.location.as_deref(), Some(song.as_str()));
    assert_eq!(r.beats(s.tracks[0].id, 0, 0).unwrap()[0].notes[0].fret, 5);
    assert_eq!(
        DocumentHandle::list_recovery_entries(rec.clone()).len(),
        1,
        "opening does not delete the recovery files"
    );

    r.save().unwrap();
    assert!(!r.summary().dirty);
    assert!(
        DocumentHandle::list_recovery_entries(rec.clone()).is_empty(),
        "a normal save removes the recovery files"
    );
    let reopened = DocumentHandle::open(song.clone()).unwrap();
    let s = reopened.summary();
    assert_eq!(
        reopened.beats(s.tracks[0].id, 0, 0).unwrap()[0].notes[0].fret,
        5
    );

    // Discard removes every rotated autosave of a project, not just the newest.
    let u = DocumentHandle::create(1).unwrap();
    for _ in 0..3 {
        u.write_autosave(rec.clone()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let entries = DocumentHandle::list_recovery_entries(rec.clone());
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, None);
    DocumentHandle::discard_recovery(rec.clone(), entries[0].project_id.clone());
    assert!(DocumentHandle::list_recovery_entries(rec.clone()).is_empty());
    assert_eq!(
        DocumentHandle::open_from_recovery(rec, entries[0].project_id.clone())
            .unwrap_err()
            .rule,
        "not-found"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

// ------------------------------------------------------------ ADR-034 pattern bridge

fn c_args(
    track: i64,
    pattern: i64,
    fit: Option<mc_ffi::api::pattern::FitDto>,
) -> mc_ffi::api::pattern::ApplyPatternDto {
    mc_ffi::api::pattern::ApplyPatternDto {
        track,
        first: 0,
        last: 1,
        pattern,
        frets: vec![0, 1, 0, 2, 3, -1],
        chord: "C".into(),
        fit,
        overwrite: false,
    }
}

#[test]
fn pattern_bridge_lists_previews_and_applies_with_a_fit() {
    use mc_ffi::api::pattern::{FitDto, patterns, preview_pattern, sample_pattern};
    let d = DocumentHandle::create(2).unwrap();
    let t = first_track(&d);
    let p = sample_pattern(&d);
    let list = patterns(&d);
    assert_eq!(list.len(), 1);
    assert_eq!(
        (list[0].id, list[0].name.as_str(), list[0].two_lanes),
        (p, "Boom-chick", false)
    );
    assert_eq!(
        (list[0].length_numerator, list[0].length_denominator),
        (1, 1)
    );
    // Bar 2 in 3/4: the range is 7/4, not a whole number of 1-bar repetitions.
    let mut bar = d.summary().master_bars[1].clone();
    bar.numerator = 3;
    d.execute(CommandDto::SetMasterBar { index: 1, bar }, None, None)
        .unwrap();
    let needs = preview_pattern(&d, c_args(t, p, None)).unwrap();
    assert!(needs.needs_fit);
    assert!(needs.report.is_none());
    let cut = preview_pattern(&d, c_args(t, p, Some(FitDto::RepeatAndCut))).unwrap();
    assert!(!cut.needs_fit);
    let r = cut.report.unwrap();
    assert_eq!(r.repetitions, 2);
    // The second repetition is cut after three quarters: its last quarter is dropped.
    assert_eq!(r.dropped.len(), 1);
    assert_eq!(
        (
            r.dropped[0].lane.as_str(),
            r.dropped[0].step,
            r.dropped[0].onset_numerator,
            r.dropped[0].onset_denominator
        ),
        ("strum", 3, 3, 4)
    );
    assert!(r.shortened.is_empty());
    // Without a fit the command is refused with the core's rule; with one it applies.
    let rev = d.summary().revision;
    let err = d
        .execute(
            CommandDto::ApplyPattern {
                args: c_args(t, p, None),
            },
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(err.rule, "PE-08");
    assert_eq!(d.summary().revision, rev);
    let out = d
        .execute(
            CommandDto::ApplyPattern {
                args: c_args(t, p, Some(FitDto::RepeatAndCut)),
            },
            None,
            None,
        )
        .unwrap();
    assert_eq!(out.pattern.unwrap(), r);
    assert_eq!(d.beats(t, 0, 0).unwrap().len(), 4);
    assert_eq!(d.beats(t, 1, 0).unwrap().len(), 3);
}

#[test]
fn pattern_bridge_rejects_bad_values_and_reports_the_overwrite_rule() {
    use mc_ffi::api::pattern::{preview_pattern, sample_pattern};
    let d = DocumentHandle::create(2).unwrap();
    let t = first_track(&d);
    let p = sample_pattern(&d);
    let mut bad = c_args(t, p, None);
    bad.frets[0] = -2;
    assert_eq!(preview_pattern(&d, bad).unwrap_err().rule, "voicing");
    insert_beat(&d, t, 1);
    let r = preview_pattern(&d, c_args(t, p, None))
        .unwrap()
        .report
        .unwrap();
    assert_eq!(r.overwritten, vec![1]);
    let err = d
        .execute(
            CommandDto::ApplyPattern {
                args: c_args(t, p, None),
            },
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(err.rule, "PE-07");
    assert!(err.message.contains("bars 2"), "{}", err.message);
}

#[test]
fn pattern_bridge_preview_revision_guards_the_confirmation() {
    use mc_ffi::api::pattern::{preview_pattern, sample_pattern};
    let d = DocumentHandle::create(2).unwrap();
    let t = first_track(&d);
    let p = sample_pattern(&d);
    insert_beat(&d, t, 1);
    let preview = preview_pattern(&d, c_args(t, p, None)).unwrap();
    assert_eq!(
        preview.revision,
        d.summary().revision,
        "same lock, same revision"
    );
    assert_eq!(preview.report.as_ref().unwrap().overwritten, vec![1]);
    // Another command changes the bar to be replaced; the confirmation is then sent
    // with the preview's revision and must be refused (E-01), leaving the note alone.
    let beat = d.beats(t, 1, 0).unwrap()[0].id;
    d.execute(
        CommandDto::SetNote {
            beat: BeatAddressDto {
                track: t,
                bar: 1,
                voice: 0,
                beat,
            },
            string: 1,
            fret: 7,
        },
        None,
        None,
    )
    .unwrap();
    let rev = d.summary().revision;
    let mut confirmed = c_args(t, p, None);
    confirmed.overwrite = true;
    let err = d
        .execute(
            CommandDto::ApplyPattern {
                args: confirmed.clone(),
            },
            None,
            Some(preview.revision),
        )
        .unwrap_err();
    assert_eq!(err.rule, "E-01");
    assert_eq!(d.summary().revision, rev);
    assert_eq!(
        d.beats(t, 1, 0).unwrap()[0].notes[0].fret,
        7,
        "nothing overwritten"
    );
    // A fresh preview and confirmation applies.
    let fresh = preview_pattern(&d, c_args(t, p, None)).unwrap();
    d.execute(
        CommandDto::ApplyPattern { args: confirmed },
        None,
        Some(fresh.revision),
    )
    .unwrap();
    assert_eq!(d.beats(t, 1, 0).unwrap().len(), 4);
}
