//! N5 (timing and payload), N6 (PDF routes) and N8 (determinism) evidence.

use crate::layout::Engine;
use crate::model::*;
use crate::pitch::Transposition;
use crate::smufl::Font;
use crate::{pdf, svg};
use mc_score::Score;
use std::fmt::Write;
use std::time::Instant;

pub struct Stats {
    pub median: f64,
    pub p95: f64,
    pub p99: f64,
    pub runs: usize,
}

/// Milliseconds per run of `f`, over `runs` runs after 5 warm-up runs.
pub fn time<T>(runs: usize, mut f: impl FnMut() -> T) -> Stats {
    for _ in 0..5 {
        std::hint::black_box(f());
    }
    let mut ms: Vec<f64> = (0..runs)
        .map(|_| {
            let t = Instant::now();
            std::hint::black_box(f());
            t.elapsed().as_secs_f64() * 1e3
        })
        .collect();
    ms.sort_by(f64::total_cmp);
    let q = |p: f64| ms[((ms.len() as f64 * p).ceil() as usize).clamp(1, ms.len()) - 1];
    Stats {
        median: q(0.5),
        p95: q(0.95),
        p99: q(0.99),
        runs,
    }
}

fn row(out: &mut String, what: &str, s: &Stats) {
    let _ = writeln!(
        out,
        "| {what} | {:.3} | {:.3} | {:.3} | {} |",
        s.median, s.p95, s.p99, s.runs
    );
}

pub fn config(staff_space: f32, notation: bool, transposition: Transposition) -> Config {
    Config {
        staff_space,
        width: 1000.0,
        staves: StaffConfiguration {
            notation,
            tab: true,
            transposition,
        },
    }
}

/// Payload estimate of one system as today's DTO packs it (ADR-031): a line is 4 f32
/// and a kind byte; a glyph a kind tag and 3 f32; a text 12 bytes and its characters;
/// a beam 5 f32; a tie 3 f32 and a flag.
fn payload(sys: &System) -> (usize, usize, usize) {
    let (mut lines, mut glyphs, mut bytes) = (0, 0, 0);
    for st in &sys.staves {
        lines += st.lines.len();
        glyphs += st.glyphs.len() + st.texts.len();
        bytes += st.lines.len() * 17
            + st.glyphs.len() * 16
            + st.texts.iter().map(|t| 12 + t.text.len()).sum::<usize>()
            + st.beams.len() * 20
            + st.ties.len() * 13;
    }
    (lines, glyphs, bytes)
}

