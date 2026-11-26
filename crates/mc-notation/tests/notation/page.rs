//! Pagination tests (ADR-041 section 11, slice 6c-5a): PG1 to PG4.

use crate::{both, random, samples};
use mc_notation::{
    LayoutMetrics, NotationError, PageSetup, PageSize, Pagination, ScoreSystem, ShownTrack,
    StaffConfiguration, layout_score_systems, outline_score_with, paginate,
};
use mc_score::Score;
use mc_score::fixtures::{Shape, reference};

const EPS: f32 = 0.01;

/// A4, Letter, and short pages that force splits: between tracks, and between a
/// track's notation and TAB.
pub fn setups() -> Vec<(&'static str, PageSetup)> {
    let short = |h: f32| {
        PageSetup::new(PageSize {
            width: PageSize::A4.width,
            height: h,
        })
    };
    vec![
        ("A4", PageSetup::new(PageSize::A4)),
        ("Letter", PageSetup::new(PageSize::LETTER)),
        ("short 420", short(420.0)),
        ("short 330", short(330.0)),
        ("short 280", short(280.0)),
    ]
}

/// The shown staves: notation and TAB everywhere; and every other track TAB only,
/// the others notation only.
pub fn views(score: &Score) -> Vec<(&'static str, Vec<ShownTrack>)> {
    let mixed = score
        .tracks()
        .enumerate()
        .map(|(i, t)| ShownTrack {
            track: t.id(),
            staves: if i % 2 == 0 {
                StaffConfiguration::TAB
            } else {
                StaffConfiguration {
                    notation: true,
                    tab: false,
                }
            },
        })
        .collect();
    vec![("both", both(score)), ("mixed", mixed)]
}

/// Multi-track and multi-page inputs: the S11 inputs and fixtures, TYPICAL and LARGE
/// shapes shortened for a debug test run (the full sizes are in the PG7 bench).
pub fn inputs() -> Vec<(String, Score)> {
    let mut v: Vec<(String, Score)> = samples::all()
        .into_iter()
        .map(|(n, s, _)| (n.to_string(), s))
        .collect();
    v.push((
        "typical-40".into(),
        reference(Shape {
            bars: 40,
            ..mc_score::fixtures::TYPICAL
        }),
    ));
    v.push((
        "two-tracks-60".into(),
        reference(Shape {
            tracks: 2,
            bars: 60,
            ..mc_score::fixtures::TYPICAL
        }),
    ));
    v.push((
        "large-8".into(),
        reference(Shape {
            bars: 8,
            ..mc_score::fixtures::LARGE
        }),
    ));
    v
}

#[derive(Default)]
pub struct Counts {
    pub pages: usize,
    pub systems: usize,
    pub track_splits: usize,
    pub stave_splits: usize,
    /// Page 1 holds only the title (its first stave is taller than page 1's body).
    pub empty_first: bool,
}

