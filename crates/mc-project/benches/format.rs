//! FF-21 format benchmarks (performance-requirements: open typical <= 500 ms,
//! open large <= 2 s, save typical <= 300 ms). Open includes ZIP read, SHA-256,
//! JSON parse, U1 detection and full domain validation.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use mc_project::{AppInfo, ProjectId, SaveInfo, load, save};
use mc_score::fixtures::{LARGE, SMALL, TYPICAL, reference};
use std::hint::black_box;

fn info() -> SaveInfo {
    let app = AppInfo {
        app: "bench".into(),
        version: "0".into(),
    };
    SaveInfo {
        project_id: ProjectId::from_parts(0, 0),
        created_by: app.clone(),
        saved_by: app,
        saved_revision: 1,
    }
}

fn format(c: &mut Criterion) {
    let mut group = c.benchmark_group("format");
    group.sample_size(10);
    for (name, shape) in [("small", SMALL), ("typical", TYPICAL), ("large", LARGE)] {
        let score = reference(shape);
        let bytes = save(&score, &info());
        eprintln!(
            "{name}: {} notes, package {} bytes",
            score.note_count(),
            bytes.len()
        );
        group.bench_with_input(BenchmarkId::new("save", name), &score, |b, s| {
            b.iter(|| black_box(save(s, &info())))
        });
        group.bench_with_input(BenchmarkId::new("open", name), &bytes, |b, p| {
            b.iter(|| black_box(load(p).expect("valid")))
        });
    }
    group.finish();
}

criterion_group!(benches, format);
criterion_main!(benches);
