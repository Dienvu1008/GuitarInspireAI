//! ADR-041 section 12: PD1 to PD5 on exported PDFs, read back.

#[path = "../../mc-notation/tests/notation/random.rs"]
mod random;
#[path = "../../mc-notation/tests/notation/samples.rs"]
#[allow(dead_code)]
mod samples;
mod support {
    pub mod read;
}

use mc_notation::{
    GlyphKind, LineKind, PageSetup, PageSize, Pagination, ScoreSystem, ShownTrack,
    StaffConfiguration, StaveKind, SystemSpacing, layout_score_systems, paginate,
};
use mc_pdf::{PdfError, export_pdf};
use mc_score::Score;
use skrifa::prelude::*;
use support::read::{Content, Doc};

const EPS: f32 = 0.01;
const NOTO: &[u8] = include_bytes!("../../../packages/ui_kit/fonts/NotoSansMono-Variable.ttf");
const BRAVURA: &[u8] = include_bytes!("../../../packages/ui_kit/fonts/bravura/Bravura.otf");

fn both(score: &Score) -> Vec<ShownTrack> {
    score
        .tracks()
        .map(|t| ShownTrack {
            track: t.id(),
            staves: StaffConfiguration::NOTATION_AND_TAB,
        })
        .collect()
}

fn setups() -> Vec<(&'static str, PageSetup)> {
    vec![
        ("A4", PageSetup::new(PageSize::A4)),
        (
            "Letter compact",
            PageSetup {
                system_spacing: SystemSpacing::Compact,
                ..PageSetup::new(PageSize::LETTER)
            },
        ),
    ]
}

fn inputs() -> Vec<(String, Score)> {
    let mut v: Vec<(String, Score)> = samples::all()
        .into_iter()
        .map(|(n, s, _)| (n.to_string(), s))
        .collect();
    v.push((
        "typical-12".into(),
        mc_score::fixtures::reference(mc_score::fixtures::Shape {
            bars: 12,
            ..mc_score::fixtures::TYPICAL
        }),
    ));
    v.extend((0..50).map(|s| (format!("seed {s}"), random::random_score(s))));
    v
}

/// The Noto Sans Mono quantities the painting rules use (section 12.1), read here
/// from the font rather than from `mc-pdf`.
struct TextFont {
    digit_em: f32,
    ascent: f32,
    descent: f32,
}

fn text_font() -> TextFont {
    let f = FontRef::new(NOTO).unwrap();
    let m = f.metrics(Size::unscaled(), LocationRef::default());
    let zero = f.charmap().map('0').unwrap();
    let adv = f
        .glyph_metrics(Size::unscaled(), LocationRef::default())
        .advance_width(zero)
        .unwrap();
    let upem = f32::from(m.units_per_em);
    TextFont {
        digit_em: adv / upem,
        ascent: m.ascent,
        descent: m.descent,
    }
}

/// What a page must contain, from the pagination and the laid-out systems only.
#[derive(Default)]
struct Expected {
    /// (em, x, baseline y, character) of every notation glyph.
    glyphs: Vec<(f32, f32, f32, char)>,
    /// (x1, y1, x2, y2, width, grey) of every notation and TAB line.
    lines: Vec<[f32; 6]>,
    /// First two points of every beam band.
    beams: Vec<[f32; 4]>,
    /// (text, centre x, baseline y, size) of every fret number.
    frets: Vec<(String, f32, f32, f32)>,
}

