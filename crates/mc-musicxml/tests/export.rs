//! ADR-033 E2 to E7: the exported file is parsed back (roxmltree, tests only) and
//! compared fact by fact with the score.

use mc_musicxml::{MusicXmlReport, export_musicxml};
use mc_score::fixtures::{Shape, feature_rich, reference};
use mc_score::{BeatRef, FrettedInstrument, InstrumentSpec, MasterBar, NoteEffects, Score, Track};
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning, pitch_at};
use mc_theory::{BaseValue, KeySignature, Mode, NoteValue, Rational, Tempo, TimeSignature, Tuplet};
use proptest::prelude::*;
use roxmltree::{Document, Node};

// ------------------------------------------------------------------ XML helpers

fn kids<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children()
        .filter(move |c| c.is_element() && c.tag_name().name() == name)
}

fn kid<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> Option<Node<'a, 'i>> {
    kids(n, name).next()
}

fn text<'a>(n: Node<'a, '_>, name: &str) -> Option<&'a str> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
        .and_then(|c| c.text())
}

fn num(n: Node, name: &str) -> i64 {
    text(n, name)
        .unwrap_or_else(|| panic!("<{name}> missing"))
        .parse()
        .unwrap()
}

fn has(n: Node, name: &str, attr: Option<(&str, &str)>) -> bool {
    n.descendants().any(|c| {
        c.is_element()
            && c.tag_name().name() == name
            && attr.is_none_or(|(k, v)| c.attribute(k) == Some(v))
    })
}

fn count(n: Node, name: &str, attr: Option<(&str, &str)>) -> usize {
    n.descendants()
        .filter(|c| {
            c.is_element()
                && c.tag_name().name() == name
                && attr.is_none_or(|(k, v)| c.attribute(k) == Some(v))
        })
        .count()
}

fn step_semitone(step: &str) -> i64 {
    match step {
        "C" => 0,
        "D" => 2,
        "E" => 4,
        "F" => 5,
        "G" => 7,
        "A" => 9,
        "B" => 11,
        s => panic!("bad step {s}"),
    }
}

fn type_of(v: NoteValue) -> &'static str {
    [
        "whole", "half", "quarter", "eighth", "16th", "32nd", "64th", "128th", "256th",
    ][v.base().exponent() as usize]
}

fn divisions_of(length: Rational, per_quarter: i64) -> i64 {
    let v = length.numerator() as i128 * 4 * per_quarter as i128;
    assert_eq!(v % length.denominator() as i128, 0, "exact duration");
    (v / length.denominator() as i128) as i64
}

// ------------------------------------------------------------ the full checker

/// True when the note at (bar, voice, beat, note) starts a tie, computed from the
/// score: the next beat in written order has a tied note at the same position.
fn starts_tie(track: &Track, bar: usize, v: usize, k: usize, pos: FretPosition) -> bool {
    let voice = &track.bar(bar).unwrap().voices[v];
    let next = if k + 1 < voice.beats.len() {
        voice.beats.get(k + 1)
    } else {
        track
            .bar(bar + 1)
            .and_then(|b| b.voices.get(v))
            .and_then(|vv| vv.beats.first())
    };
    next.is_some_and(|b| b.notes.iter().any(|n| n.tie && n.position() == pos))
}