/// Every PG1, PG3 and PG4 property of one pagination; returns what it saw.
pub fn check(
    name: &str,
    score: &Score,
    tracks: &[ShownTrack],
    base: &LayoutMetrics,
    setup: &PageSetup,
) -> Counts {
    let p = paginate(score, tracks, base, setup).unwrap_or_else(|e| panic!("{name}: {e}"));
    let mut counts = Counts::default();
    // PG3: the outline is the screen's at the content width; the systems as laid out.
    let metrics = setup.metrics(base);
    assert_eq!(p.metrics, metrics, "{name}");
    assert_eq!(p.metrics.width, setup.content_width(), "{name}");
    assert_eq!(
        p.outline,
        outline_score_with(score, tracks, &metrics).unwrap(),
        "{name}: same line breaking as the screen"
    );
    let systems =
        layout_score_systems(score, &p.outline, 0..p.outline.systems.len(), &metrics).unwrap();
    assert_eq!(systems.len(), p.outline.systems.len());
    counts.systems = systems.len();
    counts.pages = p.page_count();

    // PG5 values and PG4 reservations.
    assert!(p.page_count() >= 1);
    for (i, page) in p.pages.iter().enumerate() {
        assert_eq!(page.number, i + 1, "{name}");
        assert_eq!(page.body, setup.body(page.number), "{name}");
        assert_eq!(page.footer, setup.footer(), "{name}");
        assert!(
            page.footer.y >= page.body.bottom() - EPS,
            "{name}: footer below the body"
        );
        match (i, page.title) {
            (0, Some(t)) => assert!(t.bottom() <= page.body.y + EPS, "{name}: title above"),
            (0, None) => panic!("{name}: no title on page 1"),
            (_, Some(_)) => panic!("{name}: a title after page 1"),
            (_, None) => {}
        }
        assert!(page.body.y >= setup.margin - EPS);
        assert!(page.footer.bottom() <= setup.size.height - setup.margin + EPS);
    }

    // PG1: every stave of every system exactly once, in order; no body overfilled.
    let mut expected = (0usize, 0usize); // (system, next stave)
    for page in &p.pages {
        let mut previous_bottom: Option<f32> = None;
        for item in &page.items {
            assert_eq!(
                (item.system, item.staves.start),
                expected,
                "{name}: page {} out of order",
                page.number
            );
            let sys = &systems[item.system];
            let n = sys.staves.len();
            assert!(item.staves.end > item.staves.start && item.staves.end <= n);
            assert_eq!(item.whole, item.staves == (0..n), "{name}");
            // PG3: heights are the laid-out staves'.
            assert_close(item.height, part_height(sys, item.staves.clone()), name);
            if item.whole {
                assert_close(item.height, sys.height, name);
                assert_close(item.height, p.outline.systems[item.system].height, name);
            }
            // Inside the body, stacked with the system gap.
            assert!(item.y >= page.body.y - EPS, "{name}: above the body");
            assert!(
                item.y + item.height <= page.body.bottom() + EPS,
                "{name}: page {} overfilled",
                page.number
            );
            match previous_bottom {
                Some(b) => assert_close(item.y, b + metrics.system_gap, name),
                None => assert_close(item.y, page.body.y, name),
            }
            previous_bottom = Some(item.y + item.height);
            expected = if item.staves.end == n {
                (item.system + 1, 0)
            } else {
                (item.system, item.staves.end)
            };
        }
    }
    assert_eq!(expected, (systems.len(), 0), "{name}: every system placed");

    // PG1 breaks: a split only for a system taller than its page body; a track's
    // staves apart only for a track taller than a body.
    let starts = first_pages(&p);
    for (s, sys) in systems.iter().enumerate() {
        let parts: Vec<_> = items_of(&p, s);
        if parts.len() == 1 {
            continue;
        }
        let (page, earlier) = starts[s];
        let body = setup.body(page).h;
        assert!(
            sys.height > body + EPS || (earlier && sys.height > setup.body(page + 1).h + EPS),
            "{name}: system {s} ({:.1}) split though it fits a body ({body:.1})",
            sys.height
        );
        for (page_no, item) in &parts[1..] {
            let first = &sys.staves[item.staves.start];
            let before = &sys.staves[item.staves.start - 1];
            if first.track == before.track {
                // The track starts on an empty page; split only if taller than its body.
                let track = track_range(sys, item.staves.start);
                let (start_page, start_item) = parts
                    .iter()
                    .find(|(_, i)| i.staves.contains(&track.start))
                    .unwrap();
                assert_eq!(
                    start_item.staves.start, track.start,
                    "{name}: track at a page top"
                );
                assert!(*page_no > *start_page);
                let h = part_height(sys, track);
                assert!(
                    h > setup.body(*start_page).h + EPS,
                    "{name}: system {s} track split though the track ({h:.1}) fits a body"
                );
                counts.stave_splits += 1;
            } else {
                counts.track_splits += 1;
            }
        }
    }

    // Page 1 alone may be empty: when the first stave is taller than its body only.
    if p.pages[0].items.is_empty() {
        let first = &systems[0].staves[0];
        assert!(
            first.layout.height > setup.body(1).h,
            "{name}: page 1 empty"
        );
        counts.empty_first = true;
    }

    // PG2 greedy: the first item of each page could not have gone on the page before.
    for w in p.pages.windows(2) {
        let (prev, next) = (&w[0], &w[1]);
        let Some(item) = next.items.first() else {
            panic!("{name}: an empty page {}", next.number)
        };
        let sys = &systems[item.system];
        let unit = if item.whole {
            item.staves.clone()
        } else {
            let group = track_range(sys, item.staves.start);
            if group.start == item.staves.start {
                group
            } else {
                item.staves.start..item.staves.start + 1
            }
        };
        let joins = prev
            .items
            .last()
            .is_some_and(|l| l.system == item.system && l.staves.end == item.staves.start);
        let top = match prev.items.last() {
            Some(l) if joins => l.y + l.height,
            Some(l) => l.y + l.height + metrics.system_gap,
            None => prev.body.y,
        };
        let h = part_height(sys, unit);
        assert!(
            top + h > prev.body.bottom() - EPS,
            "{name}: page {} had room for the next item",
            prev.number
        );
    }
    counts
}

