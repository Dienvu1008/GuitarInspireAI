//! Transport (slice 4d): the session drives an [`AudioPort`] with compiled timelines
//! and simple commands. These tests use a fake port that records calls, so they check
//! the application's decisions (compile, mute, swap on edit, start bar, events) without
//! any audio hardware.

mod common;
use common::*;
use mc_app::playback::{PerformanceTimeline, TempoFactor, Tick};
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
