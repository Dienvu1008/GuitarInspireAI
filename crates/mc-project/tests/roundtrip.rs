//! FF-11 round-trip suite (project-format section 14) and FF-32 schema snapshot.
//!
//! Fixtures and schema snapshots are embedded at compile time, so this test does no
//! file I/O (FF-03 applies to every target of this domain crate). They are append-only
//! (protected paths) and are created with `mc-cli format-fixtures crates/mc-project`,
//! which never overwrites existing files. FF-20 checks that every file under
//! tests/fixtures/ is listed in `FIXTURES` below.

use mc_project::{
    AppInfo, FORMAT_VERSION, Limits, LoadError, LoadWarning, ProjectId, SaveInfo, load,
    load_with_limits, save, schemas, score_json, sha256_hex, write_entries,
};
use mc_score::Score;
use mc_score::fixtures::{SMALL, feature_rich, reference, techniques};

/// (format version, fixture name, package bytes, golden score.json bytes) for every
/// released version. Append when a new version is released; never edit entries.
const FIXTURES: &[(&str, &str, &[u8], &[u8])] = &[
    (
        "1.0",
        "small",
        include_bytes!("fixtures/v1.0/small.package"),
        include_bytes!("fixtures/v1.0/small.score.json"),
    ),
    (
        "1.0",
        "feature-rich",
        include_bytes!("fixtures/v1.0/feature-rich.package"),
        include_bytes!("fixtures/v1.0/feature-rich.score.json"),
    ),
    (
        "1.1",
        "small",
        include_bytes!("fixtures/v1.1/small.package"),
        include_bytes!("fixtures/v1.1/small.score.json"),
    ),
    (
        "1.1",
        "feature-rich",
        include_bytes!("fixtures/v1.1/feature-rich.package"),
        include_bytes!("fixtures/v1.1/feature-rich.score.json"),
    ),
    (
        "1.1",
        "techniques",
        include_bytes!("fixtures/v1.1/techniques.package"),
        include_bytes!("fixtures/v1.1/techniques.score.json"),
    ),
];

/// (format version, schema name, snapshot) for every released version.
const SCHEMAS: &[(&str, &str, &str)] = &[
    (
        "1.0",
        "score",
        include_str!("../schema/score-1.0.schema.json"),
    ),
    (
        "1.0",
        "manifest",
        include_str!("../schema/manifest-1.0.schema.json"),
    ),
    (
        "1.1",
        "score",
        include_str!("../schema/score-1.1.schema.json"),
    ),
    (
        "1.1",
        "manifest",
        include_str!("../schema/manifest-1.1.schema.json"),
    ),
];

fn current_version() -> String {
    format!("{}.{}", FORMAT_VERSION.major, FORMAT_VERSION.minor)
}

fn info() -> SaveInfo {
    let app = AppInfo {
        app: "GuitarInspireAI".into(),
        version: "0.1.0".into(),
    };
    SaveInfo {
        project_id: ProjectId::from_parts(
            1_767_225_600_000,
            0x1234_5678_9abc_def0_1234_5678_9abc_def0,
        ),
        created_by: app.clone(),
        saved_by: app,
        saved_revision: 7,
    }
}

fn builders() -> Vec<(&'static str, Score)> {
    vec![
        ("small", reference(SMALL)),
        ("feature-rich", feature_rich()),
        ("techniques", techniques()),
    ]
}

// 1. Every fixture of every released version loads and validates.
#[test]
fn rt1_p03_p05_every_fixture_of_every_version_loads() {
    for (version, name, package, _) in FIXTURES {
        let loaded = load(package).unwrap_or_else(|e| panic!("v{version}/{name}: {e}"));
        assert!(loaded.score.validate().is_empty(), "v{version}/{name}");
        assert!(
            loaded.warnings.is_empty(),
            "v{version}/{name}: {:?}",
            loaded.warnings
        );
    }
    let current: Vec<_> = FIXTURES
        .iter()
        .filter(|f| f.0 == current_version())
        .map(|f| f.1)
        .collect();
    for (name, _) in builders() {
        assert!(
            current.contains(&name),
            "no v{} fixture for '{name}'; run mc-cli format-fixtures",
            current_version()
        );
    }
}

// 2. Load, save, load gives semantic equality.
#[test]
fn rt2_load_save_load_is_semantically_equal() {
    for (name, score) in builders() {
        let first = load(&save(&score, &info())).unwrap();
        assert_eq!(first.score, score, "{name}");
        let second = load(&save(&first.score, &info())).unwrap();
        assert_eq!(second.score, first.score, "{name}");
        assert_eq!(second.project_id, info().project_id.as_str());
        assert_eq!(second.saved_revision, 7);
    }
}

