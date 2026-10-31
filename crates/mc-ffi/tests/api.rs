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
