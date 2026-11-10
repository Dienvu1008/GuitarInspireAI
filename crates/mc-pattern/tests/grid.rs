//! The grid editor front end (ADR-034 section 9, PE-14 core part): snap and values,
//! every edit with its typed errors, spans following their steps, and the grid view.

mod common;
use common::*;
use mc_pattern::grid::{
    self, Cell, CellKind, Edit, GridError, Row, beat_length, cell_onset, single_value,
};
use mc_pattern::*;
use mc_theory::{BaseValue, NoteValue, Rational, TimeSignature, Tuplet};
use proptest::prelude::*;

use LaneId::{Fingers, Strum as StrumLane, Thumb};
use StringSelector::{Alt, Bass};

fn c(beat: u32, parts: u8, cell: u8) -> Cell {
    Cell { beat, parts, cell }
}

fn value(base: BaseValue, dots: u8, triplet: bool) -> NoteValue {
    NoteValue::new(base, dots, triplet.then_some(Tuplet::TRIPLET)).unwrap()
}

/// An empty strum pattern of one 4/4 bar.
fn empty() -> PatternParts {
    PatternParts {
        meter: Some(TimeSignature::COMMON),
        ..pattern(Rational::ONE, Lanes::Strum(Lane::default()))
    }
}

fn place(p: &PatternParts, lane: LaneId, at: Cell, cells: u8, a: Action) -> PatternParts {
    grid::edit(
        p,
        &Edit::Place {
            lane,
            at,
            cells,
            action: a,
        },
    )
    .unwrap()
}

fn onsets(p: &PatternParts, lane: LaneId) -> Vec<Rational> {
    p.lanes
        .get(lane)
        .unwrap()
        .steps
        .iter()
        .map(|s| s.onset)
        .collect()
}

/// Corpus pattern 2 (two lanes, gaps), from the text syntax.
fn travis() -> PatternParts {
    text::parse(
        "\"length\": 1\nmeter 4/4\nthumb: Bass:4 Alt:4 Bass:4 Alt:4\nfingers: 1:8 2:8 @3/8 1:8 @5/8 2:8 @7/8 1:8",
        &PatternLibrary::new(),
    )
    .unwrap()
}

// ------------------------------------------------------------ snap and values

#[test]
fn a_beat_is_one_unit_of_the_meter_or_a_quarter() {
    assert_eq!(beat_length(None), r(1, 4));
    assert_eq!(beat_length(Some(TimeSignature::COMMON)), r(1, 4));
    assert_eq!(
        beat_length(Some(TimeSignature::new(6, 8).unwrap())),
        r(1, 8)
    );
    assert_eq!(
        beat_length(Some(TimeSignature::new(2, 2).unwrap())),
        r(1, 2)
    );
}

#[test]
fn snaps_give_plain_dotted_and_triplet_values_and_never_round() {
    use BaseValue as B;
    let cases: [(u8, u8, Option<NoteValue>); 13] = [
        (1, 1, Some(value(B::QUARTER, 0, false))),
        (2, 1, Some(value(B::EIGHTH, 0, false))),
        (4, 1, Some(value(B::SIXTEENTH, 0, false))),
        (8, 1, Some(value(B::from_exponent(5).unwrap(), 0, false))),
        // 3 creates triplets (ADR-034 section 9), 6 triplet sixteenths.
        (3, 1, Some(value(B::EIGHTH, 0, true))),
        (3, 2, Some(value(B::QUARTER, 0, true))),
        (6, 1, Some(value(B::SIXTEENTH, 0, true))),
        (3, 3, Some(value(B::QUARTER, 0, false))),
        (4, 3, Some(value(B::EIGHTH, 1, false))),
        (2, 3, Some(value(B::QUARTER, 1, false))),
        (8, 7, Some(value(B::EIGHTH, 2, false))),
        // 5/16 and 5/12 are not one value.
        (4, 5, None),
        (3, 5, None),
    ];
    for (parts, cells, want) in cases {
        let got = grid::edit(
            &empty(),
            &Edit::Place {
                lane: StrumLane,
                at: c(0, parts, 0),
                cells,
                action: strike(&[Bass]),
            },
        );
        match want {
            Some(v) => assert_eq!(
                got.unwrap().lanes.get(StrumLane).unwrap().steps[0].duration,
                v,
                "{cells} of {parts}"
            ),
            None => assert!(
                matches!(got, Err(GridError::UnwritableLength { .. })),
                "{cells} of {parts}: {got:?}"
            ),
        }
    }
    assert_eq!(single_value(r(1, 4)), Some(q()));
    assert_eq!(single_value(r(5, 16)), None);
}

