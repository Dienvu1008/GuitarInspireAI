//! ADR-041 L1: with TAB only, the layout is exactly what it was before notation.
//!
//! The fingerprints below were taken from the code at commit 958bf6a (before 6c-2
//! step 2), by running this file there (`print_fingerprints`). They cover every field
//! TAB layouts had then, with floats as raw bits, so any change shows.

use mc_notation::{
    LayoutMetrics, ScoreLayout, ShownTrack, SystemLayout, TabLayout, layout_score,
    layout_score_with, layout_tab,
};
use mc_score::Score;
use mc_score::fixtures::{LARGE, SMALL, TYPICAL, feature_rich, reference, techniques};
use std::fmt::Write;

fn bits(v: f32) -> u32 {
    v.to_bits()
}

fn system(s: &mut String, l: &SystemLayout) {
    let _ = write!(
        s,
        "S{} {} {} {}|",
        l.index,
        bits(l.y),
        bits(l.height),
        bits(l.staff_top)
    );
    for b in &l.bars {
        let _ = write!(
            s,
            "B{} {} {} {}|",
            b.bar,
            bits(b.x),
            bits(b.width),
            b.diagnostic
        );
    }
    for x in &l.lines {
        let _ = write!(
            s,
            "L{:?} {} {} {} {}|",
            x.kind,
            bits(x.x1),
            bits(x.y1),
            bits(x.x2),
            bits(x.y2)
        );
    }
    for g in &l.glyphs {
        let _ = write!(
            s,
            "G{:?} {} {} {}|",
            g.kind,
            bits(g.x),
            bits(g.y),
            bits(g.width)
        );
    }
    for h in &l.hits {
        let _ = write!(
            s,
            "H{} {} {:?} {} {} {} {}|",
            h.bar,
            h.string,
            h.target,
            bits(h.rect.x),
            bits(h.rect.y),
            bits(h.rect.w),
            bits(h.rect.h)
        );
    }
}

fn tab_text(l: &TabLayout) -> String {
    let mut s = format!(
        "T{} {} {} {}|",
        l.track.raw(),
        l.strings,
        bits(l.width),
        bits(l.height)
    );
    for x in &l.systems {
        system(&mut s, x);
    }
    s
}

fn score_text(l: &ScoreLayout) -> String {
    let mut s = format!("{:?} {} {}|", l.tracks, bits(l.width), bits(l.height));
    for x in &l.systems {
        let _ = write!(s, "Y{} {} {}|", x.index, bits(x.y), bits(x.height));
        for st in &x.staves {
            let _ = write!(s, "V{} {} {}|", st.track.raw(), st.strings, bits(st.top));
            system(&mut s, &st.layout);
        }
    }
    s
}

/// FNV-1a 64.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

fn cases() -> Vec<(&'static str, String)> {
    let m = LayoutMetrics::default();
    let narrow = LayoutMetrics {
        width: 500.0,
        ..LayoutMetrics::default()
    };
    let mut out = Vec::new();
    let scores: Vec<(&str, Score)> = vec![
        ("small", reference(SMALL)),
        ("typical", reference(TYPICAL)),
        ("large", reference(LARGE)),
        ("feature_rich", feature_rich()),
        ("techniques", techniques()),
    ];
    for (name, score) in &scores {
        let ids: Vec<_> = score.tracks().map(|t| t.id()).collect();
        // LARGE: track 1 and the first four tracks together (all 20 is slow in debug).
        let first: Vec<_> = ids.iter().copied().take(4).collect();
        for (mname, metrics) in [("default", &m), ("narrow", &narrow)] {
            let tab = layout_tab(score, ids[0], metrics).unwrap();
            out.push((leak(format!("{name}/{mname}/tab")), tab_text(&tab)));
            let all = layout_score(score, &first, metrics).unwrap();
            out.push((leak(format!("{name}/{mname}/score")), score_text(&all)));
        }
    }
    out
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

#[test]
#[ignore = "prints the fingerprints (run on the old code to record them)"]
fn print_fingerprints() {
    for (name, text) in cases() {
        println!("(\"{name}\", 0x{:016x}),", fnv(&text));
    }
}

const BEFORE: &[(&str, u64)] = &[
    // Recorded from 958bf6a (release build; also checked in debug by CI).
    ("small/default/tab", 0xf5b42d4f444c6eca),
    ("small/default/score", 0x8aa508673cc3ab17),
    ("small/narrow/tab", 0x089cc3eefa2bb425),
    ("small/narrow/score", 0xe7e9219b59b5ec36),
    ("typical/default/tab", 0x25f4a5366a6a3474),
    ("typical/default/score", 0xebeb6040c4195890),
    ("typical/narrow/tab", 0x5c401f3003713cfc),
    ("typical/narrow/score", 0x7ba04232258c9c6a),
    ("large/default/tab", 0xd1cf5bb06d4b54d2),
    ("large/default/score", 0x1bf6885c11ac4b13),
    ("large/narrow/tab", 0x2265f9e2570e471b),
    ("large/narrow/score", 0x672089e6bceb74d0),
    ("feature_rich/default/tab", 0x74dd2b7f497f6cc3),
    ("feature_rich/default/score", 0x2de81c33442d64cc),
    ("feature_rich/narrow/tab", 0xa20fb03abaf75675),
    ("feature_rich/narrow/score", 0xa84ab92b2f1b8dfb),
    ("techniques/default/tab", 0x3bd9d70e21aa5cde),
    ("techniques/default/score", 0xa78b9836e8c592e9),
    ("techniques/narrow/tab", 0x42d98eba5782a521),
    ("techniques/narrow/score", 0xe0564241d05c9c78),
];

#[test]
fn l1_tab_layouts_equal_the_layouts_before_notation() {
    let now = cases();
    assert_eq!(now.len(), BEFORE.len(), "case list");
    for ((name, text), (old_name, old)) in now.iter().zip(BEFORE) {
        assert_eq!(name, old_name);
        assert_eq!(fnv(text), *old, "{name}: TAB layout changed");
    }
}

#[test]
fn l1_tab_only_configuration_equals_the_old_api() {
    let m = LayoutMetrics::default();
    for score in [reference(SMALL), feature_rich(), techniques()] {
        let ids: Vec<_> = score.tracks().map(|t| t.id()).collect();
        let shown: Vec<_> = ids.iter().map(|t| ShownTrack::tab(*t)).collect();
        assert_eq!(
            layout_score(&score, &ids, &m).unwrap(),
            layout_score_with(&score, &shown, &m).unwrap()
        );
    }
}