fn expected(p: &Pagination, systems: &[ScoreSystem], page: usize, tf: &TextFont) -> Expected {
    let k = p.metrics.string_spacing / 12.0;
    let size = p.metrics.digit_width / tf.digit_em;
    let mut e = Expected::default();
    let pg = &p.pages[page];
    for item in &pg.items {
        let sys = &systems[item.system];
        let offset = item.y - sys.staves[item.staves.start].top;
        for st in &sys.staves[item.staves.clone()] {
            let (dx, dy) = (pg.body.x, offset + st.top);
            let lay = &st.layout;
            match st.kind {
                StaveKind::Notation => {
                    let n = &lay.notation;
                    for g in &n.glyphs {
                        let ch = char::from_u32(g.glyph.codepoint()).unwrap();
                        e.glyphs.push((4.0 * n.staff_space, dx + g.x, dy + g.y, ch));
                    }
                    for l in &n.lines {
                        let gray = if l.kind == mc_notation::EngravedLineKind::StaffLine {
                            0.55
                        } else {
                            0.0
                        };
                        e.lines.push([
                            dx + l.x1,
                            dy + l.y1,
                            dx + l.x2,
                            dy + l.y2,
                            l.thickness,
                            gray,
                        ]);
                    }
                    for b in &n.beams {
                        let h = b.thickness / 2.0;
                        e.beams
                            .push([dx + b.x1, dy + b.y1 - h, dx + b.x2, dy + b.y2 - h]);
                    }
                }
                StaveKind::Tab => {
                    for l in &lay.lines {
                        let (gray, w) = match l.kind {
                            LineKind::String => (0.55, 1.0),
                            LineKind::ThickBarLine => (0.0, 3.0),
                            LineKind::Beam => (0.0, 2.5),
                            _ => (0.0, 1.0),
                        };
                        e.lines
                            .push([dx + l.x1, dy + l.y1, dx + l.x2, dy + l.y2, w * k, gray]);
                    }
                    for g in &lay.glyphs {
                        if let GlyphKind::Fret {
                            fret, tied, dead, ..
                        } = g.kind
                        {
                            let text = if dead {
                                "x".to_string()
                            } else if tied {
                                format!("({fret})")
                            } else {
                                fret.to_string()
                            };
                            let base =
                                dy + g.y - size / 2.0 + size * tf.ascent / (tf.ascent - tf.descent);
                            e.frets.push((text, dx + g.x, base, size));
                        }
                    }
                }
            }
        }
    }
    e
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= EPS
}

/// Decodes a text run through the font's ToUnicode map.
fn decode(
    doc: &Doc,
    fonts: &std::collections::BTreeMap<String, u32>,
    t: &support::read::Text,
) -> String {
    let map = doc.to_unicode(fonts[&t.font]);
    t.cids.iter().map(|c| map[c]).collect()
}

/// Checks one export against its pagination; returns the items matched.
fn check(name: &str, score: &Score, shown: &[ShownTrack], setup: &PageSetup) -> usize {
    let title = "Check";
    let out = export_pdf(score, shown, setup, title).unwrap_or_else(|e| panic!("{name}: {e}"));
    let p = paginate(score, shown, setup).unwrap();
    let systems =
        layout_score_systems(score, &p.outline, 0..p.outline.systems.len(), &p.metrics).unwrap();
    let doc = Doc::parse(&out.bytes);
    let pages = doc.pages();
    let tf = text_font();
    // PD5: pages and their size.
    assert_eq!(pages.len(), p.page_count(), "{name}: page count");
    assert_eq!(out.pages, p.page_count(), "{name}");
    let mut matched = 0;
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(
            doc.media_box(*page),
            vec![0.0, 0.0, setup.size.width, setup.size.height],
            "{name}"
        );
        let fonts = doc.page_fonts(*page);
        let c: Content = doc.content(*page);
        assert_eq!(
            c.cm,
            vec![1.0, 0.0, 0.0, -1.0, 0.0, setup.size.height],
            "{name}"
        );
        let e = expected(&p, &systems, i, &tf);
        // PD1: notation glyphs, one to one.
        let music: Vec<&support::read::Text> = c.texts.iter().filter(|t| t.font == "F0").collect();
        assert_eq!(
            music.len(),
            e.glyphs.len(),
            "{name} page {}: music glyph count",
            i + 1
        );
        let mut used = vec![false; music.len()];
        for (em, x, y, ch) in &e.glyphs {
            let hit = music.iter().enumerate().position(|(j, t)| {
                !used[j]
                    && t.gray == 0.0
                    && close(t.size, *em)
                    && close(t.x, *x)
                    && close(t.y, *y)
                    && decode(&doc, &fonts, t) == ch.to_string()
            });
            let j = hit.unwrap_or_else(|| panic!("{name} page {}: glyph {ch:?} at {x},{y}", i + 1));
            used[j] = true;
            matched += 1;
        }
        // PD1: every line, with its width and grey.
        let mut used = vec![false; c.strokes.len()];
        for l in &e.lines {
            let hit = c.strokes.iter().enumerate().position(|(j, s)| {
                !used[j]
                    && close(s.x1, l[0])
                    && close(s.y1, l[1])
                    && close(s.x2, l[2])
                    && close(s.y2, l[3])
                    && close(s.width, l[4])
                    && close(s.gray, l[5])
            });
            let j = hit.unwrap_or_else(|| panic!("{name} page {}: line {l:?}", i + 1));
            used[j] = true;
            matched += 1;
        }
        // PD1: every beam band.
        for b in &e.beams {
            assert!(
                c.fills.iter().any(|f| f.path.len() == 5
                    && f.gray == 0.0
                    && close(f.path[0].1[0], b[0])
                    && close(f.path[0].1[1], b[1])
                    && close(f.path[1].1[0], b[2])
                    && close(f.path[1].1[1], b[3])),
                "{name} page {}: beam {b:?}",
                i + 1
            );
            matched += 1;
        }
        // PD1: every fret number, its text, centre and baseline.
        let text: Vec<(String, &support::read::Text)> = c
            .texts
            .iter()
            .filter(|t| t.font == "F1")
            .map(|t| (decode(&doc, &fonts, t), t))
            .collect();
        for (s, x, y, size) in &e.frets {
            let w = s.chars().count() as f32 * tf.digit_em * size;
            assert!(
                text.iter().any(|(d, t)| d == s
                    && t.gray == 0.0
                    && close(t.size, *size)
                    && close(t.x + w / 2.0, *x)
                    && close(t.y, *y)),
                "{name} page {}: fret {s} at {x},{y}",
                i + 1
            );
            matched += 1;
        }
        // PD5: the footer on every page, the title on page 1 only.
        let strings: Vec<&str> = text.iter().map(|(d, _)| d.as_str()).collect();
        let footer = format!("page {} of {}", i + 1, p.page_count());
        assert!(strings.contains(&footer.as_str()), "{name}: {footer}");
        assert_eq!(
            strings.contains(&title),
            i == 0,
            "{name} page {}: title",
            i + 1
        );
    }
    matched
}

