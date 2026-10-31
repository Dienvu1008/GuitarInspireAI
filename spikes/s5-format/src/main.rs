//! Spike S5: ZIP + JSON format size and load time, U1 unknown-content detection,
//! flatten overhead, ZIP safety, and Windows atomic replace behaviour.
#![allow(dead_code)] // plain DTOs exist only to measure parsing cost

mod dto;
mod package;
mod replace;

use package::LoadError;
use serde_json::Value;
use std::time::{Duration, Instant};
use zip::CompressionMethod;

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn main() {
    sizes_and_timing();
    unknown_detection();
    zip_safety();
    atomic_replace();
}

fn sizes_and_timing() {
    println!("## size and timing (median of runs; open = safe unzip + sha256 + parse; excludes domain construction)");
    for (label, t, b, k, n, runs) in [("small", 1, 50, 8, 2, 21), ("typical", 4, 150, 8, 2, 11), ("large", 20, 300, 16, 3, 5)] {
        let score = dto::fixture(t, b, k, n);
        let notes = t * b * k * n;
        let json = serde_json::to_vec(&score).unwrap();
        let pretty = serde_json::to_vec_pretty(&score).unwrap().len();
        let mut saves = Vec::new();
        let mut pkg = Vec::new();
        for _ in 0..runs {
            let t0 = Instant::now();
            let j = serde_json::to_vec(&score).unwrap();
            pkg = package::write(&j);
            saves.push(t0.elapsed());
        }
        let mut opens = Vec::new();
        let mut opens_plain = Vec::new();
        let mut parse_full = Vec::new();
        let mut parse_plain = Vec::new();
        for _ in 0..runs {
            let t0 = Instant::now();
            let bytes = package::open(&pkg).unwrap();
            let t1 = Instant::now();
            let s: dto::full::Score = serde_json::from_slice(&bytes).unwrap();
            let unknown = dto::full::unknown_paths(&s);
            assert!(unknown.is_empty());
            opens.push(t0.elapsed());
            parse_full.push(t1.elapsed());

            let t0 = Instant::now();
            let bytes = package::open(&pkg).unwrap();
            let t1 = Instant::now();
            let p: dto::plain::Score = serde_json::from_slice(&bytes).unwrap();
            std::hint::black_box(&p.tracks.len());
            opens_plain.push(t0.elapsed());
            parse_plain.push(t1.elapsed());
        }
        let (pf, pp) = (median(parse_full), median(parse_plain));
        println!(
            "{label}: notes={notes} json_bytes={} pretty_json_bytes={pretty} package_bytes={} save_ms={:.1} open_ms={:.1} open_plain_ms={:.1} parse_flatten_ms={:.1} parse_plain_ms={:.1} flatten_overhead={:.2}x",
            json.len(),
            pkg.len(),
            ms(median(saves)),
            ms(median(opens)),
            ms(median(opens_plain)),
            ms(pf),
            ms(pp),
            pf.as_secs_f64() / pp.as_secs_f64()
        );
    }
}

/// Injects an unknown key into every object at every DTO level plus an unknown
/// effect variant, and checks U1 reports exactly those paths.
fn unknown_detection() {
    println!("## U1 unknown-content detection");
    let score = dto::fixture(2, 3, 2, 2);
    let mut v: Value = serde_json::to_value(&score).unwrap();
    let mut expected = Vec::new();
    fn inject(v: &mut Value, path: String, expected: &mut Vec<String>) {
        match v {
            Value::Object(map) => {
                let is_effect = map.contains_key("kind");
                if !is_effect {
                    map.insert("x_future".into(), Value::from(1));
                    expected.push(format!("{path}.x_future"));
                }
                for (k, child) in map.iter_mut() {
                    if k != "x_future" {
                        inject(child, format!("{path}.{k}"), expected);
                    }
                }
            }
            Value::Array(items) if items.iter().all(Value::is_object) => {
                for (i, c) in items.iter_mut().enumerate() {
                    inject(c, format!("{path}[{i}]"), expected);
                }
            }
            _ => {}
        }
    }
    inject(&mut v, "$".into(), &mut expected);
    let note = &mut v["tracks"][1]["bars"][2]["voices"][0]["beats"][1]["notes"][0];
    note["effects"] = serde_json::json!([{ "kind": "harmonic", "data": { "type": "natural" } }]);
    expected.push("$.tracks[1].bars[2].voices[0].beats[1].notes[0].effects[0]:harmonic".into());

    let parsed: dto::full::Score = serde_json::from_value(v.clone()).unwrap();
    let mut found = dto::full::unknown_paths(&parsed);
    found.sort();
    expected.sort();
    println!("injected={} detected={} exact_match={}", expected.len(), found.len(), found == expected);
    let levels: std::collections::BTreeSet<String> = expected
        .iter()
        .map(|p| p.split('.').filter(|s| !s.starts_with('x')).map(|s| s.split('[').next().unwrap().to_string()).collect::<Vec<_>>().join("."))
        .collect();
    println!("levels_covered={levels:?}");

    // Re-serialization keeps captured content (basis for U3; not required for MVP).
    let reserialized = serde_json::to_value(&parsed).unwrap();
    println!("roundtrip_value_equal={}", reserialized == v);

    // The plain DTO silently drops the same content: why U1 is needed.
    let plain: dto::plain::Score = serde_json::from_value(v).unwrap();
    println!("plain_dto_accepts_and_drops_unknown_content={}", plain.tracks.len() == 2);
}

