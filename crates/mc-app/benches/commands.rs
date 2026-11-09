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

/// Phase 2 acceptance: voicing search under 50 ms for one chord [Assumption]. The
/// heaviest Phase 2 chord (five tones, every result kept) is measured alongside a triad.
fn guitar_queries(c: &mut Criterion) {
    use mc_app::guitar::{
        IdentifyOptions, SearchOptions, chord_voicings, identify_shape, parse_tuning,
    };
    let tuning = parse_tuning("standard").expect("preset");
    let all = SearchOptions {
        max_results: usize::MAX,
        ..SearchOptions::default()
    };
    for symbol in ["C", "C9", "Cdim7"] {
        let mut samples = Vec::with_capacity(200);
        for _ in 0..200 {
            let t = Instant::now();
            black_box(chord_voicings(symbol, &tuning, 0, &all).expect("valid"));
            samples.push(t.elapsed().as_nanos());
        }
        percentiles(&format!("chord_voicings/{symbol} (budget 50 ms)"), samples);
    }
    let mut group = c.benchmark_group("guitar");
    group.sample_size(20);
    group.bench_function("chord_voicings/C9", |bench| {
        bench.iter(|| black_box(chord_voicings("C9", &tuning, 0, &all).expect("valid")))
    });
    group.bench_function("identify/x02010", |bench| {
        bench.iter(|| {
            black_box(identify_shape("x02010", &tuning, 0, IdentifyOptions::default()).expect("ok"))
        })
    });
    group.finish();
}

/// Phase 3 edit feedback (performance-requirements): a note edit plus TAB relayout of
/// the edited track must stay within the frame-scale budget (p99 <= 16 ms typical,
/// <= 33 ms large). Painting and FFI transfer are measured in the app (Phase 3c/3f).
fn notation_layout(c: &mut Criterion) {
    use mc_app::notation::LayoutMetrics;
    let metrics = LayoutMetrics::default();
    for (name, shape) in [("typical", TYPICAL), ("large", LARGE)] {
        let mut s = session(shape);
        let b = middle_beat(&s);
        let mut samples = Vec::with_capacity(300);
        for i in 0..300 {
            let t = Instant::now();
            edit(&mut s, b, i);
            black_box(s.tab_layout(b.track, &metrics).expect("layout"));
            samples.push(t.elapsed().as_nanos());
        }
        percentiles(
            &format!(
                "edit_and_relayout/{name} (budget {})",
                if name == "large" { "33 ms" } else { "16 ms" }
            ),
            samples,
        );
        let layout = s.tab_layout(b.track, &metrics).expect("layout");
        let glyphs: usize = layout.systems.iter().map(|x| x.glyphs.len()).sum();
        let hits: usize = layout.systems.iter().map(|x| x.hits.len()).sum();
        eprintln!(
            "tab_layout/{name}: {} systems, {glyphs} glyphs, {hits} hit regions",
            layout.systems.len()
        );
    }
    // ADR-031 P1 to P3: a note edit plus multi-track relayout of the shown tracks.
    for (name, shape, shown, budget) in [
        ("P1 typical/all 4 tracks", TYPICAL, 4, "16 ms"),
        ("P2 large/4 tracks", LARGE, 4, "33 ms"),
        ("P3 large/all 20 tracks", LARGE, 20, "33 ms"),
    ] {
        let mut s = session(shape);
        let b = middle_beat(&s);
        let tracks: Vec<_> = s.score().tracks().map(|t| t.id()).take(shown).collect();
        // The edited beat is on a shown track in every case.
        let tracks = if tracks.contains(&b.track) {
            tracks
        } else {
            let mut t = tracks;
            t[0] = b.track;
            t
        };
        let mut samples = Vec::with_capacity(100);
        for i in 0..100 {
            let t = Instant::now();
            edit(&mut s, b, i);
            black_box(s.score_layout(&tracks, &metrics).expect("layout"));
            samples.push(t.elapsed().as_nanos());
        }
        percentiles(
            &format!("edit_and_score_relayout/{name} full (budget {budget})"),
            samples,
        );
        // What the view does (ADR-031 fallback 2): outline for the whole score, then
        // the systems in view; three systems around the edited bar is more than a
        // screen holds when many tracks are shown.
        let mut samples = Vec::with_capacity(100);
        for i in 0..100 {
            let t = Instant::now();
            edit(&mut s, b, 1000 + i);
            let at = s
                .score_outline(&tracks, &metrics)
                .expect("outline")
                .systems
                .iter()
                .position(|x| (x.first_bar..x.end_bar).contains(&b.bar))
                .expect("bar in a system");
            let from = at.saturating_sub(1);
            black_box(
                s.score_systems(&tracks, &metrics, from..from + 3)
                    .expect("systems"),
            );
            samples.push(t.elapsed().as_nanos());
        }
        percentiles(
            &format!("edit_and_score_relayout/{name} in view (budget {budget})"),
            samples,
        );
    }

    let s = session(LARGE);
    let track = s.score().track_at(0).unwrap().id();
    let mut group = c.benchmark_group("notation");
    group.sample_size(20);
    group.bench_function("tab_layout/large_track", |bench| {
        bench.iter(|| black_box(s.tab_layout(track, &metrics).expect("layout")))
    });
    group.finish();
}

criterion_group!(benches, commands, guitar_queries, notation_layout);
criterion_main!(benches);
