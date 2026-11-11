//! Export acceptance tests (ADR-036 section 6: G7, G8, G10, and the G11 timing run).
//! Every export is read back with our importer and compared fact by fact with the
//! score, after the approximations the target version needs.

mod common;

use common::{FIXTURES, describe, describe_without_tempo, expected_after_export};
use mc_guitarpro::{GpExportIssueKind as X, GpExportReport, GpVersion, export_gp, import_gp};
use mc_score::fixtures::{Shape, TYPICAL, feature_rich, reference, techniques};
use mc_score::{BeatRef, FrettedInstrument, InstrumentSpec, MasterBar, NoteEffects, Score, Stroke};
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning};
use mc_theory::{BaseValue, KeySignature, Mode, NoteValue, Rational, Tempo, TimeSignature, Tuplet};
use proptest::prelude::*;
use std::collections::BTreeSet;

const VERSIONS: [GpVersion; 3] = [GpVersion::Gp5, GpVersion::Gp4, GpVersion::Gp3];

fn count(r: &GpExportReport, kind: X) -> usize {
    r.issues
        .iter()
        .filter(|i| i.kind == kind)
        .map(|i| i.count)
        .sum()
}

/// Half-up rounding of a tempo to whole BPM, as the exporter writes it.
fn rounded(t: Tempo) -> i64 {
    let (n, d) = (t.bpm().numerator() as i128, t.bpm().denominator() as i128);
    ((2 * n + d) / (2 * d)) as i64
}

/// G7 and G8 for one score and version: export, import, compare. Returns the report.
fn round_trip(score: &Score, v: GpVersion) -> GpExportReport {
    let (bytes, report) = export_gp(score, v).expect("export");
    let (back, import) = import_gp(&bytes).unwrap_or_else(|e| panic!("{v:?}: re-import: {e}"));
    // G8: nothing is lost silently.
    assert_eq!(
        report.notes_written + report.notes_dropped,
        score.note_count()
    );
    assert_eq!(import.notes_imported, report.notes_written, "{v:?}");
    assert_eq!(import.notes_dropped, 0, "{v:?}: {:?}", import.issues);
    // G7: every fact the version holds.
    let (got, want) = (
        describe_without_tempo(&back),
        expected_after_export(score, v),
    );
    if got != want {
        let line = got
            .lines()
            .zip(want.lines())
            .find(|(a, b)| a != b)
            .map(|(a, b)| format!("got  {a}\nwant {b}"))
            .unwrap_or_else(|| "one description is a prefix of the other".into());
        panic!("{v:?}: the re-import differs\n{line}");
    }
    // Tempo: every change keeps its bar and its rounded BPM; only the counted moves
    // change an offset, and only counted replacements lose a change.
    let before = score.tempo_map();
    let after = back.tempo_map();
    let dropped = count(&report, X::TempoDropped);
    assert_eq!(after.len() + dropped, before.len(), "{v:?}");
    let bpm = |t: Tempo| t.bpm().numerator();
    let mut moved = 0;
    for t in before {
        let exact = after
            .iter()
            .any(|a| (a.bar, a.offset, bpm(a.tempo)) == (t.bar, t.offset, rounded(t.tempo)));
        if !exact {
            moved += 1;
        }
    }
    assert!(
        moved <= count(&report, X::TempoMoved) + dropped,
        "{v:?}: {moved} moved"
    );
    // Every change read back is one of the score's, in its bar with its rounded BPM.
    for a in after {
        assert!(
            before
                .iter()
                .any(|t| t.bar == a.bar && rounded(t.tempo) == bpm(a.tempo)),
            "{v:?}: unexpected tempo {:?} in bar {}",
            a.tempo,
            a.bar
        );
    }
    report
}

#[test]
fn g7_reference_scores_round_trip() {
    for score in [feature_rich(), techniques(), reference(TYPICAL)] {
        for v in VERSIONS {
            round_trip(&score, v);
        }
    }
}