pub fn timing(font: &Font, typical: &Score, large: &Score, feature_rich: &Score) -> String {
    let mut out = String::new();
    let runs = 200;
    let both = config(8.0, true, Transposition::GUITAR);
    let tab_only = config(8.0, false, Transposition::GUITAR);
    let _ = writeln!(out, "### N5 (a) layout computation, ms (release)\n");
    let _ = writeln!(out, "| Case | Median | p95 | p99 | Runs |\n|---|---|---|---|---|");
    for (name, score) in [("TYPICAL", typical), ("LARGE", large)] {
        let e = Engine::new(score, font, &both);
        row(&mut out, &format!("{name}, notation + TAB, one system"), &time(runs, || e.layout(Some(0..1))));
        row(&mut out, &format!("{name}, notation + TAB, in view (4 systems)"), &time(runs, || e.layout(Some(0..4))));
        row(&mut out, &format!("{name}, notation + TAB, whole score"), &time(runs, || e.layout(None)));
        // With cached bar plans: an edit rebuilds two plans, then lays out the view.
        let cached = e.plans();
        row(&mut out, &format!("{name}, notation + TAB, in view from cached plans"), &time(runs, || e.layout_with(&cached, Some(0..4))));
        let mut edited = cached.clone();
        let mid = score.master_bar_count() / 2;
        row(&mut out, &format!("{name}, notation + TAB, edit: replan 2 bars + in view from cache"), &time(runs, || {
            e.replan(&mut edited, mid);
            e.layout_with(&edited, Some(0..4))
        }));
        row(&mut out, &format!("{name}, building all plans (cache fill, once per open)"), &time(20, || e.plans()));
        let t = Engine::new(score, font, &tab_only);
        row(&mut out, &format!("{name}, spike TAB only, in view"), &time(runs, || t.layout(Some(0..4))));
        // Today's engine, the same score, its default metrics (ADR-031 fallback 2).
        let ids: Vec<_> = score.tracks().map(|t| t.id()).collect();
        let m = mc_notation::LayoutMetrics::default();
        row(&mut out, &format!("{name}, today's mc-notation TAB, in view (outline + 4 systems)"), &time(runs, || {
            let o = mc_notation::outline_score(score, &ids, &m).unwrap();
            mc_notation::layout_score_systems(score, &o, 0..4, &m).unwrap()
        }));
    }
    let _ = writeln!(out, "\n### N5 (b) rendering (layout model to SVG), ms\n");
    let _ = writeln!(out, "| Case | Median | p95 | p99 | Runs |\n|---|---|---|---|---|");
    let e = Engine::new(typical, font, &both);
    let in_view = e.layout(Some(0..4));
    let whole = e.layout(None);
    row(&mut out, "TYPICAL in view (4 systems)", &time(runs, || svg::render(&in_view, None)));
    row(&mut out, "TYPICAL whole score", &time(runs, || svg::render(&whole, None)));
    let _ = writeln!(out, "\n### N5 (c) PDF generation, ms\n");
    let _ = writeln!(out, "| Case | Median | p95 | p99 | Runs |\n|---|---|---|---|---|");
    let fr = Engine::new(feature_rich, font, &both).layout(None);
    let fr_svg = svg::render(&fr, None);
    let ty_svg = svg::render(&whole, None);
    row(&mut out, "feature_rich, vector", &time(runs, || pdf::vector(&fr, font)));
    row(&mut out, "feature_rich, SVG-to-PDF (SVG given)", &time(runs, || pdf::from_svg(&fr_svg, font, true)));
    row(&mut out, "TYPICAL whole, vector", &time(50, || pdf::vector(&whole, font)));
    row(&mut out, "TYPICAL whole, SVG-to-PDF (SVG given)", &time(50, || pdf::from_svg(&ty_svg, font, true)));
    // Part of the SVG-to-PDF time that is set-up, not conversion: the system font scan.
    row(&mut out, "of which: system font scan (usvg fontdb)", &time(20, || {
        let mut db = svg2pdf::usvg::fontdb::Database::new();
        db.load_system_fonts();
        db.len()
    }));
    // Payload per system.
    let _ = writeln!(out, "\n### N5 payload per system (TYPICAL, system 0)\n");
    let (l, g, b) = payload(&in_view.systems[0]);
    let t = Engine::new(typical, font, &tab_only).layout(Some(0..1));
    let (tl, tg, tb) = payload(&t.systems[0]);
    let ids: Vec<_> = typical.tracks().map(|t| t.id()).collect();
    let m = mc_notation::LayoutMetrics::default();
    let o = mc_notation::outline_score(typical, &ids, &m).unwrap();
    let today = mc_notation::layout_score_systems(typical, &o, 0..1, &m).unwrap();
    let (mut dl, mut dg, mut dh) = (0, 0, 0);
    for st in &today[0].staves {
        dl += st.layout.lines.len();
        dg += st.layout.glyphs.len();
        dh += st.layout.hits.len();
    }
    let _ = writeln!(out, "| Layout | Bars | Lines | Glyphs and texts | Bytes (estimate) |\n|---|---|---|---|---|");
    let _ = writeln!(out, "| Spike, notation + TAB | {} | {l} | {g} | {b} |", in_view.systems[0].end_bar - in_view.systems[0].first_bar);
    let _ = writeln!(out, "| Spike, TAB only | {} | {tl} | {tg} | {tb} |", t.systems[0].end_bar - t.systems[0].first_bar);
    let _ = writeln!(
        out,
        "| Today's mc-notation TAB (default metrics) | {} | {dl} | {dg} | {} (+ {dh} hit regions, 32 bytes each) |",
        o.systems[0].end_bar - o.systems[0].first_bar,
        dl * 17 + dg * 16
    );
    out
}

