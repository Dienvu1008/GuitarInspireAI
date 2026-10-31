//! Effective master bar duration and global bar start times (domain-invariants 4a,
//! PB-01 to PB-08) and D-01 diagnostics. PB-09 (timeline determinism) is a Phase 4
//! mc-playback test.

mod common;
use common::*;
use mc_score::{DiagnosticKind, MasterBar};
use mc_theory::Rational;
use proptest::prelude::*;

#[test]
fn pb01_full_voices_give_the_nominal_duration() {
    let mut s = score(2, 2);
    for i in 0..2 {
        let t = track(&s, i);
        fill(&mut s, t, 0, 0, &[q(), q(), q(), q()]);
    }
    assert_eq!(s.effective_bar_duration(0).unwrap(), r(1, 1));
    assert_eq!(s.bar_start_times().unwrap(), [(0, r(0, 1)), (1, r(1, 1))]);
    assert!(s.diagnostics(0..2).unwrap().is_empty());
}

#[test]
fn pb02_d01_overfull_voice_extends_the_bar_for_every_track() {
    let mut s = score(4, 2);
    let t1 = track(&s, 0);
    let t2 = track(&s, 1);
    fill(&mut s, t1, 2, 0, &[q(), q(), q(), q()]);
    fill(&mut s, t2, 2, 0, &[q(), q(), q(), q(), eighth()]);
    assert_eq!(s.effective_bar_duration(2).unwrap(), r(9, 8));
    let starts = s.bar_start_times().unwrap();
    // Bars 0 and 1 last 1 each, bar 2 lasts 9/8: bar 3 starts at 2 + 9/8 for every track.
    assert_eq!(starts[2], (2, r(2, 1)));
    assert_eq!(starts[3], (3, r(2, 1).checked_add(r(9, 8)).unwrap()));
    let d = s.diagnostics(2..3).unwrap();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "D-01");
    assert_eq!(d[0].track, t2);
    assert_eq!(
        d[0].kind,
        DiagnosticKind::Overfull {
            actual: r(9, 8),
            nominal: r(1, 1)
        }
    );
}

#[test]
fn pb03_d01_underfull_voice_does_not_shorten_the_bar() {
    let mut s = score(3, 2);
    let t1 = track(&s, 0);
    fill(&mut s, t1, 1, 0, &[q(), q()]);
    assert_eq!(s.effective_bar_duration(1).unwrap(), r(1, 1));
    assert_eq!(s.bar_start_times().unwrap()[2], (2, r(2, 1)));
    let d = s.diagnostics(0..3).unwrap();
    assert_eq!(d.len(), 1);
    assert!(matches!(d[0].kind, DiagnosticKind::Underfull { .. }));
    // The per-track variant applies the same rule to one track only.
    assert_eq!(s.track_diagnostics(t1, 0..3).unwrap(), d);
    assert!(s.track_diagnostics(track(&s, 1), 0..3).unwrap().is_empty());
}

#[test]
fn pb04_mute_is_not_part_of_the_score_so_muted_tracks_still_count() {
    // Mute lives in playback options (Phase 4); the score has no mute state at all.
    // E(m) therefore includes every track, which is what muting must preserve.
    let mut s = score(2, 3);
    let t3 = track(&s, 2);
    fill(&mut s, t3, 0, 0, &[half(), half(), q()]);
    assert_eq!(s.effective_bar_duration(0).unwrap(), r(5, 4));
    assert_eq!(s.bar_start_times().unwrap()[1], (1, r(5, 4)));
}

#[test]
fn pb05_longest_voice_within_one_track_counts() {
    let mut s = score(1, 1);
    let t = track(&s, 0);
    let (v1, _) = s.add_voice(t, 0).unwrap();
    fill(&mut s, t, 0, 0, &[q(), q(), q(), q()]);
    fill(&mut s, t, 0, v1, &[half(), half(), half()]);
    assert_eq!(s.effective_bar_duration(0).unwrap(), r(3, 2));
}

#[test]
fn pb06_empty_pickup_bar_is_skipped() {
    let mut s = score(3, 1);
    let pickup = MasterBar {
        pickup: true,
        ..MasterBar::default()
    };
    s.set_master_bar(0, pickup).unwrap();
    assert_eq!(s.effective_bar_duration(0).unwrap(), Rational::ZERO);
    assert_eq!(s.bar_start_times().unwrap(), [(1, r(0, 1)), (2, r(1, 1))]);
    // A pickup with content lasts as long as its content, and a short pickup is no diagnostic.
    let t = track(&s, 0);
    fill(&mut s, t, 0, 0, &[q()]);
    assert_eq!(s.effective_bar_duration(0).unwrap(), r(1, 4));
    assert_eq!(s.bar_start_times().unwrap()[1], (1, r(1, 4)));
    assert!(s.diagnostics(0..1).unwrap().is_empty());
}

#[test]
fn pb07_overfull_bar_in_a_repeat_uses_the_same_duration_each_pass() {
    let mut s = score(3, 2);
    s.set_master_bar(
        0,
        MasterBar {
            repeat_start: true,
            ..MasterBar::default()
        },
    )
    .unwrap();
    s.set_master_bar(
        1,
        MasterBar {
            repeat_end: Some(2),
            ..MasterBar::default()
        },
    )
    .unwrap();
    let t2 = track(&s, 1);
    fill(&mut s, t2, 1, 0, &[half(), half(), q()]);
    let starts = s.bar_start_times().unwrap();
    let bars: Vec<usize> = starts.iter().map(|x| x.0).collect();
    assert_eq!(bars, [0, 1, 0, 1, 2]);
    // pass 1: 0 at 0, 1 at 1 (lasts 5/4); pass 2: 0 at 9/4, 1 at 13/4; then 2 at 9/2.
    let times: Vec<Rational> = starts.iter().map(|x| x.1).collect();
    assert_eq!(times, [r(0, 1), r(1, 1), r(9, 4), r(13, 4), r(9, 2)]);
}

proptest! {
    /// PB-08: for random content, start times are non-decreasing, shared by all
    /// tracks (a single list), and every E(m) is at least the nominal duration.
    #[test]
    fn pb08_start_times_are_monotonic_and_effective_duration_dominates_nominal(
        content in proptest::collection::vec((0usize..3, 0usize..4, 0u8..6), 0..40)
    ) {
        let mut s = score(4, 3);
        let values = [eighth(), q(), half(), common::q()];
        for (track_i, bar, v) in content {
            let t = track(&s, track_i);
            fill(&mut s, t, bar, 0, &[values[v as usize % values.len()]]);
        }
        prop_assert!(s.validate().is_empty());
        let starts = s.bar_start_times().unwrap();
        for w in starts.windows(2) {
            prop_assert!(w[0].1 < w[1].1);
        }
        for m in 0..s.master_bar_count() {
            prop_assert!(s.effective_bar_duration(m).unwrap() >= s.nominal_bar_duration(m).unwrap());
        }
    }
}