#[test]
fn cells_and_snaps_are_checked() {
    let m = Some(TimeSignature::COMMON);
    assert_eq!(cell_onset(m, c(2, 3, 1)).unwrap(), r(7, 12));
    for parts in [0, 5, 7, 12] {
        assert_eq!(
            cell_onset(m, c(0, parts, 0)),
            Err(GridError::BadSnap { parts })
        );
    }
    assert_eq!(
        cell_onset(m, c(0, 4, 4)),
        Err(GridError::BadCell { cell: c(0, 4, 4) })
    );
    assert!(matches!(
        grid::edit(
            &empty(),
            &Edit::Place {
                lane: StrumLane,
                at: c(0, 4, 0),
                cells: 0,
                action: strike(&[Bass]),
            }
        ),
        Err(GridError::BadCell { .. })
    ));
}

// ------------------------------------------------------------------ edits

#[test]
fn place_inserts_in_onset_order_and_widens_or_shifts_spans() {
    let mut p = empty();
    for (beat, t) in [(0, Bass), (2, Bass), (3, Alt)] {
        p = place(&p, StrumLane, c(beat, 1, 0), 1, strike(&[t]));
    }
    p.spans = vec![
        TechniqueSpan {
            lane: StrumLane,
            first: 0,
            last: 1,
            technique: Technique::PalmMute,
        },
        TechniqueSpan {
            lane: StrumLane,
            first: 2,
            last: 2,
            technique: Technique::Accent,
        },
    ];
    // Into the gap between steps 0 and 1: inside the first span, before the second.
    let p = place(&p, StrumLane, c(1, 2, 0), 1, strike(&[Alt]));
    assert_eq!(onsets(&p, StrumLane), [r(0, 1), r(1, 4), r(1, 2), r(3, 4)]);
    assert_eq!((p.spans[0].first, p.spans[0].last), (0, 2), "widened");
    assert_eq!((p.spans[1].first, p.spans[1].last), (3, 3), "shifted");
    assert!(PatternLibrary::new().validate_pattern(&p).is_empty());
    // Placing over an existing step is refused by validation (PT-03), not hidden.
    let over = place(&p, StrumLane, c(0, 2, 1), 1, strike(&[Bass]));
    assert!(
        PatternLibrary::new()
            .validate_pattern(&over)
            .iter()
            .any(|e| matches!(e, PatternError::Overlap { .. }))
    );
}

#[test]
fn move_keeps_the_value_and_never_passes_a_neighbour() {
    let p = travis();
    let moved = grid::edit(
        &p,
        &Edit::Move {
            lane: Fingers,
            index: 2,
            to: c(1, 2, 0),
        },
    )
    .unwrap();
    let f = &moved.lanes.get(Fingers).unwrap().steps;
    assert_eq!(f[2].onset, r(1, 4));
    assert_eq!(f[2].duration, e());
    for to in [c(0, 2, 1), c(2, 2, 1), c(0, 1, 0)] {
        assert_eq!(
            grid::edit(
                &p,
                &Edit::Move {
                    lane: Fingers,
                    index: 2,
                    to
                }
            ),
            Err(GridError::WouldReorder {
                lane: Fingers,
                index: 2
            }),
            "{to:?}"
        );
    }
    assert_eq!(
        grid::edit(
            &p,
            &Edit::Move {
                lane: Fingers,
                index: 9,
                to: c(0, 2, 0)
            }
        ),
        Err(GridError::NoSuchStep {
            lane: Fingers,
            index: 9
        })
    );
}