/// E2 to E5: every structural and note-level fact of `score` is in `xml`.
fn check(score: &Score, xml: &str, report: &MusicXmlReport) {
    let doc = Document::parse(xml).expect("well-formed XML");
    let root = doc.root_element();
    assert_eq!(root.tag_name().name(), "score-partwise");
    assert_eq!(root.attribute("version"), Some("4.0"));
    let list: Vec<_> = kids(kid(root, "part-list").unwrap(), "score-part").collect();
    let parts: Vec<_> = kids(root, "part").collect();
    assert_eq!(list.len(), score.tracks().len());
    assert_eq!(parts.len(), score.tracks().len());
    assert_eq!(report.parts, parts.len());
    assert_eq!(report.measures, score.master_bar_count());

    let bars = score.master_bars();
    let pickup_first = bars.first().is_some_and(|b| b.pickup);
    let mut notes_seen = 0usize;
    for (ti, (track, part)) in score.tracks().zip(&parts).enumerate() {
        assert_eq!(list[ti].attribute("id"), part.attribute("id"));
        let measures: Vec<_> = kids(*part, "measure").collect();
        assert_eq!(
            measures.len(),
            score.master_bar_count(),
            "E2 one measure per bar"
        );
        let per_quarter = num(kid(measures[0], "attributes").unwrap(), "divisions");
        // E2: tie starts and stops pair up within the part.
        assert_eq!(
            count(*part, "tie", Some(("type", "start"))),
            count(*part, "tie", Some(("type", "stop"))),
            "tie start/stop pairs"
        );
        assert_eq!(
            count(*part, "tied", Some(("type", "start"))),
            count(*part, "tied", Some(("type", "stop")))
        );
        for (m, measure) in measures.iter().enumerate() {
            let expected_number = if pickup_first { m } else { m + 1 };
            assert_eq!(
                measure.attribute("number"),
                Some(expected_number.to_string().as_str())
            );
            assert_eq!(
                measure.attribute("implicit") == Some("yes"),
                bars[m].pickup,
                "pickup is implicit"
            );
            // Parsed beats per voice number, skipping measure rests and padding rests
            // (a rest without <type>).
            let mut voices: std::collections::BTreeMap<i64, Vec<Vec<Node>>> = Default::default();
            // Total written duration per voice, padding and measure rests included.
            let mut totals: std::collections::BTreeMap<i64, i64> = Default::default();
            for note in kids(*measure, "note") {
                let voice = num(note, "voice");
                if kid(note, "chord").is_none() {
                    *totals.entry(voice).or_default() += num(note, "duration");
                }
                let is_rest = kid(note, "rest").is_some();
                if is_rest && kid(note, "type").is_none() {
                    continue; // measure rest or padding
                }
                let beats = voices.entry(voice).or_default();
                if kid(note, "chord").is_some() {
                    beats.last_mut().expect("chord after a note").push(note);
                } else {
                    beats.push(vec![note]);
                }
            }
            let content = track.bar(m).unwrap();
            // Each written voice fills the measure: padded when short (choice 5.4),
            // longer only when the score voice itself is longer.
            let measure_len = if bars[m].pickup {
                score.effective_bar_duration(m).unwrap()
            } else {
                bars[m].time_signature.length()
            };
            let expected_len = divisions_of(measure_len, per_quarter);
            for (v, voice) in content.voices.iter().enumerate() {
                let Some(&total) = totals.get(&(v as i64 + 1)) else {
                    assert!(
                        voice.beats.is_empty() && (v > 0 || expected_len == 0),
                        "track {ti} bar {m} voice {v} missing"
                    );
                    continue;
                };
                let used: Rational = voice.beats.iter().fold(Rational::ZERO, |a, b| {
                    a.checked_add(b.value.length()).unwrap()
                });
                let used = divisions_of(used, per_quarter);
                assert_eq!(
                    total,
                    used.max(expected_len),
                    "track {ti} bar {m} voice {v}: length"
                );
            }
            for (v, voice) in content.voices.iter().enumerate() {
                let parsed = voices.remove(&(v as i64 + 1)).unwrap_or_default();
                assert_eq!(
                    parsed.len(),
                    voice.beats.len(),
                    "track {ti} bar {m} voice {v}: beat count"
                );
                for (k, (beat, xml_beat)) in voice.beats.iter().zip(&parsed).enumerate() {
                    let first = xml_beat[0];
                    assert_eq!(
                        num(first, "duration"),
                        divisions_of(beat.value.length(), per_quarter),
                        "E3 duration"
                    );
                    assert_eq!(text(first, "type"), Some(type_of(beat.value)));
                    assert_eq!(kids(first, "dot").count(), beat.value.dots() as usize);
                    let tm = kid(first, "time-modification");
                    match beat.value.tuplet() {
                        Some(t) => {
                            let tm = tm.expect("time-modification");
                            assert_eq!(num(tm, "actual-notes"), t.actual() as i64);
                            assert_eq!(num(tm, "normal-notes"), t.normal() as i64);
                        }
                        None => assert!(tm.is_none()),
                    }
                    if beat.notes.is_empty() {
                        assert!(kid(first, "rest").is_some(), "E3 rest");
                        assert_eq!(xml_beat.len(), 1);
                        continue;
                    }
                    assert_eq!(xml_beat.len(), beat.notes.len(), "E3 chord size");
                    for (note, xn) in beat.notes.iter().zip(xml_beat) {
                        notes_seen += 1;
                        let pos = note.position();
                        let tech = xn
                            .descendants()
                            .find(|c| c.tag_name().name() == "technical")
                            .expect("technical");
                        assert_eq!(num(tech, "string"), pos.string.get() as i64, "E3 string");
                        assert_eq!(num(tech, "fret"), pos.fret.0 as i64, "E3 fret");
                        let p = kid(*xn, "pitch").expect("pitch");
                        // Spelling follows the key (choice 5.3): no sharps in flat keys,
                        // no flats otherwise.
                        let alter = text(p, "alter").map_or(0, |a| a.parse::<i64>().unwrap());
                        if bars[m].key.fifths() < 0 {
                            assert!(alter <= 0, "sharp in a flat key");
                        } else {
                            assert!(alter >= 0, "flat outside a flat key");
                        }
                        let semitone = (num(p, "octave") + 1) * 12
                            + step_semitone(text(p, "step").unwrap())
                            + text(p, "alter").map_or(0, |a| a.parse::<i64>().unwrap());
                        let f = track.fretted();
                        assert_eq!(
                            semitone,
                            pitch_at(&f.tuning, f.capo, pos).unwrap() as i64,
                            "E3 sounding pitch"
                        );
                        assert_eq!(has(*xn, "tie", Some(("type", "stop"))), note.tie);
                        assert_eq!(
                            has(*xn, "tie", Some(("type", "start"))),
                            starts_tie(track, m, v, k, pos),
                            "E3 tie start"
                        );
                        assert_eq!(
                            count(*xn, "tied", Some(("type", "let-ring"))),
                            usize::from(note.effects.let_ring),
                            "E3 let ring"
                        );
                        assert_eq!(text(*xn, "notehead") == Some("x"), note.effects.dead);
                    }
                }
            }
            assert!(voices.is_empty(), "no extra voices: {:?}", voices.keys());
        }
    }
    // E5: nothing dropped.
    assert_eq!(notes_seen, score.note_count());
    assert_eq!(report.notes, score.note_count());
}

