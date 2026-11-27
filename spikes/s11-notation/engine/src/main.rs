mod check;
mod evidence;
mod layout;
mod model;
mod musicxml;
mod pdf;
mod pitch;
mod samples;
mod smufl;
mod svg;

use model::{Config, StaffConfiguration};

/// The review configuration: 1 000 units wide at 8 units per staff space, scaled by
/// `S11_SCALE` for larger review images (the layout in staff spaces is the same).
fn config(width: f32) -> Config {
    let scale: f32 = std::env::var("S11_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
    Config {
        staff_space: 8.0 * scale,
        width: width * scale,
        staves: StaffConfiguration::GUITAR,
    }
}

/// Per measure and voice, the notes in onset order (a chord low to high), with the
/// accidental shown: ours from the layout facts, MuseScore's from `<accidental>`.
fn xcheck(l: &model::Layout, ms: &str) -> (usize, usize, Vec<String>) {
    use std::collections::BTreeMap;
    // Ours, first track only (the spike's MusicXML parts are one per track).
    let mut ours: BTreeMap<(usize, usize), Vec<(usize, i32, Option<i8>)>> = BTreeMap::new();
    for sys in &l.systems {
        for st in sys.staves.iter().filter(|s| s.kind == model::StaffKind::Notation && s.position.track == 0) {
            for n in &st.notes {
                ours.entry((n.bar, n.voice)).or_default().push((n.beat, n.position, n.accidental));
            }
        }
    }
    for v in ours.values_mut() {
        v.sort();
    }
    // MuseScore writes a DOCTYPE; it is allowed here (the file is our own conversion).
    let opts = roxmltree::ParsingOptions { allow_dtd: true, ..roxmltree::ParsingOptions::default() };
    let doc = roxmltree::Document::parse_with_options(ms, opts).expect("MuseScore XML");
    let part = doc.descendants().find(|n| n.has_tag_name("part")).unwrap();
    let alter_of = |a: &str| match a {
        "sharp" => Some(1),
        "flat" => Some(-1),
        "natural" => Some(0),
        "double-sharp" => Some(2),
        "flat-flat" => Some(-2),
        _ => None,
    };
    let (mut same, mut differ, mut notes) = (0, 0, Vec::new());
    for (bar, m) in part.children().filter(|n| n.has_tag_name("measure")).enumerate() {
        // MuseScore's notes per voice, a chord sorted by pitch.
        let mut theirs: BTreeMap<usize, Vec<Vec<(i32, Option<i8>)>>> = BTreeMap::new();
        for n in m.children().filter(|n| n.has_tag_name("note")) {
            let Some(p) = n.children().find(|c| c.has_tag_name("pitch")) else { continue };
            fn text<'a>(node: roxmltree::Node<'a, 'a>, tag: &str) -> &'a str {
                node.children().find(|c| c.has_tag_name(tag)).and_then(|c| c.text()).unwrap_or("")
            }
            let step = "CDEFGAB".find(text(p, "step")).unwrap() as i32;
            let octave: i32 = text(p, "octave").parse().unwrap();
            let voice: usize = text(n, "voice").parse::<usize>().unwrap_or(1) - 1;
            let acc = n.children().find(|c| c.has_tag_name("accidental")).and_then(|a| a.text()).and_then(alter_of);
            // Written position: sounding + one octave on the treble staff.
            let pos = (octave + 1) * 7 + step - 30;
            let chord = n.children().any(|c| c.has_tag_name("chord"));
            let list = theirs.entry(voice).or_default();
            if chord {
                list.last_mut().unwrap().push((pos, acc));
            } else {
                list.push(vec![(pos, acc)]);
            }
        }
        for (voice, chords) in theirs {
            let flat: Vec<(i32, Option<i8>)> = chords
                .into_iter()
                .flat_map(|mut c| {
                    c.sort();
                    c
                })
                .collect();
            let mine: Vec<(i32, Option<i8>)> = ours.get(&(bar, voice)).map(|v| v.iter().map(|x| (x.1, x.2)).collect()).unwrap_or_default();
            for (i, t) in flat.iter().enumerate() {
                match mine.get(i) {
                    Some(o) if o.0 == t.0 && o.1 == t.1 => same += 1,
                    other => {
                        differ += 1;
                        notes.push(format!("bar {} note {}: ours {:?}, MuseScore {:?}", bar + 1, i + 1, other, t));
                    }
                }
            }
        }
    }
    (same, differ, notes)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = std::env::var("BRAVURA_DIR").expect("set BRAVURA_DIR");
    let font = smufl::Font::load(&dir).expect("font");
    let out = args.get(1).cloned().unwrap_or_else(|| "out".into());
    std::fs::create_dir_all(&out).unwrap();
    match args.first().map(String::as_str) {
        Some("render") => {
            let url = format!("file:///{}/Bravura.otf", dir.replace('\\', "/"));
            for (name, score, _) in samples::all() {
                let cfg = config(1000.0);
                let l = layout::Engine::new(&score, &font, &cfg).layout(None);
                let svg = svg::render(&l, Some(&url));
                std::fs::write(format!("{out}/{name}.svg"), &svg).unwrap();
                println!("{name}: {} systems, height {:.0}", l.systems.len(), l.height);
            }
        }
        Some("check") => {
            for (name, score, hand_built) in samples::all() {
                let cfg = config(1000.0);
                let l = layout::Engine::new(&score, &font, &cfg).layout(None);
                let r = check::check(&score, &l, &font);
                let checks: usize = r.n3_checks.values().sum();
                println!(
                    "{name}{}: {} notes; N1 {} errors; N2 {} errors (max bar line {:.4} units, max anchor {:.4} sp over {} beats); N3 {} violations in {} rule checks",
                    if hand_built { " (hand-built)" } else { "" },
                    r.notes, r.n1.len(), r.n2.len(), r.n2_max_barline, r.n2_max_anchor_sp, r.n2_anchors, r.n3.len(), checks
                );
                for e in r.n1.iter().chain(&r.n2).chain(&r.n3).take(6) {
                    println!("    {e}");
                }
            }
        }
        Some("musicxml") => {
            for (name, score, _) in samples::all() {
                std::fs::write(format!("{out}/{name}.musicxml"), musicxml::write(&score)).unwrap();
            }
        }
        Some("xcheck") => {
            // Our accidentals against MuseScore's own, from its re-export of our file.
            let (mut same, mut differ) = (0, 0);
            for (name, score, _) in samples::all() {
                let path = format!("{out}/{name}.ms.musicxml");
                let Ok(text) = std::fs::read_to_string(&path) else {
                    println!("{name}: no MuseScore re-export");
                    continue;
                };
                let cfg = config(1000.0);
                let l = layout::Engine::new(&score, &font, &cfg).layout(None);
                let (s, d, notes) = xcheck(&l, &text);
                same += s;
                differ += d;
                println!("{name}: {s} of {} notes agree with MuseScore{}", s + d, if notes.is_empty() { String::new() } else { format!("; differ: {}", notes.join("; ")) });
            }
            println!("total: {same} agree, {differ} differ");
        }
        Some("timing") => {
            let typical = mc_score::fixtures::reference(mc_score::fixtures::TYPICAL);
            let large = mc_score::fixtures::reference(mc_score::fixtures::LARGE);
            print!("{}", evidence::timing(&font, &typical, &large, &mc_score::fixtures::feature_rich()));
        }
        Some("pdf") => {
            let fr = mc_score::fixtures::feature_rich();
            let ty = mc_score::fixtures::reference(mc_score::fixtures::TYPICAL);
            print!("{}", evidence::pdfs(&font, &[("feature_rich", &fr), ("typical", &ty)], &out));
        }
        Some("determinism") => {
            let inputs = samples::all();
            let refs: Vec<(&str, &mc_score::Score)> = inputs.iter().map(|(n, s, _)| (*n, s)).collect();
            let (table, files) = evidence::determinism(&font, &refs);
            print!("{table}");
            for (name, bytes) in files {
                std::fs::write(format!("{out}/{name}"), bytes).unwrap();
            }
        }
        Some("dump") => {
            let name = args.get(1).expect("input name");
            let (_, score, _) = samples::all().into_iter().find(|(n, _, _)| n == name).expect("input");
            let cfg = config(1000.0);
            print!("{}", layout::Engine::new(&score, &font, &cfg).layout(None).canonical());
        }
        _ => println!("usage: s11-notation render <out> | dump <input>"),
    }
}