#[test]
fn pd1_pd5_positions_and_pages_equal_the_layout() {
    let mut matched = 0;
    let mut pages = 0;
    for (name, score) in inputs() {
        for (size, setup) in setups() {
            matched += check(&format!("{name} {size}"), &score, &both(&score), &setup);
            pages += paginate(&score, &both(&score), &setup)
                .unwrap()
                .page_count();
        }
    }
    assert!(
        matched > 20_000 && pages > 120,
        "{matched} items, {pages} pages"
    );
}

/// PD5: split systems continue on the next page (LARGE with 8 bars, 20 tracks).
#[test]
fn pd5_split_systems_and_many_pages() {
    let score = mc_score::fixtures::reference(mc_score::fixtures::Shape {
        bars: 8,
        ..mc_score::fixtures::LARGE
    });
    let setup = PageSetup::default();
    let p = paginate(&score, &both(&score), &setup).unwrap();
    assert!(
        p.pages
            .iter()
            .flat_map(|pg| &pg.items)
            .any(|i| i.continued())
    );
    check("large-8", &score, &both(&score), &setup);
}

/// PD2: exactly the fonts used, each an embedded subset of one of the two, with the
/// full font's outline for every character it maps.
#[test]
fn pd2_only_the_two_fonts_embedded_with_their_outlines() {
    use skrifa::outline::{DrawSettings, OutlinePen};
    #[derive(Default, PartialEq, Debug)]
    struct Rec(Vec<(char, Vec<i64>)>);
    impl Rec {
        fn p(&mut self, c: char, v: &[f32]) {
            self.0
                .push((c, v.iter().map(|x| (x * 1000.0).round() as i64).collect()));
        }
    }
    impl OutlinePen for Rec {
        fn move_to(&mut self, x: f32, y: f32) {
            self.p('M', &[x, y]);
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.p('L', &[x, y]);
        }
        fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) {
            self.p('Q', &[a, b, x, y]);
        }
        fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
            self.p('C', &[a, b, c, d, x, y]);
        }
        fn close(&mut self) {
            self.p('Z', &[]);
        }
    }
    let outline = |f: &FontRef, g: u32| {
        let mut r = Rec::default();
        if let Some(o) = f.outline_glyphs().get(GlyphId::new(g)) {
            o.draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut r,
            )
            .unwrap();
        }
        r
    };
    let mut checked = 0;
    for (name, score, staves) in [
        (
            "feature_rich",
            mc_score::fixtures::feature_rich(),
            StaffConfiguration::NOTATION_AND_TAB,
        ),
        (
            "keys",
            mc_score::fixtures::notation_keys(),
            StaffConfiguration::NOTATION_AND_TAB,
        ),
        (
            "tab only",
            mc_score::fixtures::feature_rich(),
            StaffConfiguration::TAB,
        ),
    ] {
        let shown: Vec<ShownTrack> = score
            .tracks()
            .map(|t| ShownTrack {
                track: t.id(),
                staves,
            })
            .collect();
        let out = export_pdf(&score, &shown, &PageSetup::default(), "Fonts").unwrap();
        let doc = Doc::parse(&out.bytes);
        let fonts: Vec<(&u32, &support::read::Obj)> = doc
            .objs
            .iter()
            .filter(|(_, o)| {
                o.dict.contains("/Type /Font")
                    && !o.dict.contains("/Type /FontDescriptor")
                    && !o.dict.contains("/Subtype /CIDFontType")
            })
            .collect();
        let want = if staves.notation { 2 } else { 1 };
        assert_eq!(fonts.len(), want, "{name}: fonts in the file");
        assert_eq!(out.fonts.len(), want, "{name}");
        for (num, o) in fonts {
            assert!(
                o.dict.contains("/Subtype /Type0"),
                "{name}: only Type 0 fonts"
            );
            let base = support::read::between(&o.dict, "/BaseFont /", "\n");
            let (tag, family) = base.split_once('+').expect("a subset name");
            assert!(
                tag.len() == 6 && tag.chars().all(|c| c.is_ascii_uppercase()),
                "{name}: {base}"
            );
            let full_bytes = match family {
                "Bravura" => BRAVURA,
                "NotoSansMono" => NOTO,
                other => panic!("{name}: unexpected font {other}"),
            };
            assert!(
                out.fonts.iter().any(|f| f.name == base),
                "{name}: reported {base}"
            );
            // Descendant, descriptor, embedded file.
            let cid =
                support::read::refs(support::read::between(&o.dict, "/DescendantFonts [", "]"))[0];
            let desc =
                support::read::int_after(&doc.obj(cid).dict, "/FontDescriptor ").unwrap() as u32;
            let d = &doc.obj(desc).dict;
            let file = support::read::int_after(d, "/FontFile3 ")
                .or_else(|| support::read::int_after(d, "/FontFile2 "))
                .expect("an embedded font file") as u32;
            let sub_bytes = doc.data(file);
            let sub = FontRef::new(&sub_bytes).expect("the subset parses");
            let full = FontRef::new(full_bytes).unwrap();
            for (cid, ch) in doc.to_unicode(*num) {
                let g = full.charmap().map(ch).unwrap().to_u32();
                assert_eq!(
                    outline(&full, g),
                    outline(&sub, u32::from(cid)),
                    "{name}: {ch:?}"
                );
                checked += 1;
            }
        }
        // No other font kind and no unembedded reference anywhere.
        let all: String = doc.objs.values().map(|o| o.dict.as_str()).collect();
        for banned in [
            "/Subtype /Type1",
            "/Subtype /TrueType",
            "/Subtype /Type3",
            "/Subtype /MMType1",
        ] {
            assert!(!all.contains(banned), "{name}: {banned}");
        }
    }
    assert!(checked > 60, "{checked} glyphs compared");
}