fn export(score: &Score) -> (String, MusicXmlReport) {
    export_musicxml(score).expect("export")
}

// ----------------------------------------------------------------------- E2

#[test]
fn e2_e5_feature_rich_round_trips_every_fact() {
    let s = feature_rich();
    let (xml, report) = export(&s);
    check(&s, &xml, &report);
}

#[test]
fn e2_structure_of_feature_rich() {
    let s = feature_rich();
    let (xml, _) = export(&s);
    let doc = Document::parse(&xml).unwrap();
    let root = doc.root_element();
    let names: Vec<_> = kids(kid(root, "part-list").unwrap(), "score-part")
        .map(|p| text(p, "part-name").unwrap().to_string())
        .collect();
    assert_eq!(names, ["Lead", "Rhythm"]);
    let lead = kid(root, "part").unwrap();
    let ms: Vec<_> = kids(lead, "measure").collect();
    assert_eq!(ms.len(), 5);
    // Pickup measure 0; repeat start and marker on 1; ending 1 and repeat end on 2;
    // ending 2 on 3; 6/8 and E-flat minor on 4 with a mid-bar tempo of 120.5.
    assert_eq!(ms[0].attribute("implicit"), Some("yes"));
    let attrs0 = kid(ms[0], "attributes").unwrap();
    assert_eq!(text(kid(attrs0, "key").unwrap(), "fifths"), Some("0"));
    assert_eq!(text(kid(attrs0, "time").unwrap(), "beats"), Some("4"));
    assert!(has(ms[1], "repeat", Some(("direction", "forward"))));
    assert_eq!(
        ms[1]
            .descendants()
            .find(|n| n.tag_name().name() == "rehearsal")
            .and_then(|n| n.text()),
        Some("Verse")
    );
    let back = ms[2]
        .descendants()
        .find(|n| n.tag_name().name() == "repeat")
        .unwrap();
    assert_eq!(back.attribute("direction"), Some("backward"));
    assert_eq!(back.attribute("times"), Some("2"));
    let endings = |m: Node| -> Vec<(String, String)> {
        m.descendants()
            .filter(|n| n.tag_name().name() == "ending")
            .map(|n| {
                (
                    n.attribute("number").unwrap().to_string(),
                    n.attribute("type").unwrap().to_string(),
                )
            })
            .collect()
    };
    assert_eq!(
        endings(ms[2]),
        [("1".into(), "start".into()), ("1".into(), "stop".into())]
    );
    assert_eq!(
        endings(ms[3]),
        [
            ("2".into(), "start".into()),
            ("2".into(), "discontinue".into())
        ]
    );
    let attrs4 = kid(ms[4], "attributes").unwrap();
    let key4 = kid(attrs4, "key").unwrap();
    assert_eq!(text(key4, "fifths"), Some("-3"));
    assert_eq!(text(key4, "mode"), Some("minor"));
    assert_eq!(text(kid(attrs4, "time").unwrap(), "beat-type"), Some("8"));
    let tempos: Vec<_> = lead
        .descendants()
        .filter(|n| n.tag_name().name() == "sound")
        .map(|n| n.attribute("tempo").unwrap().to_string())
        .collect();
    assert_eq!(tempos, ["96", "120.5"]);
    // 3/8 of a whole note = 1.5 quarters, in divisions, applied to playback.
    let per_quarter = num(attrs0, "divisions");
    let offset = ms[4]
        .descendants()
        .find(|n| n.tag_name().name() == "offset")
        .unwrap();
    assert_eq!(offset.attribute("sound"), Some("yes"));
    assert_eq!(
        offset.text().unwrap().parse::<i64>().unwrap(),
        per_quarter * 3 / 2
    );
    // Song-level directions only in the first part.
    let rhythm = kids(root, "part").nth(1).unwrap();
    assert_eq!(count(rhythm, "direction", None), 0);
}