#[test]
fn resize_sets_one_written_value() {
    let p = grid::edit(
        &travis(),
        &Edit::Resize {
            lane: Thumb,
            index: 0,
            parts: 4,
            cells: 3,
        },
    )
    .unwrap();
    assert_eq!(
        p.lanes.get(Thumb).unwrap().steps[0].duration,
        value(BaseValue::EIGHTH, 1, false)
    );
    assert!(matches!(
        grid::edit(
            &travis(),
            &Edit::Resize {
                lane: Thumb,
                index: 0,
                parts: 4,
                cells: 5,
            }
        ),
        Err(GridError::UnwritableLength { .. })
    ));
}

#[test]
fn delete_narrows_removes_and_shifts_spans() {
    let mut p = empty();
    for k in 0..8 {
        p = place(
            &p,
            StrumLane,
            c(k / 2, 2, (k % 2) as u8),
            1,
            strike(&[Bass]),
        );
    }
    let span = |first, last, technique| TechniqueSpan {
        lane: StrumLane,
        first,
        last,
        technique,
    };
    p.spans = vec![
        span(0, 1, Technique::Accent),           // before the run: kept
        span(1, 3, Technique::PalmMute),         // ends inside: narrowed to 1..=1
        span(2, 4, Technique::Dead),             // inside: removed
        span(4, 6, Technique::LetRing),          // starts inside: 2..=3 after the shift
        span(1, 6, Technique::StringPercussive), // over it: 1..=3
        span(6, 7, Technique::Accent),           // after: shifted to 3..=4
    ];
    let d = grid::edit(
        &p,
        &Edit::Delete {
            lane: StrumLane,
            first: 2,
            last: 4,
        },
    )
    .unwrap();
    assert_eq!(d.lanes.get(StrumLane).unwrap().steps.len(), 5);
    let got: Vec<_> = d
        .spans
        .iter()
        .map(|s| (s.first, s.last, s.technique))
        .collect();
    assert_eq!(
        got,
        [
            (0, 1, Technique::Accent),
            (1, 1, Technique::PalmMute),
            (2, 3, Technique::LetRing),
            (1, 3, Technique::StringPercussive),
            (3, 4, Technique::Accent),
        ]
    );
    assert!(PatternLibrary::new().validate_pattern(&d).is_empty());
    for (first, last) in [(3, 2), (0, 8)] {
        assert!(
            grid::edit(
                &p,
                &Edit::Delete {
                    lane: StrumLane,
                    first,
                    last
                }
            )
            .is_err()
        );
    }
}