fn assert_close(a: f32, b: f32, name: &str) {
    assert!((a - b).abs() <= EPS, "{name}: {a} != {b}");
}

/// Top of the first stave to the bottom of the last, as laid out.
fn part_height(sys: &ScoreSystem, staves: std::ops::Range<usize>) -> f32 {
    let last = &sys.staves[staves.end - 1];
    last.top + last.layout.height - sys.staves[staves.start].top
}

/// The staves of the track of stave `i`.
fn track_range(sys: &ScoreSystem, i: usize) -> std::ops::Range<usize> {
    let track = sys.staves[i].track;
    let start = (0..=i)
        .rev()
        .take_while(|&k| sys.staves[k].track == track)
        .last()
        .unwrap();
    let end = (i..sys.staves.len())
        .take_while(|&k| sys.staves[k].track == track)
        .last()
        .unwrap()
        + 1;
    start..end
}

/// Every item of system `s` with its page number.
fn items_of(p: &Pagination, s: usize) -> Vec<(usize, &mc_notation::PageItem)> {
    p.pages
        .iter()
        .flat_map(|pg| pg.items.iter().map(move |i| (pg.number, i)))
        .filter(|(_, i)| i.system == s)
        .collect()
}

/// For each system: the page its first item is on, and whether that page holds
/// earlier items.
fn first_pages(p: &Pagination) -> Vec<(usize, bool)> {
    let mut out = Vec::new();
    for page in &p.pages {
        for (k, item) in page.items.iter().enumerate() {
            if item.staves.start == 0 {
                out.push((page.number, k > 0));
            }
        }
    }
    out
}

#[test]
fn pg1_pg3_pg4_inputs_fixtures_and_shapes_at_every_page_size() {
    let base = LayoutMetrics::default();
    let mut total = Counts::default();
    let mut multi_page = 0;
    for (name, score) in inputs() {
        for (view, tracks) in views(&score) {
            for (size, setup) in setups() {
                let label = format!("{name} {view} {size}");
                let c = check(&label, &score, &tracks, &base, &setup);
                if c.pages > 1 {
                    multi_page += 1;
                }
                total.systems += c.systems;
                total.track_splits += c.track_splits;
                total.stave_splits += c.stave_splits;
            }
        }
    }
    assert!(
        multi_page > 20 && total.track_splits > 20 && total.stave_splits > 5,
        "{multi_page} multi-page, {} track splits, {} stave splits",
        total.track_splits,
        total.stave_splits
    );
}

#[test]
fn pg1_random_scores_at_every_page_size() {
    let base = LayoutMetrics::default();
    let mut multi_page = 0;
    let mut stave_splits = 0;
    let mut empty_first = 0;
    for seed in 0..300 {
        let score = random::random_score(seed);
        let tracks = both(&score);
        for (size, setup) in setups() {
            let c = check(
                &format!("seed {seed} {size}"),
                &score,
                &tracks,
                &base,
                &setup,
            );
            multi_page += usize::from(c.pages > 1);
            stave_splits += c.stave_splits;
            empty_first += usize::from(c.empty_first);
        }
    }
    assert!(
        multi_page > 100 && stave_splits > 50 && empty_first > 0,
        "{multi_page} {stave_splits} {empty_first}"
    );
}

