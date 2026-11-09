#![allow(dead_code)]

use mc_app::ports::{Clock, Entropy, ProjectStore, StoreError};
use mc_app::types::{MasterBar, TrackId};
use mc_app::{
    BeatAddress, Command, CommandEnvelope, DocumentId, DocumentSession, DurationSpec,
    InstrumentSpecInput, ScoreCommand, SessionConfig, TrackCommand,
};
use proptest::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct FixedClock(pub Cell<u64>);

impl Clock for FixedClock {
    fn now_unix_millis(&self) -> u64 {
        self.0.get()
    }
}

pub struct FixedEntropy;

impl Entropy for FixedEntropy {
    fn random_u128(&self) -> u128 {
        0x0123_4567_89ab_cdef_0123_4567_89ab_cdef
    }
}

/// In-memory ProjectStore for session tests.
#[derive(Default)]
pub struct MemStore(pub RefCell<HashMap<PathBuf, Vec<u8>>>);

impl ProjectStore for MemStore {
    fn read(&self, path: &Path) -> Result<Vec<u8>, StoreError> {
        self.0
            .borrow()
            .get(path)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(path.to_path_buf()))
    }

    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.0
            .borrow_mut()
            .insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }
}

pub fn clock() -> FixedClock {
    FixedClock(Cell::new(1_767_225_600_000))
}

pub fn session(bars: usize) -> DocumentSession {
    let score = mc_app::new_score("Track 1", bars).unwrap();
    DocumentSession::new(
        DocumentId(1),
        score,
        SessionConfig::default(),
        &clock(),
        &FixedEntropy,
    )
}

pub fn cmd(id: u64, c: Command) -> CommandEnvelope {
    CommandEnvelope::new(id, c)
}

pub fn first_track(s: &DocumentSession) -> TrackId {
    s.score().track_at(0).unwrap().id()
}

/// Inserts a quarter beat at the end of voice 0 of `bar` and returns its address.
pub fn add_beat(s: &mut DocumentSession, track: TrackId, bar: usize) -> BeatAddress {
    let index = s.score().track(track).unwrap().bar(bar).unwrap().voices[0]
        .beats
        .len();
    s.execute(cmd(
        0,
        Command::Score(ScoreCommand::InsertBeat {
            track,
            bar,
            voice: 0,
            index,
            duration: DurationSpec::plain(2),
        }),
    ))
    .unwrap();
    let beat = s.score().track(track).unwrap().bar(bar).unwrap().voices[0].beats[index].id;
    BeatAddress {
        track,
        bar,
        voice: 0,
        beat,
    }
}

/// Abstract operations for property tests. Indices are resolved against the current
/// score, so most operations are meaningful; some are deliberately invalid.
#[derive(Clone, Debug)]
pub enum Op {
    InsertBars {
        at: usize,
        count: usize,
    },
    RemoveBars {
        at: usize,
        count: usize,
    },
    InsertBeat {
        track: usize,
        bar: usize,
        voice: usize,
        index: usize,
        base: u8,
        dots: u8,
        triplet: bool,
    },
    RemoveBeat {
        track: usize,
        bar: usize,
        beat: usize,
    },
    SetNote {
        track: usize,
        bar: usize,
        beat: usize,
        string: u8,
        fret: u8,
    },
    Tie {
        track: usize,
        bar: usize,
        beat: usize,
        note: usize,
        on: bool,
    },
    AddVoice {
        track: usize,
        bar: usize,
    },
    RemoveVoice {
        track: usize,
        bar: usize,
        voice: usize,
    },
    Tempo {
        bar: usize,
        num: i64,
        bpm: i64,
    },
    Repeat {
        start: usize,
        end: usize,
        count: u8,
    },
    Pickup {
        on: bool,
    },
    AddTrack,
    RemoveTrack {
        track: usize,
    },
    Capo {
        track: usize,
        capo: u8,
    },
    Shift {
        track: usize,
        from: usize,
        len: usize,
        delta: i16,
    },
    Retune {
        track: usize,
        preset: usize,
        capo: u8,
        keep_pitches: bool,
    },
    /// Copies a run of beats from `src_track` and pastes it into `dst_track` (ADR-030).
    Paste {
        dst_track: usize,
        bar: usize,
        voice: usize,
        index: usize,
        src_track: usize,
        from_bar: usize,
        from_index: usize,
        to_extra: usize,
        to_index: usize,
    },
    /// Removes a run of beats (ADR-030 cut).
    Cut {
        track: usize,
        voice: usize,
        from_bar: usize,
        from_index: usize,
        to_extra: usize,
        to_index: usize,
    },
}

