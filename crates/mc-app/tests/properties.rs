//! FF-13: hard invariants hold after every command of random sequences, and rejected
//! commands change nothing. FF-14: undo and redo restore identical states.

mod common;
use common::*;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    #[test]
    fn ff13_hard_invariants_hold_after_every_command(ops in proptest::collection::vec(op(), 1..60)) {
        let mut s = session(4);
        let mut accepted = 0;
        for (i, o) in ops.iter().enumerate() {
            let command = to_command(&s, o);
            let before = s.snapshot();
            let (rev, depth) = (s.revision(), s.undo_depth());
            match s.execute(cmd(i as u64, command)) {
                Ok(out) => {
                    accepted += 1;
                    prop_assert_eq!(out.revision.0, rev.0 + 1);
                    let v = s.score().validate();
                    prop_assert!(v.is_empty(), "after {:?}: {:?}", o, v);
                }
                Err(_) => {
                    // Rejected: score, revision and history are unchanged.
                    prop_assert_eq!(s.score(), &before);
                    prop_assert_eq!(s.revision(), rev);
                    prop_assert_eq!(s.undo_depth(), depth);
                }
            }
            // Timing stays computable for every committed state (D-01 never blocks).
            prop_assert!(s.score().bar_start_times().is_ok());
        }
        let _ = accepted;
    }

    #[test]
    fn ff14_undo_all_returns_to_the_initial_state(ops in proptest::collection::vec(op(), 1..40)) {
        let mut s = session(4);
        let initial = s.snapshot();
        let mut states = vec![initial.clone()];
        for (i, o) in ops.iter().enumerate() {
            let command = to_command(&s, o);
            if s.execute(cmd(i as u64, command)).is_ok() {
                states.push(s.snapshot());
            }
        }
        // Undo walks back through every recorded state, in order.
        for expected in states.iter().rev().skip(1) {
            s.undo().unwrap();
            prop_assert_eq!(s.score(), expected);
        }
        prop_assert!(s.undo().is_err());
        prop_assert_eq!(s.score(), &initial);
        // Redo walks forward again to the final state.
        for expected in states.iter().skip(1) {
            s.redo().unwrap();
            prop_assert_eq!(s.score(), expected);
        }
        prop_assert!(s.redo().is_err());
    }

    #[test]
    fn ff14_undo_n_then_redo_n_is_identity(ops in proptest::collection::vec(op(), 1..40), n in 0usize..10) {
        let mut s = session(4);
        for (i, o) in ops.iter().enumerate() {
            let command = to_command(&s, o);
            let _ = s.execute(cmd(i as u64, command));
        }
        let end = s.snapshot();
        let k = n.min(s.undo_depth());
        for _ in 0..k { s.undo().unwrap(); }
        for _ in 0..k { s.redo().unwrap(); }
        prop_assert_eq!(s.score(), &end);
        prop_assert!(s.score().validate().is_empty());
    }

    /// Events: seq is gapless from 1 over commands, undo and redo; the revision in each
    /// envelope is the revision after the change; command events carry the command id.
    #[test]
    fn event_sequence_is_gapless_and_correlated(ops in proptest::collection::vec(op(), 1..30)) {
        let mut s = session(3);
        let mut all = Vec::new();
        for (i, o) in ops.iter().enumerate() {
            let command = to_command(&s, o);
            if let Ok(out) = s.execute(cmd(1000 + i as u64, command)) {
                prop_assert!(out.events.iter().all(|e| e.correlation_id == Some(mc_app::CommandId(1000 + i as u64))));
                prop_assert!(out.events.iter().all(|e| e.revision == out.revision));
                all.extend(out.events);
            }
            if i % 7 == 6 && let Ok(out) = s.undo() {
                all.extend(out.events);
            }
        }
        for (k, e) in all.iter().enumerate() {
            prop_assert_eq!(e.seq, k as u64 + 1);
        }
    }
}