// 3. Current version output is byte-stable against golden files.
#[test]
fn rt3_current_output_is_byte_stable() {
    for (name, score) in builders() {
        let (_, _, package, golden) = FIXTURES
            .iter()
            .find(|f| f.0 == current_version() && f.1 == name)
            .expect("fixture exists (rt1)");
        assert_eq!(
            score_json(&score),
            *golden,
            "{name}: score.json output changed; this is a format change (bump FORMAT_VERSION)"
        );
        let loaded = load(package).unwrap();
        assert_eq!(
            score_json(&loaded.score),
            *golden,
            "{name}: fixture does not re-serialize identically"
        );
    }
}

/// Inserts `x_future` into every JSON object of the tree and returns the expected paths.
fn inject(v: &mut serde_json::Value, path: String, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(map) => {
            for (k, child) in map.iter_mut() {
                inject(child, format!("{path}.{k}"), out);
            }
            map.insert("x_future".into(), serde_json::json!({"nested": [1, 2]}));
            out.push(format!("{path}.x_future"));
        }
        serde_json::Value::Array(items) => {
            for (i, c) in items.iter_mut().enumerate() {
                inject(c, format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}

fn entries(pkg: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let e = mc_project::read_entries(pkg, &Limits::default()).unwrap();
    (e[0].1.clone(), e[1].1.clone())
}

fn repackage(
    score_value: &serde_json::Value,
    manifest_edit: impl FnOnce(&mut serde_json::Value),
) -> Vec<u8> {
    let score_bytes = serde_json::to_vec(score_value).unwrap();
    let (manifest, _) = entries(&save(&feature_rich(), &info()));
    let mut manifest: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    manifest["entries"][0]["sha256"] = sha256_hex(&score_bytes).into();
    manifest["entries"][0]["size"] = (score_bytes.len() as u64).into();
    manifest_edit(&mut manifest);
    write_entries(&[
        ("manifest.json", &serde_json::to_vec(&manifest).unwrap()),
        ("score.json", &score_bytes),
    ])
}

fn dto_value() -> serde_json::Value {
    serde_json::to_value(mc_project::to_dto(&feature_rich())).unwrap()
}

// 4a. Unknown content at every DTO level is detected with exact paths (U1).
#[test]
fn rt4a_unknown_fields_at_every_level_are_detected() {
    let score = feature_rich();
    let mut v = dto_value();
    let mut expected = Vec::new();
    inject(&mut v, "$".into(), &mut expected);
    let pkg = repackage(&v, |m| {
        m["x_manifest_future"] = true.into();
        m["optional_features"] = serde_json::json!(["chord-diagrams"]);
    });
    let loaded = load(&pkg).unwrap();
    assert_eq!(loaded.score, score, "known content still loads unchanged");
    let mut found = loaded.unknown_content().to_vec();
    expected.push("manifest.x_manifest_future".into());
    expected.push("optional_feature:chord-diagrams".into());
    found.sort();
    expected.sort();
    assert_eq!(found, expected);
    for level in [
        "$.x_future",
        "master_bars[0].x_future",
        "key.x_future",
        "tempo_map[0].x_future",
        "tracks[0].x_future",
        "instrument.x_future",
        "bars[0].x_future",
        "voices[0].x_future",
        "beats[0].x_future",
        "value.x_future",
        "notes[0].x_future",
    ] {
        assert!(
            found.iter().any(|p| p.ends_with(level)),
            "no unknown field detected at level {level}"
        );
    }
}

#[test]
fn rt4a_reserved_entries_are_unknown_content() {
    let (manifest, score) = entries(&save(&feature_rich(), &info()));
    let mut m: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    let asset = b"RIFF....";
    m["entries"].as_array_mut().unwrap().push(serde_json::json!({"path": "assets/take1.wav", "sha256": sha256_hex(asset), "size": asset.len()}));
    let pkg = write_entries(&[
        ("manifest.json", &serde_json::to_vec(&m).unwrap()),
        ("score.json", &score),
        ("assets/take1.wav", asset),
    ]);
    assert_eq!(
        load(&pkg).unwrap().unknown_content(),
        ["package:assets/take1.wav"]
    );
}

#[test]
fn rt4a_newer_minor_version_loads_with_a_warning() {
    let pkg = repackage(&dto_value(), |m| m["format_version"]["minor"] = 3.into());
    let loaded = load(&pkg).unwrap();
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| matches!(w, LoadWarning::NewerMinorVersion { found } if found.minor == 3))
    );
}

// 5. Migration chain. 1.1 (ADR-035) only adds optional fields, so the 1.0 to 1.1
//    migration is the identity: a 1.0 file loads to the same score as the 1.1 file of
//    the same name, with every technique flag off. Major 0 is refused.
#[test]
fn rt5_no_older_major_versions_exist_and_major_zero_is_refused() {
    let pkg = repackage(&dto_value(), |m| m["format_version"]["major"] = 0.into());
    assert!(matches!(
        load(&pkg),
        Err(LoadError::UnsupportedVersion { major: 0, .. })
    ));
    let mut versions: Vec<_> = FIXTURES.iter().map(|f| f.0).collect();
    versions.dedup();
    assert_eq!(
        versions,
        ["1.0", "1.1"],
        "when another version exists, add migration fixtures and tests here"
    );
}

#[test]
fn rt5_tq3_v1_0_fixtures_load_unchanged_in_1_1_with_every_technique_off() {
    for (_, name, package, golden) in FIXTURES.iter().filter(|f| f.0 == "1.0") {
        let old = load(package).unwrap().score;
        let (_, _, new_package, new_golden) = FIXTURES
            .iter()
            .find(|f| f.0 == "1.1" && f.1 == *name)
            .expect("same fixture in 1.1");
        assert_eq!(old, load(new_package).unwrap().score, "{name}");
        // Additive minor version: the score entry did not change by one byte.
        assert_eq!(
            golden, new_golden,
            "{name}: score.json identical in 1.0 and 1.1"
        );
        for t in old.tracks() {
            for bar in t.bars() {
                for b in bar.voices.iter().flat_map(|v| &v.beats) {
                    assert_eq!(b.stroke, None, "{name}");
                    for n in &b.notes {
                        let e = n.effects;
                        assert!(!e.accent && !e.palm_mute && !e.string_percussive, "{name}");
                    }
                }
            }
        }
    }
}

/// TQ-3: every stored technique flag and stroke survives save and load, and only the
/// flags that are set are written (absent means off).
#[test]
fn tq3_every_technique_flag_and_stroke_round_trips() {
    let score = techniques();
    let loaded = load(&save(&score, &info())).unwrap().score;
    assert_eq!(loaded, score);
    let json = String::from_utf8(score_json(&score)).unwrap();
    for (key, want) in [
        ("\"accent\":true", 11),
        ("\"palm_mute\":true", 4),
        ("\"string_percussive\":true", 6),
        ("\"dead\":true", 2),
        ("\"let_ring\":true", 3),
        ("\"stroke\":\"down\"", 3),
        ("\"stroke\":\"up\"", 1),
    ] {
        assert_eq!(json.matches(key).count(), want, "{key}");
    }
    assert!(!json.contains(":false"), "off flags are omitted");
    assert!(!json.contains("\"stroke\":null"));
}

// 6. Newer major and unknown required features are refused.
#[test]
fn rt6_p03_newer_major_and_unknown_required_features_are_refused() {
    let v = dto_value();
    assert_eq!(
        load(&repackage(&v, |m| m["format_version"]["major"] = 2.into())).unwrap_err(),
        LoadError::NewerMajorVersion {
            found: 2,
            supported: 1
        }
    );
    assert_eq!(
        load(&repackage(&v, |m| m["required_features"] =
            serde_json::json!(["polymeter"])))
        .unwrap_err(),
        LoadError::UnknownRequiredFeatures(vec!["polymeter".into()])
    );
    assert!(matches!(
        load(&repackage(&v, |m| m["format"] = "something.else".into())),
        Err(LoadError::NotAProject(_))
    ));
}

// 7. Corruption and malicious packages give typed errors and no panic.
#[test]
fn rt7_p02_p06_corruption_and_malicious_packages_are_rejected() {
    let good = save(&feature_rich(), &info());
    let (manifest, score) = entries(&good);

    let mut tampered = score.clone();
    let i = tampered.iter().position(|&b| b == b'5').unwrap();
    tampered[i] = b'6';
    let pkg = write_entries(&[("manifest.json", &manifest), ("score.json", &tampered)]);
    assert_eq!(
        load(&pkg).unwrap_err(),
        LoadError::HashMismatch("score.json".into())
    );

    let mut truncated = good.clone();
    truncated.truncate(good.len() / 2);
    assert!(matches!(load(&truncated), Err(LoadError::Container(_))));

    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "traversal",
            write_entries(&[
                ("manifest.json", &manifest),
                ("score.json", &score),
                ("../evil", b"x"),
            ]),
        ),
        (
            "absolute",
            write_entries(&[("manifest.json", &manifest), ("/score.json", &score)]),
        ),
        ("drive", write_entries(&[("C:/x", b"x")])),
        ("backslash", write_entries(&[("assets\\a", b"x")])),
        (
            "unknown top-level",
            write_entries(&[
                ("manifest.json", &manifest),
                ("score.json", &score),
                ("evil.exe", b"x"),
            ]),
        ),
    ];
    for (name, pkg) in cases {
        assert!(
            matches!(load(&pkg), Err(LoadError::BadEntryName(_))),
            "{name}"
        );
    }
    let dup = write_entries(&[
        ("manifest.json", &manifest),
        ("score.json", &score),
        ("assets/a", b"1"),
        ("assets/A", b"2"),
    ]);
    assert!(matches!(load(&dup), Err(LoadError::DuplicateEntry(_))));
    let unlisted = write_entries(&[
        ("manifest.json", &manifest),
        ("score.json", &score),
        ("assets/a", b"1"),
    ]);
    assert_eq!(
        load(&unlisted).unwrap_err(),
        LoadError::UnlistedEntry("assets/a".into())
    );
    assert_eq!(
        load(&write_entries(&[("manifest.json", &manifest)])).unwrap_err(),
        LoadError::MissingEntry("score.json".into())
    );
    assert!(matches!(
        load(&write_entries(&[
            ("manifest.json", b"{not json"),
            ("score.json", &score)
        ])),
        Err(LoadError::Json { .. })
    ));
}