// ----------------------------------------------------------------------- E4

#[test]
fn e4_tab_staff_with_drop_d_and_capo() {
    let s = feature_rich();
    let (xml, _) = export(&s);
    let doc = Document::parse(&xml).unwrap();
    let parts: Vec<_> = kids(doc.root_element(), "part").collect();
    for (p, low, capo) in [
        (parts[0], ("E", "2"), None),
        (parts[1], ("D", "2"), Some("2")),
    ] {
        let attrs = kid(kid(p, "measure").unwrap(), "attributes").unwrap();
        assert_eq!(text(kid(attrs, "clef").unwrap(), "sign"), Some("TAB"));
        let sd = kid(attrs, "staff-details").unwrap();
        assert_eq!(text(sd, "staff-lines"), Some("6"));
        let tunings: Vec<_> = kids(sd, "staff-tuning").collect();
        assert_eq!(tunings.len(), 6);
        assert_eq!(tunings[0].attribute("line"), Some("1"));
        assert_eq!(text(tunings[0], "tuning-step"), Some(low.0));
        assert_eq!(text(tunings[0], "tuning-octave"), Some(low.1));
        assert_eq!(text(tunings[5], "tuning-step"), Some("E"));
        assert_eq!(text(tunings[5], "tuning-octave"), Some("4"));
        assert_eq!(text(sd, "capo"), capo);
    }
}

// ----------------------------------------------------------------------- E5

#[test]
fn e5_feature_rich_report_counts_the_approximations() {
    let (_, report) = export(&feature_rich());
    // Lead bar 1 voice 0 is a dotted quarter, three triplet eighths and a quarter:
    // 7/8 of a 4/4 bar, padded. Rhythm bar 3 has five quarters in 4/4: overfull.
    assert_eq!(report.padded_voices, 1);
    assert_eq!(report.overfull_measures, 1);
    assert_eq!(report.tuplets_without_bracket, 0);
    assert_eq!(report.tempo_rounded, 0);
    assert_eq!(report.characters_removed, 0);
}