/// PD3: a character missing from the fonts fails with its name; nothing is written.
#[test]
fn pd3_a_missing_character_is_an_error() {
    let score = mc_score::fixtures::feature_rich();
    let shown = both(&score);
    let setup = PageSetup::default();
    assert!(
        export_pdf(&score, &shown, &setup, "Bài hát số một").is_ok(),
        "Vietnamese is covered"
    );
    assert_eq!(
        export_pdf(&score, &shown, &setup, "Song 曲").unwrap_err(),
        PdfError::MissingGlyph {
            font: "NotoSansMono",
            ch: '曲'
        }
    );
    // A marker in the score.
    let mut s = score.clone();
    let mut bar = s.master_bars()[0].clone();
    bar.marker = Some("Intro ♫".into());
    s.set_master_bar(0, bar).unwrap();
    let e = export_pdf(&s, &both(&s), &setup, "Song").unwrap_err();
    assert_eq!(
        e,
        PdfError::MissingGlyph {
            font: "NotoSansMono",
            ch: '♫'
        }
    );
    assert!(e.to_string().contains("U+266B"), "{e}");
}

/// FNV-1a 64 of the bytes (the same function on every platform).
fn fnv(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for x in b {
        h ^= u64::from(*x);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// PD4: identical bytes in one process, and equal to the recorded hash in every
/// process and build (CI runs this test in debug and in release).
#[test]
fn pd4_bytes_are_identical_across_runs_and_builds() {
    let cases: [(&str, Score, u64); 2] = [
        (
            "feature_rich",
            mc_score::fixtures::feature_rich(),
            // Replaced in 6c-6 (ADR-041 section 13, choice 3), approved by the user.
            0xf471_347a_ea5d_f19d,
        ),
        (
            "keys",
            mc_score::fixtures::notation_keys(),
            // Replaced in 6c-6 (ADR-041 section 13, choice 3), approved by the user.
            0xf0dc_0a0c_5608_b7ad,
        ),
    ];
    let mut report = Vec::new();
    for (name, score, golden) in cases {
        let a = export_pdf(&score, &both(&score), &PageSetup::default(), name).unwrap();
        let b = export_pdf(&score, &both(&score), &PageSetup::default(), name).unwrap();
        assert_eq!(a.bytes, b.bytes, "{name}: two runs");
        let h = fnv(&a.bytes);
        report.push(format!("{name}: {h:#018x} ({} bytes)", a.bytes.len()));
        assert_eq!(h, golden, "{name}: recorded hash; all: {report:?}");
        // No date and no producer-specific clock anywhere.
        let text = String::from_utf8_lossy(&a.bytes);
        assert!(
            !text.contains("/CreationDate") && !text.contains("/ModDate"),
            "{name}"
        );
    }
}

/// PD6: export time on the reference machine, release build only:
/// `cargo test -p mc-pdf --release --test export pd6 -- --ignored --nocapture`.
/// TYPICAL must take at most 2 s; LARGE is reported.
#[test]
#[ignore = "timing; run in release on the reference machine"]
fn pd6_export_time() {
    use mc_score::fixtures::{LARGE, TYPICAL, reference};
    let mut typical_worst = 0.0f64;
    for (name, shape, tracks) in [
        ("TYPICAL", TYPICAL, 4),
        ("LARGE", LARGE, 4),
        ("LARGE", LARGE, 20),
    ] {
        let score = reference(shape);
        let shown: Vec<ShownTrack> = both(&score).into_iter().take(tracks).collect();
        for (spacing, s) in [
            ("spacious", SystemSpacing::Spacious),
            ("compact", SystemSpacing::Compact),
        ] {
            let setup = PageSetup {
                system_spacing: s,
                ..PageSetup::default()
            };
            let mut ms = Vec::new();
            let mut last = None;
            for _ in 0..5 {
                let t = std::time::Instant::now();
                let out = export_pdf(&score, &shown, &setup, name).unwrap();
                ms.push(t.elapsed().as_secs_f64() * 1e3);
                last = Some(out);
            }
            ms.sort_by(f64::total_cmp);
            let out = last.unwrap();
            println!(
                "PD6 {name}/{tracks} {spacing}: median {:.0} ms, max {:.0} ms, {} pages, {} bytes",
                ms[2],
                ms[4],
                out.pages,
                out.bytes.len()
            );
            if name == "TYPICAL" {
                typical_worst = typical_worst.max(ms[4]);
            }
        }
    }
    assert!(
        typical_worst <= 2000.0,
        "TYPICAL took {typical_worst:.0} ms"
    );
}

/// Choice 1: the spacing reaches the PDF. With several systems on a page, the second
/// system's first notation glyph moves down by the gap difference (19.2 - 9.6 pt).
#[test]
fn spacing_choice_moves_the_systems() {
    let score = samples::small();
    let first_glyph_y = |s: SystemSpacing| {
        let setup = PageSetup {
            system_spacing: s,
            ..PageSetup::default()
        };
        let p = paginate(&score, &both(&score), &setup).unwrap();
        assert!(p.pages[0].items.len() >= 2, "two systems on page 1");
        let out = export_pdf(&score, &both(&score), &setup, "Spacing").unwrap();
        let doc = Doc::parse(&out.bytes);
        let c = doc.content(doc.pages()[0]);
        // Systems are painted in order; the second item's top bounds its glyphs.
        let second = p.pages[0].items[1].y;
        let y = c
            .texts
            .iter()
            .filter(|t| t.font == "F0" && t.y > second)
            .map(|t| t.y)
            .fold(f32::INFINITY, f32::min);
        (second, y)
    };
    let (sa, ya) = first_glyph_y(SystemSpacing::Spacious);
    let (sc, yc) = first_glyph_y(SystemSpacing::Compact);
    assert!(close(sa - sc, 9.6), "{sa} {sc}");
    assert!(close(ya - yc, 9.6), "{ya} {yc}");
}