fn zip_safety() {
    println!("## ZIP safety");
    let ok = |b: &[u8]| package::read(b, &package::Limits::default()).map(|e| e.len());
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("traversal ../evil.json", package::raw_zip(&[("score.json", b"{}"), ("../evil.json", b"x")], CompressionMethod::Deflated)),
        ("absolute /score.json", package::raw_zip(&[("/score.json", b"x")], CompressionMethod::Deflated)),
        ("drive C:/x", package::raw_zip(&[("C:/x", b"x")], CompressionMethod::Deflated)),
        ("backslash assets\\a", package::raw_zip(&[("assets\\a", b"x")], CompressionMethod::Deflated)),
        ("dot segment assets/./a", package::raw_zip(&[("assets/./a", b"x")], CompressionMethod::Deflated)),
        ("unknown top-level", package::raw_zip(&[("evil.exe", b"x")], CompressionMethod::Deflated)),
        ("upper-case name SCORE.json", package::raw_zip(&[("score.json", b"{}"), ("SCORE.json", b"{}")], CompressionMethod::Deflated)),
        ("case duplicate assets/a.wav + assets/A.wav", package::raw_zip(&[("assets/a.wav", b"1"), ("assets/A.wav", b"2")], CompressionMethod::Deflated)),
        ("truncated zip", {
            let mut z = package::raw_zip(&[("score.json", b"{}")], CompressionMethod::Deflated);
            z.truncate(z.len() / 2);
            z
        }),
    ];
    for (name, bytes) in &cases {
        println!("{name}: {:?}", ok(bytes).map_err(|e| format!("{e:?}").split('(').next().unwrap().to_string()));
    }
    // Decompression bomb: 1 GiB of zeros deflated. Must be rejected early by the ratio cap.
    let zeros = vec![0u8; 1 << 30];
    let bomb = package::raw_zip(&[("assets/bomb.raw", &zeros)], CompressionMethod::Deflated);
    drop(zeros);
    let t0 = Instant::now();
    let r = ok(&bomb);
    println!(
        "bomb 1 GiB zeros (compressed {} bytes): {:?} after {:.0} ms",
        bomb.len(),
        r.map_err(|e| match e {
            LoadError::Ratio(_, n) => format!("Ratio rejected after reading {n} bytes"),
            other => format!("{other:?}"),
        }),
        ms(t0.elapsed())
    );
    // Corrupted payload with a valid ZIP: hash mismatch.
    let mut s = serde_json::to_vec(&dto::fixture(1, 2, 2, 1)).unwrap();
    let good = package::write(&s);
    s[10] ^= 1;
    let mut tampered = package::write(&s);
    let man_good = package::read(&good, &package::Limits::default()).unwrap()[0].1.clone();
    tampered = rebuild_with_manifest(&tampered, &man_good);
    println!("hash mismatch: {:?}", package::open(&tampered).err().map(|e| format!("{e:?}")));
    println!("valid package: {:?}", package::open(&good).map(|b| b.len()).map_err(|e| format!("{e:?}")));
}

/// Replaces the manifest of `pkg` with `manifest`, keeping score.json: simulates corruption.
fn rebuild_with_manifest(pkg: &[u8], manifest: &[u8]) -> Vec<u8> {
    let entries = package::read(pkg, &package::Limits::default()).unwrap();
    let score = entries.iter().find(|(k, _)| k == "score.json").unwrap().1.clone();
    package::raw_zip(&[("manifest.json", manifest), ("score.json", &score)], CompressionMethod::Deflated)
}

fn atomic_replace() {
    use replace::Method::*;
    println!("## Windows atomic replace (os error 5 = access denied, 32 = sharing violation)");
    for method in [StdRename, TempfilePersist, ReplaceFileW] {
        for case in ["unlocked", "lock_full_sharing", "lock_no_share_delete", "lock_released_after_400ms"] {
            let r = replace::run_case(method, case);
            println!(
                "{:?} {}: result={:?} attempts={} ms={} original_intact={} leftover_files={} creation_time_preserved={:?}",
                r.method, r.case, r.result, r.attempts, r.millis, r.original_intact_on_failure, r.temp_files_left, r.creation_time_preserved
            );
        }
    }
}
