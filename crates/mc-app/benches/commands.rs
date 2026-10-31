//! FF-21 command benchmarks (score-model-and-undo section 6). Provisional budget for an
//! edit command (performance-requirements): p99 <= 16 ms on the typical score and
//! <= 33 ms on the large score, including the undo snapshot. Projection time is added
//! in Phase 3 when layout exists.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use mc_app::ports::{Clock, Entropy};
use mc_app::{
    BeatAddress, Command, CommandEnvelope, DocumentId, DocumentSession, ScoreCommand, SessionConfig,
};
use mc_score::fixtures::{LARGE, SMALL, Shape, TYPICAL, reference};
use std::hint::black_box;
use std::time::Instant;

struct C;
impl Clock for C {
    fn now_unix_millis(&self) -> u64 {
        0
    }
}
struct E;
impl Entropy for E {
    fn random_u128(&self) -> u128 {
        0
    }
}

fn session(shape: Shape) -> DocumentSession {
    DocumentSession::new(
        DocumentId(1),
        reference(shape),
        SessionConfig::default(),
        &C,
        &E,
    )
}

/// A beat in the middle of the score, on the middle track.
fn middle_beat(s: &DocumentSession) -> BeatAddress {
    let score = s.score();
    let track = score.track_at(score.track_count() / 2).unwrap();
    let bar = score.master_bar_count() / 2;
    let beat = track.bar(bar).unwrap().voices[0].beats[0].id;
    BeatAddress {
        track: track.id(),
        bar,
        voice: 0,
        beat,
    }
}

fn edit(s: &mut DocumentSession, b: BeatAddress, i: u64) {
    let cmd = Command::Score(ScoreCommand::SetNote {
        beat: b,
        string: 1 + (i % 6) as u8,
        fret: (i % 12) as u8,
    });
    black_box(s.execute(CommandEnvelope::new(i, cmd)).expect("valid edit"));
}

fn percentiles(name: &str, mut samples: Vec<u128>) {
    samples.sort_unstable();
    let p = |q: f64| samples[((samples.len() - 1) as f64 * q) as usize] as f64 / 1000.0;
    eprintln!(
        "{name}: n={} p50={:.1} us p99={:.1} us max={:.1} us",
        samples.len(),
        p(0.5),
        p(0.99),
        p(1.0)
    );
}

fn commands(c: &mut Criterion) {
    let mut group = c.benchmark_group("commands");
    group.sample_size(20);
    for (name, shape) in [("small", SMALL), ("typical", TYPICAL), ("large", LARGE)] {
        let mut s = session(shape);
        let b = middle_beat(&s);
        let mut samples = Vec::with_capacity(1000);
        for i in 0..1000 {
            let t = Instant::now();
            edit(&mut s, b, i);
            samples.push(t.elapsed().as_nanos());
        }
        percentiles(
            &format!("apply_note_edit/{name} (incl. undo snapshot)"),
            samples,
        );
        eprintln!(
            "history after 1000 edits/{name}: depth {} steps, {} estimated bytes",
            s.undo_depth(),
            s.history_bytes()
        );
        let mut i = 1000;
        group.bench_function(BenchmarkId::new("apply_note_edit", name), |bench| {
            bench.iter(|| {
                i += 1;
                edit(&mut s, b, i)
            })
        });
    }

    let mut s = session(LARGE);
    let b = middle_beat(&s);
    for i in 0..100 {
        edit(&mut s, b, i);
    }
    group.bench_function("undo_redo_100/large", |bench| {
        bench.iter(|| {
            for _ in 0..100 {
                s.undo().expect("undo");
            }
            for _ in 0..100 {
                s.redo().expect("redo");
            }
        })
    });

    let large = reference(LARGE);
    let tracks: Vec<_> = large.tracks().map(|t| t.id()).collect();
    let bars = large.master_bar_count();
    group.bench_function("bulk_shift_all_tracks/large", |bench| {
        bench.iter_batched(
            || {
                DocumentSession::new(
                    DocumentId(2),
                    large.clone(),
                    SessionConfig::default(),
                    &C,
                    &E,
                )
            },
            |mut s| {
                for (i, &t) in tracks.iter().enumerate() {
                    let cmd = Command::Score(ScoreCommand::ShiftFrets {
                        track: t,
                        bars: (0, bars),
                        delta: 1,
                    });
                    s.execute(CommandEnvelope::new(i as u64, cmd))
                        .expect("frets stay in range");
                }
                s
            },
            criterion::BatchSize::LargeInput,
        )
    });
    group.bench_function("full_validate/large", |bench| {
        bench.iter(|| black_box(large.validate()))
    });
    group.bench_function("bar_start_times/large", |bench| {
        bench.iter(|| black_box(large.bar_start_times().expect("ok")))
    });
    group.finish();
}

criterion_group!(benches, commands);
criterion_main!(benches);