#[test]
fn e5_overfull_measure_keeps_every_note() {
    let s = feature_rich();
    let (xml, _) = export(&s);
    let doc = Document::parse(&xml).unwrap();
    let rhythm = kids(doc.root_element(), "part").nth(1).unwrap();
    let bar3 = kids(rhythm, "measure").nth(3).unwrap();
    assert_eq!(kids(bar3, "note").count(), 5, "five quarters, none cut");
}

// ----------------------------------------------------------------------- E6

const GOLDEN_FEATURE_RICH: &str = include_str!("golden/feature_rich.musicxml");
const GOLDEN_SMALL: &str = include_str!("golden/small.musicxml");

fn small() -> Score {
    reference(Shape {
        tracks: 1,
        bars: 8,
        beats: 4,
        notes_per_beat: 2,
    })
}

#[test]
fn e6_output_is_deterministic_and_matches_the_goldens() {
    for (name, score, golden) in [
        ("feature_rich", feature_rich(), GOLDEN_FEATURE_RICH),
        ("small", small(), GOLDEN_SMALL),
    ] {
        let (a, _) = export(&score);
        let (b, _) = export(&score);
        assert_eq!(a, b, "{name}: byte-identical");
        if a != golden.replace("\r\n", "\n") {
            println!("{a}");
            panic!("{name}: output differs from tests/golden/{name}.musicxml");
        }
    }
}

/// Rewrites the golden files. Review the diff before committing:
///   cargo test -p mc-musicxml --test export -- --ignored update_goldens
#[test]
#[ignore = "rewrites the golden files"]
fn update_goldens() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("feature_rich.musicxml"), export(&feature_rich()).0).unwrap();
    std::fs::write(dir.join("small.musicxml"), export(&small()).0).unwrap();
}

// ----------------------------------------------------- random scores (E3, E5, E7)

/// A small deterministic generator, so a failing case is reproducible from its seed.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
}

fn values() -> Vec<NoteValue> {
    let mut v = Vec::new();
    for e in 0..=5u8 {
        let base = BaseValue::from_exponent(e).unwrap();
        v.push(NoteValue::plain(base));
        v.push(NoteValue::new(base, 1, None).unwrap());
        v.push(NoteValue::new(base, 0, Some(Tuplet::TRIPLET)).unwrap());
    }
    v.push(NoteValue::new(BaseValue::SIXTEENTH, 0, Some(Tuplet::new(5, 4).unwrap())).unwrap());
    v.push(NoteValue::new(BaseValue::EIGHTH, 2, None).unwrap());
    v
}

