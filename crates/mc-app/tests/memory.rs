//! FF-23 memory baselines (structural measurement).
//!
//! Measures deterministic structural heap sizes (element sizes times counts) of the
//! large reference score and of undo steps, and fails if a value exceeds its baseline
//! in benchmarks/baselines/memory.toml by more than 10 percent. Baselines are a
//! protected path: lowering them is free, raising them needs an ADR trailer.
//!
//! An exact counting-allocator cross-check needs `unsafe` (GlobalAlloc), which the
//! workspace forbids; it is proposed as an ADR in the Phase 1 exit report and not
//! added here.

use mc_app::ports::{Clock, Entropy};
use mc_app::{
    BeatAddress, Command, CommandEnvelope, DocumentId, DocumentSession, ScoreCommand, SessionConfig,
};
use mc_score::fixtures::{LARGE, TYPICAL, reference};
use std::collections::BTreeMap;

struct Z;
impl Clock for Z {
    fn now_unix_millis(&self) -> u64 {
        0
    }
}
impl Entropy for Z {
    fn random_u128(&self) -> u128 {
        0
    }
}

fn baselines() -> BTreeMap<String, usize> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../benchmarks/baselines/memory.toml"
    );
    let text = std::fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter_map(|l| {
            let l = l.split('#').next()?.trim();
            let (k, v) = l.split_once('=')?;
            Some((
                k.trim().to_string(),
                v.trim().replace('_', "").parse().ok()?,
            ))
        })
        .collect()
}

fn undo_step_bytes(shape: mc_score::fixtures::Shape) -> usize {
    let mut s = DocumentSession::new(
        DocumentId(1),
        reference(shape),
        SessionConfig::default(),
        &Z,
        &Z,
    );
    let track = s
        .score()
        .track_at(s.score().track_count() / 2)
        .unwrap()
        .id();
    let bars = s.score().master_bar_count();
    for i in 0..100 {
        let bar = (i * 3) % bars;
        let beat = s.score().track(track).unwrap().bar(bar).unwrap().voices[0].beats[0].id;
        let cmd = Command::Score(ScoreCommand::SetNote {
            beat: BeatAddress {
                track,
                bar,
                voice: 0,
                beat,
            },
            string: 1 + (i % 6) as u8,
            fret: (i % 11) as u8,
        });
        s.execute(CommandEnvelope::new(i as u64, cmd))
            .expect("valid");
    }
    s.history_bytes() / s.undo_depth()
}

#[test]
fn ff23_memory_baselines() {
    let mut measured: BTreeMap<&str, usize> = BTreeMap::new();
    measured.insert(
        "large_score_structural_bytes",
        reference(LARGE).heap_bytes(),
    );
    measured.insert(
        "typical_score_structural_bytes",
        reference(TYPICAL).heap_bytes(),
    );
    measured.insert("large_undo_step_bytes", undo_step_bytes(LARGE));
    measured.insert("typical_undo_step_bytes", undo_step_bytes(TYPICAL));

    let baseline = baselines();
    let mut failures = Vec::new();
    for (key, value) in &measured {
        match baseline.get(*key) {
            Some(&b) => {
                eprintln!(
                    "{key} = {value} (baseline {b}, {:+.1}%)",
                    (*value as f64 / b as f64 - 1.0) * 100.0
                );
                if *value as f64 > b as f64 * 1.10 {
                    failures.push(format!(
                        "{key}: {value} exceeds baseline {b} by more than 10%"
                    ));
                }
            }
            None => failures.push(format!(
                "{key} = {value}: no baseline in benchmarks/baselines/memory.toml"
            )),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
