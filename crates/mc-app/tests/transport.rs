//! Transport (slice 4d): the session drives an [`AudioPort`] with compiled timelines
//! and simple commands. These tests use a fake port that records calls, so they check
//! the application's decisions (compile, mute, swap on edit, start bar, events) without
//! any audio hardware.

mod common;
use common::*;
use mc_app::playback::{Channel, Pan, PerformanceTimeline, TempoFactor, Tick, Volume};
use mc_app::types::TrackId;
use mc_app::{
    AppEvent, AudioPort, Command, DocumentSession, EventPayload, PlaybackState, ScoreCommand,
    TrackCommand, TransportCommand, TransportError,
};

/// Records every call so the test can assert what the session decided.
#[derive(Default)]
struct FakePort {
    timeline: Option<PerformanceTimeline>,
    loads: u32,
    last_start: Option<Tick>,
    last_factor: Option<TempoFactor>,
    seeks: Vec<Tick>,
    state: PlaybackState,
    position: i64,
    error: Option<String>,
    /// Every `set_channel_mix` call, in order (ADR-032).
    mix_calls: Vec<(Channel, Volume, Pan)>,
    /// If set, the next `load_timeline` fails with this message.
    fail_load: Option<String>,
}

impl AudioPort for FakePort {
    fn load_timeline(&mut self, timeline: PerformanceTimeline) -> Result<(), TransportError> {
        if let Some(m) = self.fail_load.take() {
            return Err(TransportError::Audio(m));
        }
        self.timeline = Some(timeline);
        self.loads += 1;
        Ok(())
    }
    fn play(&mut self, start: Tick) -> Result<(), TransportError> {
        self.last_start = Some(start);
        self.state = PlaybackState::Playing;
        self.position = start.0;
        Ok(())
    }
    fn stop(&mut self) -> Result<(), TransportError> {
        self.state = PlaybackState::Stopped;
        Ok(())
    }
    fn seek(&mut self, tick: Tick) -> Result<(), TransportError> {
        self.seeks.push(tick);
        self.position = tick.0;
        Ok(())
    }
    fn set_tempo_factor(&mut self, factor: TempoFactor) -> Result<(), TransportError> {
        self.last_factor = Some(factor);
        Ok(())
    }
    fn set_channel_mix(
        &mut self,
        channel: Channel,
        volume: Volume,
        pan: Pan,
    ) -> Result<(), TransportError> {
        self.mix_calls.push((channel, volume, pan));
        Ok(())
    }
    fn state(&self) -> PlaybackState {
        self.state
    }
    fn position(&self) -> Tick {
        Tick(self.position)
    }
    fn last_error(&self) -> Option<String> {
        self.error.clone()
    }
}

/// A session with `tracks` tracks and `bars` bars, each track carrying one fretted note
/// so the compiled timeline has audible events.
fn playable(tracks: usize, bars: usize) -> DocumentSession {
    let mut s = session(bars);
    for i in 1..tracks {
        s.execute(cmd(
            1000 + i as u64,
            Command::Track(TrackCommand::Add {
                name: format!("Track {}", i + 1),
                instrument: mc_app::InstrumentSpecInput::standard_guitar(),
            }),
        ))
        .unwrap();
    }
    let ids: Vec<TrackId> = s.score().tracks().map(|t| t.id()).collect();
    for t in ids {
        let b = add_beat(&mut s, t, 0);
        s.execute(cmd(
            2000,
            Command::Score(ScoreCommand::SetNote {
                beat: b,
                string: 1,
                fret: 0,
            }),
        ))
        .unwrap();
    }
    s
}