/// N6 comparison and the PDF files.
pub fn pdfs(font: &Font, scores: &[(&str, &Score)], dir: &str) -> String {
    let mut out = String::new();
    let both = config(8.0, true, Transposition::GUITAR);
    let _ = writeln!(out, "| Input | Route | Size (bytes) | Glyph positions read back | Max error (pt) |\n|---|---|---|---|---|");
    for (name, score) in scores {
        let l = Engine::new(score, font, &both).layout(None);
        let expected = expected_glyphs(&l);
        let v = pdf::vector(&l, font);
        pdf::set_compress(false);
        let v_plain = pdf::vector(&l, font);
        pdf::set_compress(true);
        let s_svg = svg::render(&l, None);
        let s = pdf::from_svg(&s_svg, font, true);
        let s_plain = pdf::from_svg(&s_svg, font, false);
        std::fs::write(format!("{dir}/{name}.vector.pdf"), &v).unwrap();
        std::fs::write(format!("{dir}/{name}.svg2pdf.pdf"), &s).unwrap();
        for (route, bytes, readable) in [("vector", &v, &v_plain), ("SVG-to-PDF", &s, &s_plain)] {
            let found = text_positions(readable);
            let (matched, err) = compare(&expected, &found);
            let _ = writeln!(out, "| {name} | {route} | {} | {matched} of {} (found {}) | {err:.4} |", bytes.len(), expected.len(), found.len());
        }
    }
    out
}