/// A stave taller than every page body is an error, not an overfilled page.
#[test]
fn pg1_a_stave_taller_than_the_body_is_an_error() {
    let score = samples::scale();
    let setup = PageSetup::new(PageSize {
        width: PageSize::A4.width,
        height: 150.0,
    });
    let r = paginate(&score, &both(&score), &LayoutMetrics::default(), &setup);
    assert!(
        matches!(
            r,
            Err(NotationError::PageTooSmall {
                system: 0,
                stave: 0
            })
        ),
        "{r:?}"
    );
}

/// PG4: page 1 loses the title's height; the other pages do not; the footer is on
/// every page. With no title or footer the bodies grow by exactly those heights.
#[test]
fn pg4_title_and_footer_reserve_their_space() {
    let a4 = PageSetup::default();
    let bare = PageSetup {
        title_height: 0.0,
        footer_height: 0.0,
        ..a4
    };
    assert_close(
        bare.body(1).h - a4.body(1).h,
        a4.title_height + a4.footer_height,
        "p1",
    );
    assert_close(bare.body(2).h - a4.body(2).h, a4.footer_height, "p2");
    // A tall title: page 1 holds fewer systems than page 2, and none above the title.
    let score = reference(Shape {
        tracks: 1,
        bars: 60,
        ..mc_score::fixtures::TYPICAL
    });
    let tall = PageSetup {
        title_height: 300.0,
        ..a4
    };
    let tab = vec![ShownTrack::tab(score.track_at(0).unwrap().id())];
    for setup in [a4, bare, tall] {
        check("pg4", &score, &tab, &LayoutMetrics::default(), &setup);
    }
    let p = paginate(&score, &tab, &LayoutMetrics::default(), &tall).unwrap();
    assert!(p.page_count() >= 3, "{}", p.page_count());
    assert!(p.pages[0].items.len() < p.pages[1].items.len());
    assert!(p.pages[0].items[0].y >= 36.0 + 300.0 - EPS);
    // With no footer, a later page ends at the bottom margin.
    assert_close(bare.body(2).bottom(), PageSize::A4.height - 36.0, "bare");
}

/// PG2: an edit to bars of systems starting on page k leaves the pages before k as
/// they were.
#[test]
fn pg2_an_edit_on_page_k_keeps_the_pages_before() {
    use mc_score::BeatRef;
    use mc_theory::fretted::{Fret, FretPosition, StringIndex};
    let base = LayoutMetrics::default();
    let mut changed = 0;
    let mut checked = 0;
    for tracks in [1, 2, 4] {
        // Quarter notes, so systems hold several bars.
        let score = reference(Shape {
            tracks,
            bars: 60,
            beats: 4,
            notes_per_beat: 1,
        });
        for (_, setup) in setups().into_iter().take(3) {
            let shown = both(&score);
            let before = paginate(&score, &shown, &base, &setup).unwrap();
            for page in &before.pages[1..] {
                // A system starting on this page with two or more bars: edit its last
                // bar, so the break of the system before is not involved.
                let Some(item) = page.items.iter().find(|i| i.staves.start == 0) else {
                    continue;
                };
                let sys = before.outline.systems[item.system];
                if sys.end_bar - sys.first_bar < 2 {
                    continue;
                }
                let bar = sys.end_bar - 1;
                let mut edited = score.clone();
                let track = edited.track_at(0).unwrap().id();
                let beat = edited.track(track).unwrap().bar(bar).unwrap().voices[0].beats[0].id;
                edited
                    .set_note(
                        BeatRef {
                            track,
                            bar,
                            voice: 0,
                            beat,
                        },
                        FretPosition {
                            string: StringIndex::new(1).unwrap(),
                            fret: Fret(22),
                        },
                    )
                    .unwrap();
                let after = paginate(&edited, &shown, &base, &setup).unwrap();
                let k = page.number - 1;
                assert_eq!(
                    before.pages[..k],
                    after.pages[..k],
                    "pages before {}",
                    page.number
                );
                assert_eq!(
                    before.outline.systems[..item.system],
                    after.outline.systems[..item.system]
                );
                checked += 1;
                if before != after {
                    changed += 1;
                }
            }
        }
    }
    assert!(
        checked > 20 && changed > 10,
        "{checked} checked, {changed} changed"
    );
}