/// A random but valid score, built through the aggregate operations (the validated
/// path). Rejected operations (an invalid repeat or tie) are simply skipped.
fn random_score(seed: u64) -> Score {
    let mut rng = Rng(seed | 1);
    let sigs = [(4, 4), (3, 4), (6, 8), (5, 4), (2, 2), (7, 8)];
    let (n, d) = sigs[rng.below(sigs.len() as u64) as usize];
    let first = MasterBar {
        pickup: rng.chance(30),
        ..MasterBar::new(
            TimeSignature::new(n, d).unwrap(),
            KeySignature::new(rng.below(15) as i8 - 7, Mode::Major).unwrap(),
        )
    };
    let instrument = |rng: &mut Rng| {
        let tuning = if rng.chance(50) {
            Tuning::standard_guitar()
        } else {
            Tuning::new(
                ["E4", "B3", "G3", "D3", "A2", "D2"]
                    .map(|p| p.parse().unwrap())
                    .to_vec(),
            )
            .unwrap()
        };
        InstrumentSpec::Fretted(FrettedInstrument {
            tuning,
            fret_count: 24,
            capo: Capo(rng.below(4) as u8),
        })
    };
    let mut s = Score::new(
        "T1",
        instrument(&mut rng),
        first,
        Tempo::from_bpm(100).unwrap(),
    )
    .unwrap();
    let bars = 1 + rng.below(5) as usize;
    if bars > 1 {
        s.insert_master_bars(1, bars - 1, MasterBar::default())
            .unwrap();
    }
    for m in 1..bars {
        let (n, d) = sigs[rng.below(sigs.len() as u64) as usize];
        let mut mb = MasterBar::new(
            TimeSignature::new(n, d).unwrap(),
            KeySignature::new(rng.below(15) as i8 - 7, Mode::Minor).unwrap(),
        );
        mb.repeat_start = rng.chance(20);
        if rng.chance(20) {
            mb.repeat_end = Some(2 + rng.below(3) as u8);
        }
        if rng.chance(20) {
            mb.marker = Some(format!("M{m} <&>"));
        }
        let _ = s.set_master_bar(m, mb);
        if rng.chance(30) {
            let _ = s.set_tempo(
                m,
                Rational::new(rng.below(4) as i64, 8).unwrap(),
                Tempo::new(
                    Rational::new(60 + rng.below(140) as i64, 1 + rng.below(3) as i64).unwrap(),
                )
                .unwrap(),
            );
        }
    }
    for t in 1..1 + rng.below(3) {
        let _ = s.add_track(&format!("T{}", t + 1), instrument(&mut rng));
    }
    let values = values();
    let ids: Vec<_> = s.tracks().map(|t| t.id()).collect();
    for &track in &ids {
        // The last beat's notes of each voice in the previous bar, so ties can cross
        // the bar line (H-06: the preceding beat of beat 0 is the previous bar's last).
        let mut last_of_voice: Vec<Option<Vec<FretPosition>>> = vec![None, None];
        for bar in 0..bars {
            let voices = 1 + usize::from(rng.chance(25));
            for (v, slot) in last_of_voice.iter_mut().enumerate().take(voices) {
                if v > 0 && s.add_voice(track, bar).is_err() {
                    continue;
                }
                let mut previous: Option<Vec<FretPosition>> = slot.take();
                let mut any_beat = false;
                for k in 0..rng.below(7) as usize {
                    let value = values[rng.below(values.len() as u64) as usize];
                    let Ok((beat, _)) = s.insert_beat(track, bar, v, k, value) else {
                        break;
                    };
                    any_beat = true;
                    let r = BeatRef {
                        track,
                        bar,
                        voice: v,
                        beat,
                    };
                    let mut placed = Vec::new();
                    // Sometimes repeat the previous beat's notes and tie them.
                    let copy_and_tie =
                        previous.as_ref().is_some_and(|p| !p.is_empty()) && rng.chance(40);
                    if copy_and_tie {
                        for &pos in previous.as_ref().unwrap() {
                            if let Ok((id, _)) = s.set_note(r, pos) {
                                let _ = s.set_tie(r, id, true);
                                placed.push(pos);
                            }
                        }
                    } else if rng.chance(80) {
                        for _ in 0..rng.below(4) {
                            let pos = FretPosition {
                                string: StringIndex::new(1 + rng.below(6) as u8).unwrap(),
                                fret: Fret(rng.below(16) as u8),
                            };
                            if let Ok((id, _)) = s.set_note(r, pos) {
                                placed.push(pos);
                                let effects = NoteEffects {
                                    let_ring: rng.chance(20),
                                    dead: rng.chance(10),
                                };
                                let _ = s.set_note_effects(r, id, effects);
                            }
                        }
                    }
                    previous = Some(placed);
                }
                *slot = if any_beat { previous } else { None };
            }
        }
    }
    assert!(s.validate().is_empty());
    s
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, .. ProptestConfig::default() })]

    /// E3, E5: every fact of a random valid score is in the file, nothing is lost.
    #[test]
    fn e3_random_scores_round_trip_every_fact(seed in any::<u64>()) {
        let s = random_score(seed);
        let (xml, report) = export(&s);
        check(&s, &xml, &report);
    }

    /// E7: names and markers with markup characters, non-ASCII text and control
    /// characters give valid XML; forbidden characters are removed and counted.
    #[test]
    fn e7_any_name_and_marker_gives_valid_xml(name in any::<String>(), marker in any::<String>()) {
        let mut s = small();
        let id = s.track_at(0).unwrap().id();
        s.rename_track(id, &name).unwrap();
        let mut mb = s.master_bars()[0].clone();
        mb.marker = Some(marker.clone());
        s.set_master_bar(0, mb).unwrap();
        let (xml, report) = export(&s);
        let doc = Document::parse(&xml).expect("valid XML");
        let allowed = |c: &char| matches!(c, '\t' | '\n' | '\r')
            || (*c >= ' ' && *c != '\u{FFFE}' && *c != '\u{FFFF}');
        let clean = |t: &str| t.chars().filter(allowed).collect::<String>();
        let removed = |t: &str| t.chars().filter(|c| !allowed(c)).count();
        let part_name = doc.descendants()
            .find(|n| n.tag_name().name() == "part-name")
            .and_then(|n| n.text()).unwrap_or("").to_string();
        prop_assert_eq!(part_name, clean(&name));
        let rehearsal = doc.descendants()
            .find(|n| n.tag_name().name() == "rehearsal")
            .and_then(|n| n.text()).unwrap_or("").to_string();
        prop_assert_eq!(rehearsal, clean(&marker));
        prop_assert_eq!(report.characters_removed, removed(&name) + removed(&marker));
    }
}