fn app_events(outcome: &mc_app::CommandOutcome) -> Vec<AppEvent> {
    outcome
        .events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::App(a) => Some(a.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn play_compiles_loads_sets_factor_and_starts_at_the_cursor_bar() {
    let mut s = playable(2, 4);
    let mut port = FakePort::default();
    let out = s
        .transport(
            TransportCommand::Play {
                from_master_bar: Some(2),
            },
            &mut port,
        )
        .unwrap();

    assert_eq!(port.loads, 1, "a fresh timeline is installed");
    assert!(port.timeline.is_some());
    assert_eq!(port.last_factor, Some(TempoFactor::NORMAL));
    let expected = port
        .timeline
        .as_ref()
        .unwrap()
        .first_played_start_of_master_bar(2)
        .unwrap();
    assert_eq!(port.last_start, Some(expected), "starts at the cursor bar");
    assert_eq!(s.transport_state().state(), PlaybackState::Playing);
    assert!(app_events(&out).contains(&AppEvent::PlaybackStateChanged {
        state: PlaybackState::Playing
    }));
    // Session state never bumps the document revision.
    assert_eq!(out.revision, s.revision());
}

#[test]
fn play_from_a_bar_past_the_end_is_rejected_without_starting() {
    let mut s = playable(1, 2);
    let mut port = FakePort::default();
    let err = s
        .transport(
            TransportCommand::Play {
                from_master_bar: Some(99),
            },
            &mut port,
        )
        .unwrap_err();
    assert_eq!(err, TransportError::BarOutOfRange { master_bar: 99 });
    assert_eq!(
        port.loads, 0,
        "nothing was installed after the range check failed"
    );
    assert!(port.last_start.is_none(), "playback did not start");
    assert_eq!(s.transport_state().state(), PlaybackState::Stopped);
}

#[test]
fn muting_while_playing_recompiles_and_swaps_but_does_not_restart() {
    let mut s = playable(2, 2);
    let mut port = FakePort::default();
    s.transport(
        TransportCommand::Play {
            from_master_bar: Some(0),
        },
        &mut port,
    )
    .unwrap();
    let loads_after_play = port.loads;
    let start_after_play = port.last_start;

    let track = s.score().track_at(1).unwrap().id();
    let out = s
        .transport(TransportCommand::SetMuted { track, muted: true }, &mut port)
        .unwrap();

    assert!(s.transport_state().is_muted(track));
    assert_eq!(port.loads, loads_after_play + 1, "swapped a new timeline");
    assert_eq!(port.last_start, start_after_play, "no restart");
    assert!(app_events(&out).contains(&AppEvent::MuteChanged { track, muted: true }));
}

#[test]
fn muting_while_stopped_does_not_touch_the_port() {
    let mut s = playable(2, 2);
    let mut port = FakePort::default();
    let track = s.score().track_at(1).unwrap().id();
    let out = s
        .transport(TransportCommand::SetMuted { track, muted: true }, &mut port)
        .unwrap();
    assert!(s.transport_state().is_muted(track));
    assert_eq!(port.loads, 0, "no compile while stopped");
    assert!(app_events(&out).contains(&AppEvent::MuteChanged { track, muted: true }));
}

#[test]
fn a_muted_track_is_omitted_from_the_compiled_timeline() {
    let mut s = playable(2, 2);
    let mut unmuted = FakePort::default();
    s.transport(
        TransportCommand::Play {
            from_master_bar: Some(0),
        },
        &mut unmuted,
    )
    .unwrap();
    let full = unmuted.timeline.clone().unwrap();

    let track = s.score().track_at(1).unwrap().id();
    let mut port = FakePort::default();
    s.transport(TransportCommand::SetMuted { track, muted: true }, &mut port)
        .unwrap();
    s.transport(
        TransportCommand::Play {
            from_master_bar: Some(0),
        },
        &mut port,
    )
    .unwrap();
    let muted = port.timeline.unwrap();

    // Mute omits the track's note events but keeps timing and the channel map (PB-04),
    // so the timelines differ in their events yet both still map two tracks.
    assert_ne!(
        full.canonical_text(),
        muted.canonical_text(),
        "muting removes that track's note events"
    );
    assert!(
        muted.canonical_text().len() < full.canonical_text().len(),
        "the muted timeline has fewer events"
    );
    assert_eq!(
        muted.channels().len(),
        2,
        "channel assignments are per track, not per mute"
    );
}

#[test]
fn tempo_factor_sends_a_scalar_and_never_recompiles() {
    let mut s = playable(1, 2);
    let mut port = FakePort::default();
    s.transport(
        TransportCommand::Play {
            from_master_bar: Some(0),
        },
        &mut port,
    )
    .unwrap();
    let loads = port.loads;
    let factor = TempoFactor::new(150).unwrap();
    let out = s
        .transport(TransportCommand::SetTempoFactor { factor }, &mut port)
        .unwrap();
    assert_eq!(port.loads, loads, "tempo factor does not recompile");
    assert_eq!(port.last_factor, Some(factor));
    assert_eq!(s.transport_state().tempo_factor(), factor);
    assert!(app_events(&out).contains(&AppEvent::TempoFactorChanged { hundredths: 150 }));
}

#[test]
fn resume_without_a_cursor_bar_starts_at_the_current_position() {
    let mut s = playable(1, 4);
    let mut port = FakePort {
        position: 123,
        ..FakePort::default()
    };
    s.transport(
        TransportCommand::Play {
            from_master_bar: None,
        },
        &mut port,
    )
    .unwrap();
    assert_eq!(port.last_start, Some(Tick(123)));
}

#[test]
fn seek_passes_the_tick_through() {
    let mut s = playable(1, 2);
    let mut port = FakePort::default();
    s.transport(TransportCommand::Seek { tick: Tick(480) }, &mut port)
        .unwrap();
    assert_eq!(port.seeks, vec![Tick(480)]);
}

#[test]
fn a_failed_load_surfaces_as_a_transport_error() {
    let mut s = playable(1, 2);
    let mut port = FakePort {
        fail_load: Some("device gone".into()),
        ..FakePort::default()
    };
    let err = s
        .transport(
            TransportCommand::Play {
                from_master_bar: Some(0),
            },
            &mut port,
        )
        .unwrap_err();
    assert_eq!(err, TransportError::Audio("device gone".into()));
    assert_eq!(s.transport_state().state(), PlaybackState::Stopped);
}

#[test]
fn removing_a_muted_track_drops_it_from_compilation() {
    let mut s = playable(2, 2);
    let track = s.score().track_at(1).unwrap().id();
    let mut port = FakePort::default();
    s.transport(TransportCommand::SetMuted { track, muted: true }, &mut port)
        .unwrap();
    // Remove the muted track through a normal edit.
    s.execute(cmd(3000, Command::Track(TrackCommand::Remove { track })))
        .unwrap();
    // Playback compiles without panicking even though the muted set names a gone track.
    s.transport(
        TransportCommand::Play {
            from_master_bar: Some(0),
        },
        &mut port,
    )
    .unwrap();
    assert_eq!(port.timeline.unwrap().channels().len(), 1);
}

// ---- ADR-032: solo, volume and pan (X6-X10) ----

mod mixer {
    use super::*;
    use mc_app::playback::TrackMix;
    use mc_app::types::AppInfo;
    use mc_app::{DocumentId, SessionConfig, effective_muted};
    use mc_playback::assign_channels;
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    fn app() -> AppInfo {
        AppInfo {
            app: "GuitarInspireAI".into(),
            version: "0.1.0".into(),
        }
    }

    fn vol(v: u8) -> Volume {
        Volume::try_new(v).unwrap()
    }
    fn pan(v: i16) -> Pan {
        Pan::try_new(v).unwrap()
    }

    fn track(s: &DocumentSession, index: usize) -> TrackId {
        s.score().track_at(index).unwrap().id()
    }

    fn play(s: &mut DocumentSession, port: &mut FakePort) {
        s.transport(
            TransportCommand::Play {
                from_master_bar: Some(0),
            },
            port,
        )
        .unwrap();
    }

    fn set_mix(s: &mut DocumentSession, port: &mut FakePort, t: TrackId, v: u8, p: i16) {
        s.transport(
            TransportCommand::SetTrackMix {
                track: t,
                volume: vol(v),
                pan: pan(p),
            },
            port,
        )
        .unwrap();
    }

    fn timeline_text(s: &mut DocumentSession) -> String {
        let mut port = FakePort::default();
        play(s, &mut port);
        port.timeline.unwrap().canonical_text()
    }

    #[test]
    fn x6_after_removing_a_track_every_channel_gets_its_own_mix() {
        let mut s = playable(3, 2);
        let (t0, t1, t2) = (track(&s, 0), track(&s, 1), track(&s, 2));
        let mut port = FakePort::default();
        play(&mut s, &mut port);
        set_mix(&mut s, &mut port, t0, 20, -30);
        set_mix(&mut s, &mut port, t2, 127, 40);
        // Remove the middle track while playing: the third track moves to channel 1.
        s.execute(cmd(
            3000,
            Command::Track(TrackCommand::Remove { track: t1 }),
        ))
        .unwrap();
        port.mix_calls.clear();
        s.refresh_playback_after_edit(&mut port).unwrap();

        let assignments = assign_channels(s.score());
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[1].track, t2);
        assert_eq!(
            assignments[1].channel,
            Channel::try_new(1).unwrap(),
            "t2 moved"
        );
        let expected: Vec<_> = [(t0, -30, 20), (t2, 40, 127)]
            .iter()
            .zip(&assignments)
            .map(|(&(t, p, v), a)| {
                assert_eq!(a.track, t);
                (a.channel, vol(v), pan(p))
            })
            .collect();
        assert_eq!(
            port.mix_calls, expected,
            "each channel gets its track's mix"
        );
        // The deleted track left the state.
        assert_eq!(s.transport_state().mix_of(t1), TrackMix::default());
    }

    #[test]
    fn x7_out_of_range_values_are_rejected_and_change_nothing() {
        let mut s = playable(2, 2);
        let t = track(&s, 1);
        let mut port = FakePort::default();
        play(&mut s, &mut port);
        let before = (
            s.transport_state().clone(),
            port.loads,
            port.mix_calls.len(),
        );
        for (v, p) in [
            (128, 0),
            (255, 0),
            (256, 0),
            (-1, 0),
            (100, 64),
            (100, -64),
            (100, i32::MAX),
            (i32::MIN, 0),
        ] {
            assert!(
                matches!(
                    TransportCommand::track_mix(t, v, p),
                    Err(TransportError::InvalidMix(_))
                ),
                "{v} {p}"
            );
        }
        assert_eq!(
            before,
            (
                s.transport_state().clone(),
                port.loads,
                port.mix_calls.len()
            )
        );
        // A track that does not exist is a typed error and changes nothing.
        let missing = TrackId::from_raw(987_654);
        let err = s
            .transport(
                TransportCommand::SetTrackMix {
                    track: missing,
                    volume: vol(10),
                    pan: pan(0),
                },
                &mut port,
            )
            .unwrap_err();
        assert_eq!(err, TransportError::UnknownTrack(missing));
        assert_eq!(
            before,
            (
                s.transport_state().clone(),
                port.loads,
                port.mix_calls.len()
            )
        );
    }

    proptest! {
        #[test]
        fn x7_track_mix_accepts_exactly_the_ranges(v in any::<i32>(), p in any::<i32>()) {
            let ok = TransportCommand::track_mix(TrackId::from_raw(1), v, p).is_ok();
            prop_assert_eq!(ok, (0..=127).contains(&v) && (-63..=63).contains(&p));
        }

        #[test]
        fn x8_the_solo_rule_holds_for_every_combination(
            n in 0usize..8,
            muted in proptest::collection::btree_set(0u64..10, 0..10),
            soloed in proptest::collection::btree_set(0u64..10, 0..10),
        ) {
            let existing: Vec<TrackId> = (0..n as u64).map(TrackId::from_raw).collect();
            let m: BTreeSet<TrackId> = muted.iter().copied().map(TrackId::from_raw).collect();
            let so: BTreeSet<TrackId> = soloed.iter().copied().map(TrackId::from_raw).collect();
            let got = effective_muted(&existing, &m, &so);
            let any_solo = existing.iter().any(|t| so.contains(t));
            for t in &existing {
                let expect = m.contains(t) || (any_solo && !so.contains(t));
                prop_assert_eq!(got.contains(t), expect, "track {:?}", t);
            }
            // Nothing outside the existing tracks.
            prop_assert!(got.iter().all(|t| existing.contains(t)));
        }
    }

    #[test]
    fn x8_solo_compiles_like_muting_the_others() {
        let mut soloed = playable(3, 2);
        let t0 = track(&soloed, 0);
        let mut port = FakePort::default();
        soloed
            .transport(
                TransportCommand::SetSoloed {
                    track: t0,
                    soloed: true,
                },
                &mut port,
            )
            .unwrap();
        let by_solo = timeline_text(&mut soloed);

        let mut by_mute = playable(3, 2);
        let (m1, m2) = (track(&by_mute, 1), track(&by_mute, 2));
        for t in [m1, m2] {
            by_mute
                .transport(
                    TransportCommand::SetMuted {
                        track: t,
                        muted: true,
                    },
                    &mut port,
                )
                .unwrap();
        }
        assert_eq!(by_solo, timeline_text(&mut by_mute));
    }

    #[test]
    fn x8_a_muted_track_stays_silent_when_soloed() {
        let mut s = playable(2, 2);
        let (t0, t1) = (track(&s, 0), track(&s, 1));
        let mut port = FakePort::default();
        for cmd in [
            TransportCommand::SetMuted {
                track: t0,
                muted: true,
            },
            TransportCommand::SetSoloed {
                track: t0,
                soloed: true,
            },
        ] {
            s.transport(cmd, &mut port).unwrap();
        }
        // t0 is muted and soloed, t1 is neither soloed nor muted: both are silent.
        let mut both = playable(2, 2);
        let (b0, b1) = (track(&both, 0), track(&both, 1));
        for t in [b0, b1] {
            both.transport(
                TransportCommand::SetMuted {
                    track: t,
                    muted: true,
                },
                &mut port,
            )
            .unwrap();
        }
        assert_eq!(timeline_text(&mut s), timeline_text(&mut both));
        let _ = t1;
    }

    #[test]
    fn x8_deleting_the_soloed_track_makes_everything_sound_again() {
        let mut s = playable(3, 2);
        let (t0, t1) = (track(&s, 0), track(&s, 1));
        let mut port = FakePort::default();
        s.transport(
            TransportCommand::SetSoloed {
                track: t1,
                soloed: true,
            },
            &mut port,
        )
        .unwrap();
        s.execute(cmd(
            3000,
            Command::Track(TrackCommand::Remove { track: t1 }),
        ))
        .unwrap();
        let after = timeline_text(&mut s);
        assert!(
            s.transport_state().soloed().is_empty(),
            "left the soloed set"
        );
        let mut fresh = playable(3, 2);
        let fresh_t1 = track(&fresh, 1);
        fresh
            .execute(cmd(
                3000,
                Command::Track(TrackCommand::Remove { track: fresh_t1 }),
            ))
            .unwrap();
        assert_eq!(after, timeline_text(&mut fresh));
        let _ = t0;
    }

    #[test]
    fn x9_what_reaches_the_port_while_playing() {
        let mut s = playable(3, 2);
        let (t0, t1) = (track(&s, 0), track(&s, 1));
        let mut port = FakePort::default();
        play(&mut s, &mut port);
        let (loads, calls) = (port.loads, port.mix_calls.len());

        // Volume and pan: one channel call, no recompile and no swap.
        set_mix(&mut s, &mut port, t1, 64, 12);
        assert_eq!(port.loads, loads);
        assert_eq!(port.mix_calls.len(), calls + 1);
        assert_eq!(
            port.mix_calls.last().copied(),
            Some((Channel::try_new(1).unwrap(), vol(64), pan(12)))
        );
        // The same values again are not a change: nothing is sent.
        set_mix(&mut s, &mut port, t1, 64, 12);
        assert_eq!(port.mix_calls.len(), calls + 1);
        assert_eq!(port.loads, loads);

        // Solo and mute: exactly one swap each, followed by every mix.
        let calls = port.mix_calls.len();
        s.transport(
            TransportCommand::SetSoloed {
                track: t0,
                soloed: true,
            },
            &mut port,
        )
        .unwrap();
        assert_eq!(port.loads, loads + 1);
        assert_eq!(port.mix_calls.len(), calls + 3);
        s.transport(
            TransportCommand::SetMuted {
                track: t1,
                muted: true,
            },
            &mut port,
        )
        .unwrap();
        assert_eq!(port.loads, loads + 2);
    }

    #[test]
    fn x9_while_stopped_nothing_reaches_the_port_and_play_sends_the_mix() {
        let mut s = playable(2, 2);
        let (t0, t1) = (track(&s, 0), track(&s, 1));
        let mut port = FakePort::default();
        set_mix(&mut s, &mut port, t1, 30, -5);
        s.transport(
            TransportCommand::SetSoloed {
                track: t0,
                soloed: true,
            },
            &mut port,
        )
        .unwrap();
        s.transport(
            TransportCommand::SetMuted {
                track: t1,
                muted: true,
            },
            &mut port,
        )
        .unwrap();
        assert_eq!(port.loads, 0);
        assert!(port.mix_calls.is_empty());
        play(&mut s, &mut port);
        assert!(
            port.mix_calls
                .contains(&(Channel::try_new(1).unwrap(), vol(30), pan(-5))),
            "play sends the stored mix"
        );
    }

    #[test]
    fn x9_events_name_the_change() {
        let mut s = playable(2, 2);
        let t1 = track(&s, 1);
        let mut port = FakePort::default();
        let out = s
            .transport(
                TransportCommand::SetTrackMix {
                    track: t1,
                    volume: vol(70),
                    pan: pan(-9),
                },
                &mut port,
            )
            .unwrap();
        assert!(app_events(&out).contains(&AppEvent::MixChanged {
            track: t1,
            mix: TrackMix {
                volume: vol(70),
                pan: pan(-9)
            }
        }));
        let out = s
            .transport(
                TransportCommand::SetSoloed {
                    track: t1,
                    soloed: true,
                },
                &mut port,
            )
            .unwrap();
        assert!(app_events(&out).contains(&AppEvent::SoloChanged {
            track: t1,
            soloed: true
        }));
    }

    #[test]
    fn x10_mix_and_solo_are_not_saved_and_not_undoable() {
        let mut s = playable(2, 2);
        let (t0, t1) = (track(&s, 0), track(&s, 1));
        let bytes_before = s.snapshot_bytes(&app());
        let (revision, undo_depth, dirty) = (s.revision(), s.undo_depth(), s.is_dirty());
        let mut port = FakePort::default();
        set_mix(&mut s, &mut port, t1, 10, 50);
        s.transport(
            TransportCommand::SetSoloed {
                track: t0,
                soloed: true,
            },
            &mut port,
        )
        .unwrap();
        s.transport(
            TransportCommand::SetMuted {
                track: t1,
                muted: true,
            },
            &mut port,
        )
        .unwrap();
        assert_eq!(
            s.snapshot_bytes(&app()),
            bytes_before,
            "the file bytes are unchanged"
        );
        assert_eq!(
            (s.revision(), s.undo_depth(), s.is_dirty()),
            (revision, undo_depth, dirty)
        );
        // An opened document starts at the defaults.
        let (opened, _) = DocumentSession::from_bytes(
            DocumentId(9),
            &bytes_before,
            None,
            SessionConfig::default(),
            &clock(),
        )
        .unwrap();
        assert!(opened.transport_state().soloed().is_empty());
        assert!(opened.transport_state().muted().is_empty());
        assert_eq!(opened.transport_state().mix_of(t1), TrackMix::default());
        // And so does a new one.
        let fresh = playable(2, 2);
        assert!(fresh.transport_state().soloed().is_empty());
        assert_eq!(fresh.transport_state().mix_of(t1), TrackMix::default());
    }
}