pub fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0usize..6, 1usize..3).prop_map(|(at, count)| Op::InsertBars { at, count }),
        (0usize..6, 1usize..3).prop_map(|(at, count)| Op::RemoveBars { at, count }),
        (
            0usize..3,
            0usize..6,
            0usize..2,
            0usize..6,
            0u8..6,
            0u8..2,
            any::<bool>()
        )
            .prop_map(
                |(track, bar, voice, index, base, dots, triplet)| Op::InsertBeat {
                    track,
                    bar,
                    voice,
                    index,
                    base,
                    dots,
                    triplet
                }
            ),
        (0usize..3, 0usize..6, 0usize..6).prop_map(|(track, bar, beat)| Op::RemoveBeat {
            track,
            bar,
            beat
        }),
        (0usize..3, 0usize..6, 0usize..6, 1u8..8, 0u8..26).prop_map(
            |(track, bar, beat, string, fret)| Op::SetNote {
                track,
                bar,
                beat,
                string,
                fret
            }
        ),
        (0usize..3, 0usize..6, 0usize..6, 0usize..3, any::<bool>()).prop_map(
            |(track, bar, beat, note, on)| Op::Tie {
                track,
                bar,
                beat,
                note,
                on
            }
        ),
        (0usize..3, 0usize..6).prop_map(|(track, bar)| Op::AddVoice { track, bar }),
        (0usize..3, 0usize..6, 0usize..3).prop_map(|(track, bar, voice)| Op::RemoveVoice {
            track,
            bar,
            voice
        }),
        (0usize..6, 0i64..5, 10i64..450).prop_map(|(bar, num, bpm)| Op::Tempo { bar, num, bpm }),
        (0usize..6, 0usize..6, 1u8..4).prop_map(|(start, end, count)| Op::Repeat {
            start,
            end,
            count
        }),
        any::<bool>().prop_map(|on| Op::Pickup { on }),
        Just(Op::AddTrack),
        (0usize..3).prop_map(|track| Op::RemoveTrack { track }),
        (0usize..3, 0u8..6).prop_map(|(track, capo)| Op::Capo { track, capo }),
        (0usize..3, 0usize..6, 1usize..3, -3i16..4).prop_map(|(track, from, len, delta)| {
            Op::Shift {
                track,
                from,
                len,
                delta,
            }
        }),
        (0usize..3, 0usize..6, 0u8..3, any::<bool>()).prop_map(
            |(track, preset, capo, keep_pitches)| Op::Retune {
                track,
                preset,
                capo,
                keep_pitches,
            }
        ),
        (
            0usize..3,
            0usize..6,
            0usize..2,
            0usize..6,
            0usize..3,
            0usize..6,
            0usize..4,
            0usize..2,
            0usize..4
        )
            .prop_map(
                |(
                    dst_track,
                    bar,
                    voice,
                    index,
                    src_track,
                    from_bar,
                    from_index,
                    to_extra,
                    to_index,
                )| {
                    Op::Paste {
                        dst_track,
                        bar,
                        voice,
                        index,
                        src_track,
                        from_bar,
                        from_index,
                        to_extra,
                        to_index,
                    }
                }
            ),
        (
            0usize..3,
            0usize..2,
            0usize..6,
            0usize..4,
            0usize..2,
            0usize..4
        )
            .prop_map(
                |(track, voice, from_bar, from_index, to_extra, to_index)| Op::Cut {
                    track,
                    voice,
                    from_bar,
                    from_index,
                    to_extra,
                    to_index,
                }
            ),
    ]
}