/// Glyph origins of the layout in PDF points (y up), sorted.
fn expected_glyphs(l: &Layout) -> Vec<(f32, f32)> {
    let h = l.height * pdf::K;
    let mut v = Vec::new();
    for sys in &l.systems {
        for st in &sys.staves {
            for g in &st.glyphs {
                v.push((g.x * pdf::K, h - (sys.y + st.position.top + g.y) * pdf::K));
            }
        }
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

/// Text origins in an uncompressed PDF content stream: the e and f of every `Tm`, under
/// the current `cm` translation (enough for both routes' simple streams).
fn text_positions(pdf: &[u8]) -> Vec<(f32, f32)> {
    let text = String::from_utf8_lossy(pdf);
    let mut out = Vec::new();
    let mut stack: Vec<[f32; 6]> = vec![[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]];
    let mut nums: Vec<f32> = Vec::new();
    for tok in text.split_ascii_whitespace() {
        if let Ok(n) = tok.parse::<f32>() {
            nums.push(n);
            continue;
        }
        let ctm = *stack.last().unwrap();
        match tok {
            "q" => stack.push(ctm),
            "Q" => {
                if stack.len() > 1 {
                    stack.pop();
                }
            }
            "cm" if nums.len() >= 6 => {
                let m: Vec<f32> = nums[nums.len() - 6..].to_vec();
                let c = ctm;
                let top = stack.last_mut().unwrap();
                *top = [
                    m[0] * c[0] + m[1] * c[2],
                    m[0] * c[1] + m[1] * c[3],
                    m[2] * c[0] + m[3] * c[2],
                    m[2] * c[1] + m[3] * c[3],
                    m[4] * c[0] + m[5] * c[2] + c[4],
                    m[4] * c[1] + m[5] * c[3] + c[5],
                ];
            }
            "Tm" if nums.len() >= 6 => {
                let (e, f) = (nums[nums.len() - 2], nums[nums.len() - 1]);
                out.push((ctm[0] * e + ctm[2] * f + ctm[4], ctm[1] * e + ctm[3] * f + ctm[5]));
            }
            _ => {}
        }
        nums.clear();
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    out
}

/// Every expected origin matched to the nearest found one within 0.5 pt.
fn compare(expected: &[(f32, f32)], found: &[(f32, f32)]) -> (usize, f32) {
    let mut matched = 0;
    let mut worst = 0.0f32;
    for e in expected {
        let best = found
            .iter()
            .map(|f| ((f.0 - e.0).powi(2) + (f.1 - e.1).powi(2)).sqrt())
            .fold(f32::MAX, f32::min);
        if best <= 0.5 {
            matched += 1;
            worst = worst.max(best);
        }
    }
    (matched, worst)
}

/// N8: canonical layout, SVG and both PDFs repeated in one process; returns the
/// artefacts of one run for the cross-process and debug/release comparison.
pub fn determinism(font: &Font, scores: &[(&str, &Score)]) -> (String, Vec<(String, Vec<u8>)>) {
    let mut out = String::new();
    let _ = writeln!(out, "| Input | Layout (100 runs) | SVG (10) | Vector PDF (10) | SVG-to-PDF (10) | Canonical chars | Staff size 8.5 | Concert pitch |\n|---|---|---|---|---|---|---|---|");
    let mut files = Vec::new();
    let cfg = config(8.0, true, Transposition::GUITAR);
    for (name, score) in scores {
        let e = Engine::new(score, font, &cfg);
        let first = e.layout(None);
        let canon = first.canonical();
        let svg0 = svg::render(&first, None);
        let v0 = pdf::vector(&first, font);
        let s0 = pdf::from_svg(&svg0, font, true);
        let (mut l_same, mut s_same, mut v_same, mut p_same) = (true, true, true, true);
        for _ in 0..100 {
            let l = e.layout(None);
            l_same &= l.canonical() == canon;
        }
        for _ in 0..10 {
            s_same &= svg::render(&first, None) == svg0;
            v_same &= pdf::vector(&first, font) == v0;
            p_same &= pdf::from_svg(&svg0, font, true) == s0;
        }
        let same = l_same && s_same && v_same && p_same;
        if !same {
            let _ = writeln!(out, "<!-- {name}: layout {l_same}, svg {s_same}, vector {v_same}, svg2pdf {p_same} -->");
            if !p_same && *name == "rests" {
                // Where two uncompressed conversions differ.
                let a = pdf::from_svg(&svg0, font, false);
                let b = pdf::from_svg(&svg0, font, false);
                let lines_a: Vec<&[u8]> = a.split(|c| *c == b'\n').collect();
                let lines_b: Vec<&[u8]> = b.split(|c| *c == b'\n').collect();
                let mut shown = 0;
                for (x, y) in lines_a.iter().zip(&lines_b) {
                    if x != y && shown < 6 {
                        let _ = writeln!(out, "<!-- a: {} | b: {} -->", String::from_utf8_lossy(&x[..x.len().min(120)]), String::from_utf8_lossy(&y[..y.len().min(120)]));
                        shown += 1;
                    }
                }
                let _ = writeln!(out, "<!-- sizes {} vs {}, lines {} vs {} -->", a.len(), b.len(), lines_a.len(), lines_b.len());
            }
        }
        // The hash covers the configuration and the transposition.
        let bigger = Engine::new(score, font, &config(8.5, true, Transposition::GUITAR)).layout(None).canonical();
        let concert = Engine::new(score, font, &config(8.0, true, Transposition { octaves: 0 })).layout(None).canonical();
        let yes = |b: bool| if b { "identical" } else { "DIFFERENT" };
        let _ = writeln!(
            out,
            "| {name} | {} | {} | {} | {} | {} | {} | {} |",
            yes(l_same),
            yes(s_same),
            yes(v_same),
            yes(p_same),
            canon.len(),
            if bigger != canon { "changes" } else { "SAME (not covered)" },
            if concert != canon { "changes" } else { "SAME (not covered)" }
        );
        files.push((format!("{name}.layout.txt"), canon.into_bytes()));
        files.push((format!("{name}.svg"), svg0.into_bytes()));
        files.push((format!("{name}.vector.pdf"), v0));
        files.push((format!("{name}.svg2pdf.pdf"), s0));
    }
    (out, files)
}
