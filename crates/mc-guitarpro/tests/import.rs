//! Import acceptance tests (ADR-036 section 6: G1, G2, G3, G5, G6). The repairs of G4
//! are unit tests in `src/import.rs`, where files can be built field by field.

mod common;

use common::{FIXTURES, describe, fixture};
use mc_guitarpro::{Feature, GpImportError, GpImportIssueKind as K, GpImportReport, import_gp};
use mc_score::Score;
use proptest::prelude::*;

fn import(name: &str) -> (Score, GpImportReport) {
    import_gp(fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn count(r: &GpImportReport, kind: K) -> usize {
    r.issues
        .iter()
        .filter(|i| i.kind == kind)
        .map(|i| i.count)
        .sum()
}

/// The reviewed import of every fixture: score description and report.
fn summary_of_all() -> String {
    let mut out = String::new();
    for (name, bytes) in FIXTURES {
        out.push_str(&format!("== {name}\n"));
        match import_gp(bytes) {
            Ok((s, r)) => {
                out.push_str(&format!(
                    "{} | tracks {} bars {} notes {} of {} dropped {}\n",
                    r.version,
                    r.tracks_imported,
                    r.bars,
                    r.notes_imported,
                    r.notes_in_file,
                    r.notes_dropped
                ));
                for i in &r.issues {
                    out.push_str(&format!("issue: {i}\n"));
                }
                out.push_str(&describe(&s));
                out.push('\n');
            }
            Err(e) => out.push_str(&format!("error: {e}\n")),
        }
    }
    out
}

/// G1: the import of all 54 fixtures equals the reviewed golden (checked against the
/// generator's content by hand on 2026-11-11). Regenerate with
/// `GP_UPDATE_GOLDEN=1 cargo test -p mc-guitarpro --test import`, then review the diff.
#[test]
fn g1_every_fixture_imports_as_reviewed() {
    let actual = summary_of_all();
    if std::env::var("GP_UPDATE_GOLDEN").is_ok() {
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/import.txt"),
            &actual,
        )
        .unwrap();
        return;
    }
    let golden = include_str!("golden/import.txt").replace("\r\n", "\n");
    assert_eq!(
        actual, golden,
        "the import differs from the reviewed golden"
    );
}

/// G1: the facts of each fixture, stated directly (not only through the golden).
#[test]
fn g1_fixture_facts() {
    for ext in ["gp3", "gp4", "gp5"] {
        let f = |n: &str| format!("{n}.{ext}");
        let (s, r) = import(&f("f01-empty"));
        assert_eq!(
            (s.tracks().len(), s.master_bars().len(), s.note_count()),
            (1, 1, 0)
        );
        assert_eq!(r.tracks_imported, 1);

        let (s, _) = import(&f("f03-chord"));
        let d = describe(&s);
        assert!(
            d.contains("1/4 (1/0, 2/1, 3/0, 4/2, 5/3) | 1/4 rest"),
            "{d}"
        );

        let (s, _) = import(&f("f04-durations"));
        let d = describe(&s);
        assert!(
            d.contains(
                "bar 1 voice 1: 1/2 (1/0) | 1/4 (1/1) | 1/8 (1/2) | 1/16 (1/3) | 1/16 (1/5)"
            ),
            "{d}"
        );
        assert!(d.contains("bar 2 voice 1: 1/4. (2/0) | 1/8 (2/1) | 1/8 3:2 (2/3) | 1/8 3:2 (2/5) | 1/8 3:2 (2/7) | 1/4 rest"), "{d}");

        let (s, _) = import(&f("f07-tracks"));
        let t: Vec<_> = s.tracks().collect();
        assert_eq!((t[0].name(), t[1].name()), ("Lead", "Rhythm"));
        let tuning: Vec<i32> = t[1]
            .fretted()
            .tuning
            .strings()
            .iter()
            .map(|p| p.semitone())
            .collect();
        assert_eq!(tuning, [64, 59, 55, 50, 45, 38]);
        assert_eq!(t[1].fretted().capo.0, 2);

        let (s, _) = import(&f("f08-meter-key-tempo"));
        let ts: Vec<_> = s
            .master_bars()
            .iter()
            .map(|b| {
                (
                    b.time_signature.numerator(),
                    b.time_signature.denominator(),
                    b.key.fifths(),
                )
            })
            .collect();
        assert_eq!(ts, [(4, 4, 1), (3, 4, 1), (6, 8, 1)]);
        let tempo: Vec<_> = s
            .tempo_map()
            .iter()
            .map(|t| (t.bar, t.tempo.bpm().numerator()))
            .collect();
        assert_eq!(tempo, [(0, 90), (2, 140)]);

        let (s, _) = import(&f("f09-repeats"));
        let b = s.master_bars();
        assert!(b[1].repeat_start);
        assert_eq!(b[3].repeat_end, Some(2));
        assert_eq!(b[1].marker.as_deref(), Some("Chorus"));

        let (s, _) = import(&f("f13-seven-string"));
        assert_eq!(
            s.tracks().next().unwrap().fretted().tuning.string_count(),
            7
        );
        assert!(describe(&s).contains("(7/0)"));

        assert_eq!(
            import_gp(fixture(&f("f14-percussion"))),
            Err(GpImportError::NoGuitarTracks)
        );
    }
    // GP4 and GP5 keep the endings; TuxGuitar's GP3 writer drops them.
    for name in ["f09-repeats.gp4", "f09-repeats.gp5"] {
        let (s, _) = import(name);
        assert_eq!(s.master_bars()[3].alternate_endings, [1]);
        assert_eq!(s.master_bars()[4].alternate_endings, [2]);
    }
    // f05 in GP5 has every note flag; ghost and staccato are reported.
    let (s, r) = import("f05-note-effects.gp5");
    let d = describe(&s);
    assert!(
        d.contains("bar 1 voice 1: 1/4 (2/5) | 1/4 (2/5 acc) | 1/4 (2/5 acc) | 1/4 (2/5 dead)"),
        "{d}"
    );
    assert!(
        d.contains("bar 2 voice 1: 1/4 (2/5 pm) | 1/4 (2/5 ring) | 1/4 (2/5) | 1/4 (2/5)"),
        "{d}"
    );
    assert!(
        d.contains("bar 3 voice 1: 1/2 (2/5) | 1/2 (2/5 tie)"),
        "{d}"
    );
    assert_eq!(count(&r, K::NotKept(Feature::GhostNote)), 1);
    assert_eq!(count(&r, K::NotKept(Feature::Staccato)), 1);
}

/// G2: notes in the file = imported + dropped, on every fixture; each feature that is
/// not kept is reported by kind with its first bar.
#[test]
fn g2_nothing_is_lost_silently() {
    for (name, bytes) in FIXTURES {
        if let Ok((s, r)) = import_gp(bytes) {
            assert_eq!(r.notes_imported, s.note_count(), "{name}");
            assert_eq!(
                r.notes_in_file,
                r.notes_imported + r.notes_dropped,
                "{name}"
            );
        }
    }
    let (_, r) = import("f12-unsupported.gp5");
    for (f, bar) in [
        (Feature::Bend, 0),
        (Feature::Slide, 0),
        (Feature::HammerOnPullOff, 0),
        (Feature::Harmonic, 0),
        (Feature::Vibrato, 1),
        (Feature::GraceNote, 1),
        (Feature::TremoloPicking, 1),
        (Feature::Tap, 1),
    ] {
        let issue = r
            .issues
            .iter()
            .find(|i| i.kind == K::NotKept(f))
            .unwrap_or_else(|| panic!("{f:?}"));
        assert_eq!((issue.count, issue.first_bar), (1, bar), "{f:?}");
    }
    for (name, f) in [
        ("f11-text.gp5", Feature::SongInformation),
        ("f15-chord-diagram.gp5", Feature::ChordDiagram),
        ("f15-chord-diagram.gp4", Feature::ChordDiagram),
        ("f16-beat-text.gp5", Feature::BeatText),
        ("f16-beat-text.gp3", Feature::BeatText),
        ("f18-lyrics.gp5", Feature::Lyrics),
        ("f18-lyrics.gp4", Feature::Lyrics),
    ] {
        assert_eq!(count(&import(name).1, K::NotKept(f)), 1, "{name}");
    }
}

/// G2: a percussion track next to a guitar track is skipped and its notes counted.
/// The file is f07 with the drum flag set on its second track.
#[test]
fn g2_a_percussion_track_is_skipped_and_counted() {
    let mut bytes = fixture("f07-tracks.gp5").to_vec();
    let at = bytes.windows(6).position(|w| w == b"Rhythm").unwrap();
    bytes[at - 2] |= 0x01;
    let (s, r) = import_gp(&bytes).unwrap();
    assert_eq!(s.tracks().len(), 1);
    assert_eq!(count(&r, K::PercussionTrackSkipped), 1);
    assert_eq!(
        (r.notes_in_file, r.notes_imported, r.notes_dropped),
        (4, 2, 2)
    );
}

/// G3, version rule: the time signature is carried forward (GP4 writes only the
/// numerator when only it changes).
#[test]
fn g3_time_signature_is_carried_forward() {
    for ext in ["gp3", "gp4", "gp5"] {
        let (s, _) = import(&format!("f08-meter-key-tempo.{ext}"));
        assert_eq!(s.master_bars()[1].time_signature.denominator(), 4, "{ext}");
    }
}

/// G3, version rule: GP5 stores the plays, GP3 and GP4 the repeats.
#[test]
fn g3_repeat_count_per_version() {
    for ext in ["gp3", "gp4", "gp5"] {
        assert_eq!(
            import(&format!("f09-repeats.{ext}")).0.master_bars()[3].repeat_end,
            Some(2)
        );
        assert_eq!(
            import(&format!("f17-combined-endings.{ext}"))
                .0
                .master_bars()[1]
                .repeat_end,
            Some(3)
        );
    }
}

/// G3, version rule: the alternate-ending byte is a bitmask of passes.
#[test]
fn g3_endings_are_a_bitmask_of_passes() {
    for ext in ["gp4", "gp5"] {
        let (s, _) = import(&format!("f17-combined-endings.{ext}"));
        assert_eq!(s.master_bars()[1].alternate_endings, [1, 2]);
        assert_eq!(s.master_bars()[2].alternate_endings, [3]);
    }
}

/// G3, version rule: pick stroke 2 is down and 1 is up; a brush becomes a stroke in the
/// direction TuxGuitar reads (ADR-036 choice 7.1). GP3 has no pick stroke.
#[test]
fn g3_pick_stroke_values_and_brush_direction() {
    for ext in ["gp4", "gp5"] {
        let (s, r) = import(&format!("f06-strokes.{ext}"));
        assert!(describe(&s).contains("1/4 down (1/0, 2/1, 3/0, 4/2, 5/3) | 1/4 up (1/0, 2/1, 3/0, 4/2, 5/3) | 1/4 down (1/0, 2/1, 3/0, 4/2, 5/3) | 1/4 (1/0"));
        assert_eq!(count(&r, K::Approximated(Feature::Brush)), 1);
    }
    let (s, _) = import("f06-strokes.gp3");
    assert!(
        describe(&s)
            .contains("1/4 (1/0, 2/1, 3/0, 4/2, 5/3) | 1/4 (1/0, 2/1, 3/0, 4/2, 5/3) | 1/4 down")
    );
}

/// G3, version rule: GP3 and GP4 have one voice; GP5 two.
#[test]
fn g3_one_voice_in_gp3_and_gp4() {
    let (s, _) = import("f10-two-voices.gp5");
    let bar = s.tracks().next().unwrap().bar(0).unwrap().voices.len();
    assert_eq!(bar, 2);
    for ext in ["gp3", "gp4"] {
        let (s, _) = import(&format!("f10-two-voices.{ext}"));
        assert_eq!(s.tracks().next().unwrap().bar(0).unwrap().voices.len(), 1);
    }
}

/// G3: a heavy accent is kept as an accent and counted.
#[test]
fn g3_heavy_accent_is_approximated() {
    let (_, r) = import("f05-note-effects.gp5");
    assert_eq!(count(&r, K::Approximated(Feature::HeavyAccent)), 1);
}

/// G5: at least 100 000 mutated fixtures give a score or a typed error, never a panic
/// (deterministic xorshift, as in the S10 spike).
#[test]
fn g5_mutated_fixtures_never_panic() {
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let rounds = 1_900; // x 54 fixtures = 102 600 inputs
    let (mut ok, mut errors) = (0usize, 0usize);
    for (_, original) in FIXTURES {
        for _ in 0..rounds {
            let mut data = original.to_vec();
            for _ in 0..1 + next() % 8 {
                let at = (next() % data.len() as u64) as usize;
                data[at] = next() as u8;
            }
            if next() % 4 == 0 {
                data.truncate((next() % data.len() as u64) as usize);
            }
            match import_gp(&data) {
                Ok((s, r)) => {
                    assert_eq!(r.notes_imported, s.note_count());
                    ok += 1;
                }
                Err(GpImportError::Invalid(v)) => panic!("an invalid score was built: {v:?}"),
                Err(_) => errors += 1,
            }
        }
    }
    assert_eq!(ok + errors, rounds * FIXTURES.len());
    assert!(ok > 0 && errors > 0, "{ok} read, {errors} errors");
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2_000, .. ProptestConfig::default() })]

    /// G5: arbitrary bytes never panic, also behind a valid version string.
    #[test]
    fn g5_arbitrary_bytes_never_panic(tail in proptest::collection::vec(any::<u8>(), 0..4096), version in 0usize..4) {
        let mut data = Vec::new();
        if version > 0 {
            let v = ["", "FICHIER GUITAR PRO v3.00", "FICHIER GUITAR PRO v4.06", "FICHIER GUITAR PRO v5.00"][version];
            data.push(v.len() as u8);
            data.extend_from_slice(v.as_bytes());
            data.resize(31, 0);
        }
        data.extend_from_slice(&tail);
        let r = import_gp(&data);
        prop_assert!(!matches!(r, Err(GpImportError::Invalid(_))));
    }
}