#[test]
fn g7_feature_rich_reports_its_approximations() {
    let s = feature_rich();
    let r = round_trip(&s, GpVersion::Gp5);
    assert_eq!(count(&r, X::PickupWritten), 1);
    assert_eq!(count(&r, X::TempoRounded), 1); // 120.5 BPM
    assert_eq!(count(&r, X::TempoMoved), 1); // 3/8 into an empty bar
    assert_eq!(r.notes_dropped, 0);
    // GP4 has one voice: the dead note of the second voice is dropped.
    let r = round_trip(&s, GpVersion::Gp4);
    assert_eq!(count(&r, X::VoiceDropped), 1);
    assert_eq!(r.notes_dropped, 1);
}

#[test]
fn g7_techniques_per_version() {
    let s = techniques();
    let r5 = round_trip(&s, GpVersion::Gp5);
    assert_eq!(count(&r5, X::AccentNotWritten), 0);
    assert!(count(&r5, X::StringPercussiveNotWritten) > 0);
    let r3 = round_trip(&s, GpVersion::Gp3);
    assert!(count(&r3, X::AccentNotWritten) > 0);
    assert!(count(&r3, X::PalmMuteNotWritten) > 0);
    assert!(count(&r3, X::StrokeNotWritten) > 0);
}

/// G7: what our importer reads from a fixture survives our export and re-import.
#[test]
fn g7_imported_fixtures_round_trip() {
    for (name, bytes) in FIXTURES {
        let Ok((s, _)) = import_gp(bytes) else {
            continue;
        };
        for v in VERSIONS {
            let r = round_trip(&s, v);
            // GP5 holds everything the importer keeps; the older versions lose only
            // what section 4 lists for them.
            let allowed: &[X] = match v {
                GpVersion::Gp5 => &[],
                GpVersion::Gp4 => &[X::VoiceDropped],
                GpVersion::Gp3 => &[
                    X::VoiceDropped,
                    X::AccentNotWritten,
                    X::PalmMuteNotWritten,
                    X::StrokeNotWritten,
                ],
            };
            for i in &r.issues {
                assert!(allowed.contains(&i.kind), "{name} {v:?}: {i}");
            }
        }
    }
}

/// G10: byte-identical output; the goldens are re-read in tests. Regenerate with
/// `GP_UPDATE_GOLDEN=1 cargo test -p mc-guitarpro --test export`.
#[test]
fn g10_output_is_deterministic_and_matches_the_goldens() {
    let small = reference(Shape {
        tracks: 1,
        bars: 4,
        beats: 8,
        notes_per_beat: 2,
    });
    let a = export_gp(&feature_rich(), GpVersion::Gp5).unwrap().0;
    assert_eq!(a, export_gp(&feature_rich(), GpVersion::Gp5).unwrap().0);
    let b = export_gp(&small, GpVersion::Gp5).unwrap().0;
    if std::env::var("GP_UPDATE_GOLDEN").is_ok() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
        std::fs::write(dir.join("feature_rich.gp5"), &a).unwrap();
        std::fs::write(dir.join("small.gp5"), &b).unwrap();
        return;
    }
    assert_eq!(a, include_bytes!("golden/feature_rich.gp5"));
    assert_eq!(b, include_bytes!("golden/small.gp5"));
    let (s, _) = import_gp(include_bytes!("golden/feature_rich.gp5")).unwrap();
    assert_eq!(describe(&s), describe(&import_gp(&a).unwrap().0));
}

#[test]
fn export_errors() {
    let mut s = Score::new(
        "Eight",
        InstrumentSpec::Fretted(FrettedInstrument {
            tuning: Tuning::new(
                ["E4", "B3", "G3", "D3", "A2", "E2", "B1", "F#1"]
                    .map(|p| p.parse().unwrap())
                    .to_vec(),
            )
            .unwrap(),
            fret_count: 24,
            capo: Capo(0),
        }),
        MasterBar::default(),
        Tempo::from_bpm(120).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        export_gp(&s, GpVersion::Gp5),
        Err(mc_guitarpro::GpExportError::TooManyStrings {
            track: 0,
            strings: 8
        })
    ));
    s.insert_master_bars(1, 10_000, MasterBar::default())
        .unwrap();
    assert!(export_gp(&s, GpVersion::Gp5).is_err());
}