#[test]
fn rt7_decompression_bomb_is_rejected_early() {
    let zeros = vec![0u8; 64 << 20];
    let bomb = write_entries(&[("assets/bomb.raw", &zeros)]);
    assert!(bomb.len() < 1 << 20);
    assert_eq!(
        load(&bomb).unwrap_err(),
        LoadError::CompressionRatio("assets/bomb.raw".into())
    );
    let tight = Limits {
        max_entry_bytes: 1024,
        ..Limits::default()
    };
    assert!(matches!(
        load_with_limits(&save(&feature_rich(), &info()), &tight),
        Err(LoadError::EntryTooLarge(_))
    ));
}

#[test]
fn rt7_p05_domain_violations_in_files_are_reported_with_paths() {
    // Value-object errors are reported with JSON paths.
    let mut v = dto_value();
    v["master_bars"][4]["time_signature"] = serde_json::json!([6, 7]);
    let LoadError::Invalid(violations) = load(&repackage(&v, |_| {})).unwrap_err() else {
        panic!("expected Invalid")
    };
    assert!(
        violations
            .iter()
            .any(|x| x.location == "$.master_bars[4].time_signature"),
        "{violations:?}"
    );
    // Domain invariants are checked once all values construct.
    let mut v = dto_value();
    v["tracks"][0]["bars"][1]["voices"][0]["beats"][1]["notes"][0]["string"] = 9.into();
    let LoadError::Invalid(violations) = load(&repackage(&v, |_| {})).unwrap_err() else {
        panic!("expected Invalid")
    };
    assert!(
        violations.iter().any(|x| x.rule == "H-03"),
        "{violations:?}"
    );
}