#[test]
fn targets_directions_and_techniques() {
    let p = travis();
    let toggled = grid::edit(
        &p,
        &Edit::ToggleTarget {
            lane: Thumb,
            index: 0,
            selector: StringSelector::TREBLE,
        },
    )
    .unwrap();
    let Action::Strike(k) = &toggled.lanes.get(Thumb).unwrap().steps[0].action else {
        panic!()
    };
    assert_eq!(k.targets, [Bass, StringSelector::TREBLE]);
    let back = grid::edit(
        &toggled,
        &Edit::ToggleTarget {
            lane: Thumb,
            index: 0,
            selector: StringSelector::TREBLE,
        },
    )
    .unwrap();
    assert_eq!(back, p);
    assert_eq!(
        grid::edit(
            &p,
            &Edit::ToggleTarget {
                lane: Thumb,
                index: 0,
                selector: Bass
            }
        ),
        Err(GridError::LastTarget {
            lane: Thumb,
            index: 0
        })
    );

    // A direction over a run with a rest: the strikes get it, the rest does not.
    let mut r = place(&empty(), StrumLane, c(0, 1, 0), 1, strike(&[Bass]));
    r = place(&r, StrumLane, c(1, 1, 0), 1, Action::Rest);
    r = place(&r, StrumLane, c(2, 1, 0), 1, strike(&[Alt]));
    let d = grid::edit(
        &r,
        &Edit::SetDirection {
            lane: StrumLane,
            first: 0,
            last: 2,
            direction: Some(Direction::Up),
        },
    )
    .unwrap();
    let steps = &d.lanes.get(StrumLane).unwrap().steps;
    assert!(matches!(&steps[0].action, Action::Strike(k) if k.direction == Some(Direction::Up)));
    assert_eq!(steps[1].action, Action::Rest);
    assert_eq!(
        grid::edit(
            &r,
            &Edit::SetDirection {
                lane: StrumLane,
                first: 1,
                last: 1,
                direction: None
            }
        ),
        Err(GridError::NoStrikeInRun {
            lane: StrumLane,
            first: 1,
            last: 1
        })
    );

    for t in [
        Technique::Accent,
        Technique::PalmMute,
        Technique::Dead,
        Technique::LetRing,
        Technique::StringPercussive,
    ] {
        let on = grid::edit(
            &r,
            &Edit::SetTechnique {
                lane: StrumLane,
                index: 2,
                technique: t,
                on: true,
            },
        )
        .unwrap();
        let Action::Strike(k) = &on.lanes.get(StrumLane).unwrap().steps[2].action else {
            panic!()
        };
        let set = k.techniques;
        let count = [
            set.accent,
            set.palm_mute,
            set.dead,
            set.let_ring,
            set.string_percussive,
        ]
        .iter()
        .filter(|x| **x)
        .count();
        assert_eq!(count, 1, "{t:?}");
        let off = grid::edit(
            &on,
            &Edit::SetTechnique {
                lane: StrumLane,
                index: 2,
                technique: t,
                on: false,
            },
        )
        .unwrap();
        assert_eq!(off, r, "{t:?}");
    }
    assert_eq!(
        grid::edit(
            &r,
            &Edit::SetTechnique {
                lane: StrumLane,
                index: 1,
                technique: Technique::Accent,
                on: true
            }
        ),
        Err(GridError::NotAStrike {
            lane: StrumLane,
            index: 1
        })
    );
}

#[test]
fn spans_lanes_and_pattern_settings() {
    let p = travis();
    let s = grid::edit(
        &p,
        &Edit::AddSpan {
            lane: Fingers,
            first: 1,
            last: 3,
            technique: Technique::LetRing,
        },
    )
    .unwrap();
    assert_eq!(s.spans.len(), 1);
    assert_eq!(grid::edit(&s, &Edit::RemoveSpan { index: 0 }).unwrap(), p);
    assert_eq!(
        grid::edit(&s, &Edit::RemoveSpan { index: 1 }),
        Err(GridError::NoSuchSpan { index: 1 })
    );
    assert_eq!(
        grid::edit(
            &p,
            &Edit::AddSpan {
                lane: StrumLane,
                first: 0,
                last: 0,
                technique: Technique::Accent
            }
        ),
        Err(GridError::NoSuchLane { lane: StrumLane })
    );
    let named = grid::edit(&p, &Edit::SetName { name: "T2".into() }).unwrap();
    assert_eq!(named.name, "T2");
    let longer = grid::edit(&p, &Edit::SetLength { length: r(2, 1) }).unwrap();
    assert_eq!(longer.length, r(2, 1));
    let free = grid::edit(&p, &Edit::SetMeter { meter: None }).unwrap();
    assert_eq!(free.meter, None);
    let meta = PatternMeta {
        description: "d".into(),
        tags: vec!["t".into()],
        origin: Some(Origin::Grid),
    };
    assert_eq!(
        grid::edit(&p, &Edit::SetMeta { meta: meta.clone() })
            .unwrap()
            .meta,
        meta
    );
}