/// G5: the file size limit (the other limits are unit tests on hand-built files).
#[test]
fn g5_file_size_limit() {
    let big = vec![0u8; mc_guitarpro::MAX_FILE_BYTES + 1];
    assert!(matches!(
        import_gp(&big),
        Err(GpImportError::TooLarge { .. })
    ));
}

/// G6: unsupported versions and other files give a typed error naming the cause.
#[test]
fn g6_unsupported_files_are_named() {
    let header = |v: &str| {
        let mut d = vec![v.len() as u8];
        d.extend_from_slice(v.as_bytes());
        d.resize(31, 0);
        d.extend_from_slice(&[0; 64]);
        d
    };
    let cases: [(Vec<u8>, &str); 7] = [
        (header("FICHIER GUITAR PRO v5.10"), "v5.10"),
        (header("FICHIER GUITAR PRO v2.21"), "v2.21"),
        (
            header("CLIPBOARD GUITAR PRO v5.00"),
            "not a Guitar Pro file",
        ),
        (b"PK\x03\x04rest".to_vec(), "Guitar Pro 7"),
        (b"BCFZ\x00\x00".to_vec(), "Guitar Pro 6"),
        (Vec::new(), "not a Guitar Pro file"),
        (vec![24, b'F'], "not a Guitar Pro file"),
    ];
    for (data, cause) in cases {
        match import_gp(&data) {
            Err(e @ GpImportError::UnsupportedVersion(_)) => {
                assert!(e.to_string().contains(cause), "{e}")
            }
            other => panic!("{cause}: {other:?}"),
        }
    }
    // A truncated file of a supported version is malformed, with the offset.
    let short = &fixture("f02-one-note.gp5")[..200];
    assert!(matches!(
        import_gp(short),
        Err(GpImportError::Malformed { .. })
    ));
}