// ------------------------------------------------------------ random scores (G7, G8)

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

/// Values the format holds, and the ones it must approximate (two dots, a 128th, a
/// duplet, 4:3).
fn values() -> Vec<NoteValue> {
    let mut v = Vec::new();
    for e in 0..=5u8 {
        let base = BaseValue::from_exponent(e).unwrap();
        v.push(NoteValue::plain(base));
        v.push(NoteValue::new(base, 1, None).unwrap());
        v.push(NoteValue::new(base, 0, Some(Tuplet::TRIPLET)).unwrap());
    }
    for (a, n) in [(5, 4), (6, 4), (7, 4), (9, 8)] {
        v.push(NoteValue::new(BaseValue::SIXTEENTH, 0, Some(Tuplet::new(a, n).unwrap())).unwrap());
    }
    v.push(NoteValue::new(BaseValue::EIGHTH, 2, None).unwrap());
    v.push(NoteValue::plain(BaseValue::from_exponent(7).unwrap()));
    v.push(NoteValue::new(BaseValue::EIGHTH, 0, Some(Tuplet::new(2, 3).unwrap())).unwrap());
    v.push(NoteValue::new(BaseValue::EIGHTH, 0, Some(Tuplet::new(4, 3).unwrap())).unwrap());
    v
}