// --------------------------------------------------------------- view model

#[test]
fn the_view_has_rows_cells_spans_and_the_snap_per_beat() {
    let (lib, g) = gesture_library();
    let p = text::parse(
        "\"length\": 1\nmeter 4/4\nstrum: Bùm:4 _(Bass):8 Treble(4)+2:8t @1/2 -:8 Strum(6..4):8 _:4\nstrum pm#1..3",
        &lib,
    )
    .unwrap();
    let v = grid::view(&p, &lib, &[2, 3]).unwrap();
    assert_eq!(v.snaps, [2, 3, 3, 3], "the last snap continues");
    assert_eq!(v.beat_length, r(1, 4));
    let labels: Vec<&str> = v.rows.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Bass",
            "Alt",
            "Treble",
            "Treble(4)",
            "Strum(6..4)",
            "String 1",
            "String 2",
            "String 3",
            "String 4",
            "String 5",
            "String 6",
            "Strum",
            "Bùm",
            "Kịch",
            "XL"
        ]
    );
    let row = |label: &str| labels.iter().position(|l| *l == label).unwrap();
    assert_eq!(v.lanes, [StrumLane]);
    assert_eq!(v.steps.len(), 6);
    let s = &v.steps;
    assert_eq!(s[0].kind, CellKind::Gesture(g[0].0));
    assert_eq!(s[0].rows, [row("Bùm")]);
    assert!(
        s[0].techniques.palm_mute && s[0].techniques.accent,
        "the gesture's flags"
    );
    assert_eq!(s[1].kind, CellKind::Hold);
    assert_eq!(s[1].rows, [row("Bass")]);
    assert_eq!(s[1].cell, Some(c(1, 3, 0)));
    assert!(s[1].span_techniques.palm_mute);
    assert!(
        !s[0].span_techniques.palm_mute && !s[4].span_techniques.palm_mute,
        "only the run"
    );
    assert_eq!(s[2].rows, [row("Treble(4)"), row("String 2")]);
    // 3/8 is on the grid of beat 2 with 2 cells, but beat 2 has 3 cells here.
    assert_eq!(s[2].onset, r(3, 8));
    assert_eq!(s[2].cell, None, "off the grid of its beat");
    assert_eq!(s[3].kind, CellKind::Rest);
    assert!(s[3].rows.is_empty());
    assert_eq!(s[5].kind, CellKind::Hold);
    assert!(s[5].rows.is_empty(), "a full hold marks no row");
    assert_eq!(v.spans.len(), 1);
    assert_eq!((v.spans[0].start, v.spans[0].end), (r(1, 4), r(5, 8)));
    assert_eq!(
        grid::view(&p, &lib, &[5]),
        Err(GridError::BadSnap { parts: 5 })
    );
}

#[test]
fn the_view_of_a_two_lane_pattern_marks_each_lane() {
    let v = grid::view(&travis(), &PatternLibrary::new(), &[]).unwrap();
    assert_eq!(v.snaps, [2, 2, 2, 2], "two cells per beat by default");
    assert_eq!(v.lanes, [Thumb, Fingers]);
    let thumb: Vec<_> = v.steps.iter().filter(|s| s.lane == Thumb).collect();
    let fingers: Vec<_> = v.steps.iter().filter(|s| s.lane == Fingers).collect();
    assert_eq!((thumb.len(), fingers.len()), (4, 5));
    assert_eq!(fingers[2].cell, Some(c(1, 2, 1)));
    assert_eq!(v.rows[thumb[1].rows[0]].0, Row::Selector(Alt));
}

// --------------------------------------------------------------- properties