#[test]
fn e3_a_tie_across_a_bar_line_pairs_up() {
    // feature_rich ties the pickup's note to the first note of bar 1.
    let s = feature_rich();
    let (xml, _) = export(&s);
    let doc = Document::parse(&xml).unwrap();
    let lead = kids(doc.root_element(), "part").next().unwrap();
    let ms: Vec<_> = kids(lead, "measure").collect();
    assert!(has(ms[0], "tie", Some(("type", "start"))));
    assert!(has(ms[1], "tie", Some(("type", "stop"))));
}

#[test]
fn e3_the_triplet_group_is_bracketed() {
    let s = feature_rich();
    let (xml, _) = export(&s);
    let doc = Document::parse(&xml).unwrap();
    let lead = kids(doc.root_element(), "part").next().unwrap();
    let bar1 = kids(lead, "measure").nth(1).unwrap();
    assert_eq!(count(bar1, "tuplet", Some(("type", "start"))), 1);
    assert_eq!(count(bar1, "tuplet", Some(("type", "stop"))), 1);
}

/// The random scores actually exercise what E3 and E5 check (no vacuous property).
#[test]
fn random_scores_cover_ties_let_ring_tuplets_voices_and_bar_lengths() {
    let (
        mut ties,
        mut ring,
        mut dead,
        mut tuplets,
        mut voices2,
        mut padded,
        mut overfull,
        mut loose,
        mut rests,
    ) = (0, 0, 0, 0, 0, 0, 0, 0, 0);
    let mut cross_bar_ties = 0;
    for seed in 0..200u64 {
        let s = random_score(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let (_, report) = export(&s);
        padded += report.padded_voices;
        overfull += report.overfull_measures;
        loose += report.tuplets_without_bracket;
        for t in s.tracks() {
            for bar in t.bars() {
                voices2 += usize::from(bar.voices.len() > 1);
                for (vi, v) in bar.voices.iter().enumerate() {
                    for (k, b) in v.beats.iter().enumerate() {
                        tuplets += usize::from(b.value.tuplet().is_some());
                        rests += usize::from(b.notes.is_empty());
                        for n in &b.notes {
                            ties += usize::from(n.tie);
                            cross_bar_ties += usize::from(n.tie && k == 0);
                            ring += usize::from(n.effects.let_ring);
                            dead += usize::from(n.effects.dead);
                        }
                        let _ = vi;
                    }
                }
            }
        }
    }
    let all = [
        ties,
        cross_bar_ties,
        ring,
        dead,
        tuplets,
        voices2,
        padded,
        overfull,
        loose,
        rests,
    ];
    println!(
        "ties {ties} cross-bar {cross_bar_ties} ring {ring} dead {dead} tuplets {tuplets} two-voice bars {voices2} padded {padded} overfull {overfull} loose tuplets {loose} rests {rests}"
    );
    assert!(all.iter().all(|&c| c > 0), "{all:?}");
}