/// PG9 report: page counts of the full TYPICAL and LARGE shapes at A4, at the
/// default staff size and at 0.6 of it (a print-sized TAB string gap of 7.2 pt).
#[test]
#[ignore = "prints the PG9 page counts of the full shapes (slow in debug)"]
fn print_shape_page_counts() {
    use mc_score::fixtures::{LARGE, TYPICAL};
    for k in [1.0, 0.6] {
        let base = crate::svg::scaled(k);
        for (name, shape, shown) in [
            ("typical", TYPICAL, 4),
            ("large", LARGE, 4),
            ("large", LARGE, 20),
        ] {
            let score = reference(shape);
            let tracks: Vec<ShownTrack> = both(&score).into_iter().take(shown).collect();
            for (size, setup) in setups().into_iter().take(2) {
                let p = paginate(&score, &tracks, &base, &setup).unwrap();
                let split = (0..p.outline.systems.len())
                    .filter(|&s| items_of(&p, s).len() > 1)
                    .count();
                let bars: f32 = p
                    .outline
                    .systems
                    .iter()
                    .map(|s| (s.end_bar - s.first_bar) as f32)
                    .sum::<f32>()
                    / p.outline.systems.len() as f32;
                println!(
                    "scale {k} {name}/{shown} {size}: {} systems ({bars:.2} bars each), {} pages, {split} split, system height {:.0}",
                    p.outline.systems.len(),
                    p.page_count(),
                    p.outline.systems[0].height
                );
            }
        }
    }
}

/// PG9 report: page counts and splits of every input, view and page size.
#[test]
#[ignore = "prints the PG9 pagination report"]
fn print_pagination_report() {
    let base = LayoutMetrics::default();
    for (name, score) in inputs() {
        for (view, tracks) in views(&score) {
            for (size, setup) in setups() {
                let label = format!("{name} {view} {size}");
                match paginate(&score, &tracks, &base, &setup) {
                    Ok(p) => {
                        let heights: Vec<String> = p
                            .outline
                            .systems
                            .iter()
                            .take(3)
                            .map(|s| format!("{:.1}", s.height))
                            .collect();
                        let split = (0..p.outline.systems.len())
                            .filter(|&s| items_of(&p, s).len() > 1)
                            .count();
                        println!(
                            "{label}: {} systems, {} pages, {split} split systems, items/page {:?}, first heights {heights:?}, body1 {:.1} body2 {:.1}",
                            p.outline.systems.len(),
                            p.page_count(),
                            p.pages.iter().map(|pg| pg.items.len()).collect::<Vec<_>>(),
                            setup.body(1).h,
                            setup.body(2).h
                        );
                    }
                    Err(e) => println!("{label}: {e}"),
                }
            }
        }
    }
}

/// PG9 page previews: split the output with the BEGIN/END markers.
#[test]
#[ignore = "prints the PG9 page preview SVGs"]
fn print_page_previews() {
    let base = LayoutMetrics::default();
    let all = inputs();
    let find = |n: &str| all.iter().find(|(name, _)| name == n).unwrap().1.clone();
    let cases: Vec<(&str, Score, Vec<ShownTrack>, PageSetup, usize)> = {
        let fr = find("feature_rich");
        let large = find("large-8");
        let keys = find("keys");
        let mixed = views(&large).remove(1).1;
        vec![
            (
                "keys-a4",
                keys.clone(),
                both(&keys),
                PageSetup::default(),
                1,
            ),
            (
                "feature_rich-a4",
                fr.clone(),
                both(&fr),
                PageSetup::default(),
                2,
            ),
            (
                "feature_rich-letter",
                fr.clone(),
                both(&fr),
                PageSetup::new(PageSize::LETTER),
                2,
            ),
            ("large8-mixed-a4", large, mixed, PageSetup::default(), 5),
        ]
    };
    for (name, score, tracks, setup, count) in cases {
        let p = paginate(&score, &tracks, &base, &setup).unwrap();
        let systems =
            layout_score_systems(&score, &p.outline, 0..p.outline.systems.len(), &p.metrics)
                .unwrap();
        for page in 0..count.min(p.page_count()) {
            println!(
                "----- BEGIN {name}-p{}.svg -----\n{}----- END {name}-p{}.svg -----",
                page + 1,
                crate::svg::render_page(&p, &systems, page, name),
                page + 1
            );
        }
    }
}