/// A random but valid score, built through the aggregate operations (the validated
/// path). Rejected operations (an invalid repeat or tie) are skipped.
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
                ["E4", "B3", "G3", "D3", "A2", "D2", "A1"][..6 + rng.below(2) as usize]
                    .iter()
                    .map(|p| p.parse().unwrap())
                    .collect(),
            )
            .unwrap()
        };
        InstrumentSpec::Fretted(FrettedInstrument {
            tuning,
            fret_count: 20 + rng.below(5) as u8,
            capo: Capo(rng.below(4) as u8),
        })
    };
    let long = |rng: &mut Rng, base: &str| {
        if rng.chance(10) {
            format!("{base} {}", "ổ long name ".repeat(5))
        } else {
            base.to_string()
        }
    };
    let name = long(&mut rng, "T1");
    let mut s = Score::new(
        &name,
        instrument(&mut rng),
        first,
        Tempo::from_bpm(100).unwrap(),
    )
    .unwrap();
    let bars = 1 + rng.below(6) as usize;
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
            let plays = 2 + rng.below(15) as u8;
            mb.repeat_end = Some(plays);
            if rng.chance(50) {
                // Endings of this bar: some passes, some above 8 when the count allows.
                mb.alternate_endings = (1..=plays).filter(|_| rng.chance(30)).collect();
            }
        }
        if rng.chance(20) {
            mb.marker = Some(if rng.chance(10) {
                "M".repeat(300)
            } else {
                format!("M{m} <&>")
            });
        }
        let _ = s.set_master_bar(m, mb);
        // Up to two tempo changes in a bar, so two can fall on the same beat.
        for _ in 0..1 + usize::from(rng.chance(30)) {
            if rng.chance(40) {
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
    }
    for t in 1..1 + rng.below(3) {
        let name = long(&mut rng, &format!("T{}", t + 1));
        let _ = s.add_track(&name, instrument(&mut rng));
    }
    let values = values();
    let ids: Vec<_> = s.tracks().map(|t| t.id()).collect();
    for &track in &ids {
        let strings = s
            .tracks()
            .find(|t| t.id() == track)
            .unwrap()
            .fretted()
            .tuning
            .string_count() as u64;
        let mut last_of_voice: Vec<Option<Vec<FretPosition>>> = vec![None, None, None];
        for bar in 0..bars {
            let voices = 1 + usize::from(rng.chance(25)) + usize::from(rng.chance(5));
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
                    let copy_and_tie =
                        previous.as_ref().is_some_and(|p| !p.is_empty()) && rng.chance(40);
                    if copy_and_tie {
                        for &pos in previous.as_ref().unwrap() {
                            if let Ok((id, _)) = s.set_note(r, pos) {
                                let _ = s.set_tie(r, id, true);
                                if rng.chance(20) {
                                    let dead = NoteEffects {
                                        dead: true,
                                        ..NoteEffects::default()
                                    };
                                    let _ = s.set_note_effects(r, id, dead);
                                }
                                placed.push(pos);
                            }
                        }
                    } else if rng.chance(80) {
                        for _ in 0..rng.below(4) {
                            let pos = FretPosition {
                                string: StringIndex::new(1 + rng.below(strings) as u8).unwrap(),
                                fret: Fret(rng.below(16) as u8),
                            };
                            if let Ok((id, _)) = s.set_note(r, pos) {
                                placed.push(pos);
                                let effects = NoteEffects {
                                    let_ring: rng.chance(20),
                                    dead: rng.chance(10),
                                    accent: rng.chance(15),
                                    palm_mute: rng.chance(15),
                                    string_percussive: rng.chance(8),
                                };
                                let _ = s.set_note_effects(r, id, effects);
                            }
                        }
                    }
                    if rng.chance(20) {
                        let stroke = if rng.chance(50) {
                            Stroke::Down
                        } else {
                            Stroke::Up
                        };
                        s.set_beat_stroke(r, Some(stroke)).unwrap();
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

    /// G7, G8: 200 random valid scores round-trip every fact each version holds.
    #[test]
    fn g7_random_scores_round_trip(seed in any::<u64>()) {
        let s = random_score(seed);
        for v in VERSIONS {
            round_trip(&s, v);
        }
    }
}

/// G8: the random scores reach every counted approximation, so G7 is not vacuous.
#[test]
fn g8_random_scores_cover_every_counted_case() {
    let mut seen = BTreeSet::new();
    for seed in 0..200u64 {
        let s = random_score(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        for v in VERSIONS {
            for i in round_trip(&s, v).issues {
                seen.insert(i.kind);
            }
        }
    }
    let all = [
        X::DotsReduced,
        X::ValueTooShort,
        X::TupletApproximated,
        X::VoiceDropped,
        X::StringPercussiveNotWritten,
        X::DeadTiedNote,
        X::AccentNotWritten,
        X::PalmMuteNotWritten,
        X::StrokeNotWritten,
        X::TempoRounded,
        X::TempoMoved,
        X::TempoDropped,
        X::EndingDropped,
        X::PickupWritten,
        X::TextCut,
    ];
    let missing: Vec<_> = all.iter().filter(|k| !seen.contains(k)).collect();
    assert!(missing.is_empty(), "never produced: {missing:?}");
}

/// G11: release-mode timing on the reference machine. Run with
/// `cargo test -p mc-guitarpro --release --test export -- --ignored --nocapture`.
#[test]
#[ignore]
fn g11_timing() {
    use mc_score::fixtures::LARGE;
    for (name, score, limit_ms) in [
        ("TYPICAL", reference(TYPICAL), 100.0),
        ("LARGE", reference(LARGE), 1000.0),
    ] {
        let mut export_ms = Vec::new();
        let mut import_ms = Vec::new();
        let mut bytes = Vec::new();
        for _ in 0..11 {
            let t = std::time::Instant::now();
            bytes = export_gp(&score, GpVersion::Gp5).unwrap().0;
            export_ms.push(t.elapsed().as_secs_f64() * 1e3);
            let t = std::time::Instant::now();
            import_gp(&bytes).unwrap();
            import_ms.push(t.elapsed().as_secs_f64() * 1e3);
        }
        export_ms.sort_by(f64::total_cmp);
        import_ms.sort_by(f64::total_cmp);
        println!(
            "{name}: {} notes, {} bytes; export median {:.1} ms (max {:.1}); import median {:.1} ms (max {:.1}); limit {limit_ms} ms",
            score.note_count(),
            bytes.len(),
            export_ms[5],
            export_ms[10],
            import_ms[5],
            import_ms[10]
        );
        assert!(export_ms[5] <= limit_ms && import_ms[5] <= limit_ms);
    }
}

/// G9 inputs: our exports of the three reference scores in every version, and our
/// re-import of each written in the text form of the S10 black-box program
/// `ReadSummary` (TuxGuitar's view). The local procedure in
/// `benchmarks/tuxguitar-guitarpro.md` runs TuxGuitar on the files and compares.
/// Run with `GP_G9_DIR=<dir> cargo test -p mc-guitarpro --test export g9 -- --ignored`.
#[test]
#[ignore]
fn g9_write_tuxguitar_inputs() {
    let dir = std::path::PathBuf::from(std::env::var("GP_G9_DIR").expect("set GP_G9_DIR"));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, score) in [
        ("feature_rich", feature_rich()),
        ("techniques", techniques()),
        ("typical", reference(TYPICAL)),
    ] {
        for v in VERSIONS {
            let (bytes, _) = export_gp(&score, v).unwrap();
            let file = format!("{name}.{}", v.extension());
            std::fs::write(dir.join(&file), &bytes).unwrap();
            let (back, _) = import_gp(&bytes).unwrap();
            std::fs::write(dir.join(format!("{file}.ours.txt")), tuxguitar_view(&back)).unwrap();
        }
    }
}

/// A score in `ReadSummary`'s text form: endings and repeat closes, then per track the
/// track line and the voice lines (durations as 1, 2, 4 ...; "." dotted; "tN" tuplet).
fn tuxguitar_view(s: &Score) -> String {
    let mut out = Vec::new();
    for (m, b) in s.master_bars().iter().enumerate() {
        let alt: u32 = b.alternate_endings.iter().map(|p| 1u32 << (p - 1)).sum();
        let close = b.repeat_end.map_or(0, |p| p - 1);
        if alt != 0 || close != 0 {
            out.push(format!(
                "  extra bar {}: alternative {alt} close {close}",
                m + 1
            ));
        }
    }
    for (i, t) in s.tracks().enumerate() {
        let f = t.fretted();
        out.push(format!(
            "  track {} \"{}\" strings {} capo {}",
            i + 1,
            t.name(),
            f.tuning.string_count(),
            f.capo.0
        ));
        for (m, bar) in t.bars().enumerate() {
            for (vi, voice) in bar.voices.iter().enumerate() {
                if voice.beats.is_empty() {
                    continue;
                }
                let beats: Vec<String> = voice
                    .beats
                    .iter()
                    .map(|b| {
                        let mut x = (1u32 << b.value.base().exponent()).to_string();
                        if b.value.dots() > 0 {
                            x.push('.');
                        }
                        if let Some(t) = b.value.tuplet() {
                            x.push_str(&format!("t{}", t.actual()));
                        }
                        if vi == 0 {
                            match b.stroke {
                                Some(Stroke::Down) => x.push_str(" down"),
                                Some(Stroke::Up) => x.push_str(" up"),
                                None => {}
                            }
                        }
                        if b.notes.is_empty() {
                            x.push_str(" rest");
                        } else {
                            let mut notes: Vec<_> = b.notes.iter().collect();
                            notes.sort_by_key(|n| n.position().string);
                            let ns: Vec<String> = notes
                                .iter()
                                .map(|n| {
                                    let p = n.position();
                                    let e = n.effects;
                                    let mut s = format!("{}/{}", p.string.get(), p.fret.0);
                                    for (on, tag) in [
                                        (n.tie, "tie"),
                                        (e.dead, "dead"),
                                        (e.accent, "acc"),
                                        (e.palm_mute, "pm"),
                                        (e.let_ring, "ring"),
                                    ] {
                                        if on {
                                            s.push(' ');
                                            s.push_str(tag);
                                        }
                                    }
                                    s
                                })
                                .collect();
                            x.push_str(&format!(" ({})", ns.join(", ")));
                        }
                        x
                    })
                    .collect();
                out.push(format!(
                    "    bar {} voice {}: {}",
                    m + 1,
                    vi + 1,
                    beats.join(" | ")
                ));
            }
        }
    }
    out.join("\n") + "\n"
}