fn lane_ids(p: &PatternParts) -> Vec<LaneId> {
    p.lanes.iter().map(|(id, _)| id).collect()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 300, .. ProptestConfig::default() })]

    /// Deleting any run of a valid pattern gives a valid pattern: spans always follow
    /// their steps and never point past the lane.
    #[test]
    fn deleting_any_run_keeps_the_pattern_valid(seed in pattern_seeds(), a in 0usize..16, b in 0usize..16, l in 0usize..2) {
        let (lib, g) = gesture_library();
        let p = build_pattern_with(&values(), &seed, &g);
        let lane = lane_ids(&p)[l % lane_ids(&p).len()];
        let n = p.lanes.get(lane).unwrap().steps.len();
        prop_assume!(n > 0);
        let (first, last) = ((a % n).min(b % n), (a % n).max(b % n));
        let d = grid::edit(&p, &Edit::Delete { lane, first, last }).unwrap();
        // Holds may lose the strike they continue (PT-05); spans must stay in range.
        let errors = lib.validate_pattern(&d);
        prop_assert!(
            errors.iter().all(|e| matches!(e, PatternError::HoldWithoutStrike { .. } | PatternError::HoldNotInStrike { .. })),
            "{errors:?}"
        );
        prop_assert_eq!(d.lanes.get(lane).unwrap().steps.len(), n - (last - first + 1));
    }

    /// Placing a rest in any free cell gives a valid pattern, with the spans still over
    /// the same steps.
    #[test]
    fn placing_into_a_free_cell_keeps_the_pattern_valid(seed in pattern_seeds(), beat in 0u32..8, parts in prop::sample::select(grid::SNAPS.to_vec()), cell in 0u8..8, l in 0usize..2) {
        let (lib, g) = gesture_library();
        let p = build_pattern_with(&values(), &seed, &g);
        let lane = lane_ids(&p)[l % lane_ids(&p).len()];
        let at = Cell { beat, parts, cell: cell % parts };
        let Ok(next) = grid::edit(&p, &Edit::Place { lane, at, cells: 1, action: Action::Rest }) else {
            return Ok(());
        };
        let errors = lib.validate_pattern(&next);
        let fits = errors.is_empty();
        let clash = errors.iter().all(|e| matches!(e,
            PatternError::Overlap { .. } | PatternError::OutsidePattern { .. }
            | PatternError::HoldWithoutStrike { .. } | PatternError::HoldNotInStrike { .. }));
        prop_assert!(fits || clash, "{errors:?}");
        // Each old step keeps its span membership.
        let onset = grid::cell_onset(p.meter, at).unwrap();
        let old = &p.lanes.get(lane).unwrap().steps;
        let pos = old.partition_point(|s| s.onset <= onset);
        for (i, sp) in p.spans.iter().enumerate().filter(|(_, s)| s.lane == lane) {
            let new = next.spans[i];
            let map = |k: usize| if k >= pos { k + 1 } else { k };
            prop_assert_eq!(new.first, map(sp.first));
            prop_assert_eq!(new.last, map(sp.last));
        }
    }

    /// The view never fails on a valid pattern, lists every step once, and marks only
    /// rows that exist.
    #[test]
    fn every_valid_pattern_has_a_view(seed in pattern_seeds(), snaps in proptest::collection::vec(prop::sample::select(grid::SNAPS.to_vec()), 0..6)) {
        let (lib, g) = gesture_library();
        let p = build_pattern_with(&ext_values(), &seed, &g);
        let v = grid::view(&p, &lib, &snaps).unwrap();
        let total: usize = p.lanes.iter().map(|(_, l)| l.steps.len()).sum();
        prop_assert_eq!(v.steps.len(), total);
        for s in &v.steps {
            prop_assert!(s.rows.iter().all(|r| *r < v.rows.len()));
            if let Some(cell) = s.cell {
                prop_assert_eq!(grid::cell_onset(p.meter, cell).unwrap(), s.onset);
            }
        }
        prop_assert_eq!(v.spans.len(), p.spans.len());
    }
}