// 8. FF-32: the DTO schema of a released version can never change.
#[test]
fn rt8_ff32_schema_snapshot_requires_a_version_bump() {
    let version = current_version();
    for (name, schema) in schemas() {
        let snapshot = SCHEMAS
            .iter()
            .find(|s| s.0 == version && s.1 == name)
            .unwrap_or_else(|| panic!("no schema snapshot for {name} {version}; run mc-cli format-fixtures crates/mc-project"))
            .2;
        assert_eq!(
            snapshot.replace("\r\n", "\n").trim_end(),
            schema.trim_end(),
            "DTO schema for format {version} changed ({name}). Released schemas are append-only: bump FORMAT_VERSION \
             (minor for additive optional fields, major otherwise), generate the new snapshot and add a migration if major."
        );
    }
}

// 9. Fuzzing proper (cargo-fuzz) is FF-15 in Phase 4. Until then, property tests mutate
//    valid packages and check that loading never panics.
mod mutation {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn rt9_mutated_packages_never_panic(edits in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..8)) {
            let mut pkg = save(&feature_rich(), &info());
            for (i, b) in edits {
                let at = i.index(pkg.len());
                pkg[at] = b;
            }
            let _ = load(&pkg);
        }

        #[test]
        fn rt9_mutated_score_json_never_panics(edits in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..8)) {
            let mut json = score_json(&feature_rich());
            for (i, b) in edits {
                let at = i.index(json.len());
                json[at] = b;
            }
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&json) {
                let _ = load(&repackage(&v, |_| {}));
            }
        }
    }
}