/// Converts an abstract op into a command against the current score. Missing targets
/// become commands with impossible addresses, which must be rejected cleanly.
pub fn to_command(s: &DocumentSession, op: &Op) -> Command {
    let score = s.score();
    let tid = |i: usize| score.track_at(i % score.track_count()).unwrap().id();
    let beat_at = |t: usize, bar: usize, k: usize| -> BeatAddress {
        let track = tid(t);
        let id = score
            .track(track)
            .unwrap()
            .bar(bar)
            .and_then(|b| b.voices[0].beats.get(k))
            .map(|b| b.id)
            .unwrap_or(mc_app::types::BeatId::from_raw(u64::MAX));
        BeatAddress {
            track,
            bar,
            voice: 0,
            beat: id,
        }
    };
    match *op {
        Op::InsertBars { at, count } => Command::Score(ScoreCommand::InsertMasterBars {
            at,
            count,
            template: MasterBar::default(),
        }),
        Op::RemoveBars { at, count } => {
            Command::Score(ScoreCommand::RemoveMasterBars { at, count })
        }
        Op::InsertBeat {
            track,
            bar,
            voice,
            index,
            base,
            dots,
            triplet,
        } => Command::Score(ScoreCommand::InsertBeat {
            track: tid(track),
            bar,
            voice,
            index,
            duration: DurationSpec {
                base,
                dots,
                tuplet: triplet.then_some((3, 2)),
            },
        }),
        Op::RemoveBeat { track, bar, beat } => Command::Score(ScoreCommand::RemoveBeat {
            beat: beat_at(track, bar, beat),
        }),
        Op::SetNote {
            track,
            bar,
            beat,
            string,
            fret,
        } => Command::Score(ScoreCommand::SetNote {
            beat: beat_at(track, bar, beat),
            string,
            fret,
        }),
        Op::Tie {
            track,
            bar,
            beat,
            note,
            on,
        } => {
            let b = beat_at(track, bar, beat);
            let nid = score
                .track(b.track)
                .unwrap()
                .bar(bar)
                .and_then(|x| x.voices[0].beats.iter().find(|y| y.id == b.beat))
                .and_then(|y| y.notes.get(note))
                .map(|n| n.id)
                .unwrap_or(mc_app::types::NoteId::from_raw(u64::MAX));
            Command::Score(ScoreCommand::SetTie {
                beat: b,
                note: nid,
                tie: on,
            })
        }
        Op::AddVoice { track, bar } => Command::Score(ScoreCommand::AddVoice {
            track: tid(track),
            bar,
        }),
        Op::RemoveVoice { track, bar, voice } => Command::Score(ScoreCommand::RemoveVoice {
            track: tid(track),
            bar,
            voice,
        }),
        Op::Tempo { bar, num, bpm } => Command::Score(ScoreCommand::SetTempo {
            bar,
            offset: (num, 4),
            bpm: (bpm, 1),
        }),
        Op::Repeat { start, end, count } => {
            let n = score.master_bar_count();
            let (start, end) = (start % n, end % n);
            let mut bar = score.master_bars()[end].clone();
            bar.repeat_end = (count > 1).then_some(count);
            if start == end {
                bar.repeat_start = true;
            }
            Command::Score(ScoreCommand::SetMasterBar { index: end, bar })
        }
        Op::Pickup { on } => {
            let mut bar = score.master_bars()[0].clone();
            bar.pickup = on;
            Command::Score(ScoreCommand::SetMasterBar { index: 0, bar })
        }
        Op::AddTrack => Command::Track(TrackCommand::Add {
            name: "Extra".into(),
            instrument: InstrumentSpecInput::standard_guitar(),
        }),
        Op::RemoveTrack { track } => Command::Track(TrackCommand::Remove { track: tid(track) }),
        Op::Capo { track, capo } => Command::Track(TrackCommand::SetInstrument {
            track: tid(track),
            instrument: InstrumentSpecInput {
                capo,
                ..InstrumentSpecInput::standard_guitar()
            },
        }),
        Op::Shift {
            track,
            from,
            len,
            delta,
        } => Command::Score(ScoreCommand::ShiftFrets {
            track: tid(track),
            bars: (from, from + len),
            delta,
        }),
        Op::Retune {
            track,
            preset,
            capo,
            keep_pitches,
        } => {
            let preset = mc_app::guitar::TuningPreset::ALL[preset % 6];
            Command::Guitar(mc_app::GuitarCommand::Retune {
                track: tid(track),
                instrument: InstrumentSpecInput {
                    tuning: preset.pitches().map(String::from).to_vec(),
                    capo,
                    ..InstrumentSpecInput::standard_guitar()
                },
                mode: if keep_pitches {
                    mc_app::guitar::RetuneMode::KeepPitches
                } else {
                    mc_app::guitar::RetuneMode::KeepFrets
                },
            })
        }
        Op::Paste {
            dst_track,
            bar,
            voice,
            index,
            src_track,
            from_bar,
            from_index,
            to_extra,
            to_index,
        } => {
            // A source run that does not exist gives an empty fragment, which the core
            // must reject cleanly.
            let beats = s
                .copy_beats(
                    tid(src_track),
                    voice,
                    (from_bar, from_index),
                    (from_bar + to_extra, to_index),
                )
                .unwrap_or_default();
            Command::Score(ScoreCommand::PasteBeats {
                track: tid(dst_track),
                bar,
                voice,
                index,
                beats,
            })
        }
        Op::Cut {
            track,
            voice,
            from_bar,
            from_index,
            to_extra,
            to_index,
        } => Command::Score(ScoreCommand::RemoveBeats {
            track: tid(track),
            voice,
            from: (from_bar, from_index),
            to: (from_bar + to_extra, to_index),
        }),
    }
}
