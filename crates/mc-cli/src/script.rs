//! Edit scripts: one command per line, `#` starts a comment. Tracks, bars, voices and
//! beats are 0-based indices; strings are 1-based (string 1 is the highest). The CLI
//! translates indices into stable ids and sends ordinary commands; all musical rules
//! are checked by the core.
//!
//! ```text
//! add-track <name>                      rename-track <t> <name>
//! remove-track <t>                      move-track <t> <to>
//! tuning <t> <pitch,...> [frets] [capo] insert-bars <at> <count> [<num>/<den>]
//! remove-bars <at> <count>              time <bar> <num>/<den>
//! pickup <bar> on|off                   repeat <start> <end> <count>
//! tempo <bar> <bpm> [<off-num>/<off-den>]
//! beat <t> <bar> <voice> <index> <dur>  remove-beat <t> <bar> <voice> <index>
//! note <t> <bar> <voice> <beat> <string> <fret>
//! remove-note <t> <bar> <voice> <beat> <string>
//! tie <t> <bar> <voice> <beat> <string> on|off
//! voice <t> <bar>                       shift <t> <from-bar> <to-bar> <delta>
//! retune <t> <preset|pitch,...> [capo] keep-pitches|keep-frets
//! undo                                  redo
//! ```
//!
//! Tuning presets: standard, drop-d, dadgad, open-g, open-d, half-step-down.
//!
//! Durations: `w h q e s t` (whole ... 1/32) or a number 0..=8, with `.` per dot and a
//! `/3` suffix for a triplet, for example `q.`, `e/3`, `h..`.

use mc_app::guitar::{RetuneMode, TuningPreset};
use mc_app::types::{MasterBar, Score, TimeSignature};
use mc_app::{
    BeatAddress, Command, CommandEnvelope, DocumentSession, DurationSpec, GuitarCommand,
    InstrumentSpecInput, ScoreCommand, TrackCommand,
};

pub enum Step {
    Command(Command),
    /// Several commands that form one undo step (same coalescing key).
    Batch(Vec<Command>),
    Undo,
    Redo,
}

pub fn run(session: &mut DocumentSession, text: &str) -> Result<usize, String> {
    let mut applied = 0;
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let at = |e: String| format!("line {}: {line}: {e}", n + 1);
        let id = (n + 1) as u64;
        match parse(session.score(), line).map_err(at)? {
            Step::Command(c) => {
                session
                    .execute(CommandEnvelope::new(id, c))
                    .map_err(|e| at(e.to_string()))?;
            }
            Step::Batch(cmds) => {
                for c in cmds {
                    let env = CommandEnvelope {
                        coalesce_key: Some(u64::MAX - id),
                        ..CommandEnvelope::new(id, c)
                    };
                    session.execute(env).map_err(|e| at(e.to_string()))?;
                }
            }
            Step::Undo => {
                session.undo().map_err(|e| at(e.to_string()))?;
            }
            Step::Redo => {
                session.redo().map_err(|e| at(e.to_string()))?;
            }
        }
        applied += 1;
    }
    Ok(applied)
}

fn num<T: std::str::FromStr>(s: Option<&&str>, what: &str) -> Result<T, String> {
    s.ok_or_else(|| format!("missing {what}"))?
        .parse()
        .map_err(|_| format!("invalid {what} '{}'", s.unwrap_or(&"")))
}

fn ratio(s: &str) -> Result<(i64, i64), String> {
    let (a, b) = s
        .split_once('/')
        .ok_or_else(|| format!("expected n/d, got '{s}'"))?;
    Ok((
        a.parse().map_err(|_| format!("invalid number '{a}'"))?,
        b.parse().map_err(|_| format!("invalid number '{b}'"))?,
    ))
}

fn on_off(s: Option<&&str>) -> Result<bool, String> {
    match s.copied() {
        Some("on") => Ok(true),
        Some("off") => Ok(false),
        other => Err(format!("expected on|off, got {other:?}")),
    }
}

pub fn duration(s: &str) -> Result<DurationSpec, String> {
    let (body, tuplet) = match s.split_once('/') {
        Some((b, "3")) => (b, Some((3, 2))),
        Some((_, other)) => return Err(format!("unsupported tuplet '/{other}' (only /3)")),
        None => (s, None),
    };
    let dots = body.chars().rev().take_while(|&c| c == '.').count() as u8;
    let base = match &body[..body.len() - dots as usize] {
        "w" => 0,
        "h" => 1,
        "q" => 2,
        "e" => 3,
        "s" => 4,
        "t" => 5,
        other => other
            .parse()
            .map_err(|_| format!("invalid duration '{s}'"))?,
    };
    Ok(DurationSpec { base, dots, tuplet })
}

fn track_id(score: &Score, index: usize) -> Result<mc_app::types::TrackId, String> {
    score
        .track_at(index)
        .map(|t| t.id())
        .ok_or_else(|| format!("no track {index}"))
}

fn beat_address(score: &Score, a: &[&str]) -> Result<BeatAddress, String> {
    let t: usize = num(a.first(), "track")?;
    let bar: usize = num(a.get(1), "bar")?;
    let voice: usize = num(a.get(2), "voice")?;
    let k: usize = num(a.get(3), "beat")?;
    let track = track_id(score, t)?;
    let beat = score
        .track(track)
        .and_then(|tr| tr.bar(bar))
        .and_then(|b| b.voices.get(voice))
        .and_then(|v| v.beats.get(k))
        .map(|b| b.id)
        .ok_or_else(|| format!("no beat {k} in track {t}, bar {bar}, voice {voice}"))?;
    Ok(BeatAddress {
        track,
        bar,
        voice,
        beat,
    })
}

fn note_on_string(
    score: &Score,
    b: BeatAddress,
    string: u8,
) -> Result<mc_app::types::NoteId, String> {
    score
        .track(b.track)
        .and_then(|t| t.bar(b.bar))
        .and_then(|bar| bar.voices[b.voice].beats.iter().find(|x| x.id == b.beat))
        .and_then(|beat| {
            beat.notes
                .iter()
                .find(|n| n.position().string.get() == string)
        })
        .map(|n| n.id)
        .ok_or_else(|| format!("no note on string {string}"))
}

fn master_bar(score: &Score, index: usize) -> Result<MasterBar, String> {
    score
        .master_bars()
        .get(index)
        .cloned()
        .ok_or_else(|| format!("no bar {index}"))
}

pub fn parse(score: &Score, line: &str) -> Result<Step, String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let (verb, a) = words.split_first().ok_or("empty line")?;
    let sc = |c: ScoreCommand| Ok(Step::Command(Command::Score(c)));
    let tc = |c: TrackCommand| Ok(Step::Command(Command::Track(c)));
    match *verb {
        "undo" => Ok(Step::Undo),
        "redo" => Ok(Step::Redo),
        "add-track" => tc(TrackCommand::Add {
            name: a.join(" "),
            instrument: InstrumentSpecInput::standard_guitar(),
        }),
        "remove-track" => tc(TrackCommand::Remove {
            track: track_id(score, num(a.first(), "track")?)?,
        }),
        "rename-track" => tc(TrackCommand::Rename {
            track: track_id(score, num(a.first(), "track")?)?,
            name: a[1..].join(" "),
        }),
        "move-track" => tc(TrackCommand::Move {
            track: track_id(score, num(a.first(), "track")?)?,
            to: num(a.get(1), "position")?,
        }),
        "tuning" => {
            let track = track_id(score, num(a.first(), "track")?)?;
            let tuning = a
                .get(1)
                .ok_or("missing tuning")?
                .split(',')
                .map(String::from)
                .collect();
            let fret_count = a
                .get(2)
                .map(|_| num(a.get(2), "fret count"))
                .transpose()?
                .unwrap_or(24);
            let capo = a
                .get(3)
                .map(|_| num(a.get(3), "capo"))
                .transpose()?
                .unwrap_or(0);
            tc(TrackCommand::SetInstrument {
                track,
                instrument: InstrumentSpecInput {
                    tuning,
                    fret_count,
                    capo,
                },
            })
        }
        "retune" => {
            let index: usize = num(a.first(), "track")?;
            let track = track_id(score, index)?;
            let spec = a.get(1).ok_or("missing tuning")?;
            let tuning = match TuningPreset::from_name(spec) {
                Ok(p) => p.pitches().map(String::from).to_vec(),
                Err(_) => spec.split(',').map(String::from).collect(),
            };
            let (capo, mode) = match a.len() {
                3 => (0, a[2]),
                4 => (num(a.get(2), "capo")?, a[3]),
                _ => {
                    return Err(
                        "expected retune <t> <tuning> [capo] keep-pitches|keep-frets".into(),
                    );
                }
            };
            let mode = RetuneMode::from_name(mode)
                .ok_or_else(|| format!("expected keep-pitches or keep-frets, got '{mode}'"))?;
            let fret_count = score
                .track(track)
                .map(|t| t.fretted().fret_count)
                .unwrap_or(24);
            Ok(Step::Command(Command::Guitar(GuitarCommand::Retune {
                track,
                instrument: InstrumentSpecInput {
                    tuning,
                    fret_count,
                    capo,
                },
                mode,
            })))
        }
        "insert-bars" => {
            let mut template = MasterBar::default();
            if let Some(ts) = a.get(2) {
                let (n, d) = ratio(ts)?;
                template.time_signature =
                    TimeSignature::new(n as u8, d as u8).map_err(|e| e.to_string())?;
            }
            sc(ScoreCommand::InsertMasterBars {
                at: num(a.first(), "position")?,
                count: num(a.get(1), "count")?,
                template,
            })
        }
        "remove-bars" => sc(ScoreCommand::RemoveMasterBars {
            at: num(a.first(), "position")?,
            count: num(a.get(1), "count")?,
        }),
        "time" => {
            let index = num(a.first(), "bar")?;
            let (n, d) = ratio(a.get(1).ok_or("missing time signature")?)?;
            let mut bar = master_bar(score, index)?;
            bar.time_signature = TimeSignature::new(n as u8, d as u8).map_err(|e| e.to_string())?;
            sc(ScoreCommand::SetMasterBar { index, bar })
        }
        "pickup" => {
            let index = num(a.first(), "bar")?;
            let mut bar = master_bar(score, index)?;
            bar.pickup = on_off(a.get(1))?;
            sc(ScoreCommand::SetMasterBar { index, bar })
        }
        "repeat" => {
            // Marks the start bar and sets the count on the end bar. Two commands, run
            // as one undo step (see `run`).
            let (start, end): (usize, usize) = (num(a.first(), "start")?, num(a.get(1), "end")?);
            let count: u8 = num(a.get(2), "count")?;
            let mut first = master_bar(score, start)?;
            first.repeat_start = true;
            if start == end {
                first.repeat_end = Some(count);
                return sc(ScoreCommand::SetMasterBar {
                    index: start,
                    bar: first,
                });
            }
            let mut last = master_bar(score, end)?;
            last.repeat_end = Some(count);
            Ok(Step::Batch(vec![
                Command::Score(ScoreCommand::SetMasterBar {
                    index: start,
                    bar: first,
                }),
                Command::Score(ScoreCommand::SetMasterBar {
                    index: end,
                    bar: last,
                }),
            ]))
        }
        "tempo" => {
            let bpm = num(a.get(1), "bpm")?;
            let offset = a.get(2).map(|o| ratio(o)).transpose()?.unwrap_or((0, 1));
            sc(ScoreCommand::SetTempo {
                bar: num(a.first(), "bar")?,
                offset,
                bpm: (bpm, 1),
            })
        }
        "beat" => sc(ScoreCommand::InsertBeat {
            track: track_id(score, num(a.first(), "track")?)?,
            bar: num(a.get(1), "bar")?,
            voice: num(a.get(2), "voice")?,
            index: num(a.get(3), "index")?,
            duration: duration(a.get(4).ok_or("missing duration")?)?,
        }),
        "remove-beat" => sc(ScoreCommand::RemoveBeat {
            beat: beat_address(score, a)?,
        }),
        "note" => sc(ScoreCommand::SetNote {
            beat: beat_address(score, a)?,
            string: num(a.get(4), "string")?,
            fret: num(a.get(5), "fret")?,
        }),
        "remove-note" => {
            let b = beat_address(score, a)?;
            sc(ScoreCommand::RemoveNote {
                beat: b,
                note: note_on_string(score, b, num(a.get(4), "string")?)?,
            })
        }
        "tie" => {
            let b = beat_address(score, a)?;
            sc(ScoreCommand::SetTie {
                beat: b,
                note: note_on_string(score, b, num(a.get(4), "string")?)?,
                tie: on_off(a.get(5))?,
            })
        }
        "voice" => sc(ScoreCommand::AddVoice {
            track: track_id(score, num(a.first(), "track")?)?,
            bar: num(a.get(1), "bar")?,
        }),
        "shift" => sc(ScoreCommand::ShiftFrets {
            track: track_id(score, num(a.first(), "track")?)?,
            bars: (
                num(a.get(1), "from bar")?,
                num::<usize>(a.get(2), "to bar")? + 1,
            ),
            delta: num(a.get(3), "delta")?,
        }),
        other => Err(format!("unknown command '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(
            duration("q").unwrap(),
            DurationSpec {
                base: 2,
                dots: 0,
                tuplet: None
            }
        );
        assert_eq!(
            duration("h..").unwrap(),
            DurationSpec {
                base: 1,
                dots: 2,
                tuplet: None
            }
        );
        assert_eq!(
            duration("e/3").unwrap(),
            DurationSpec {
                base: 3,
                dots: 0,
                tuplet: Some((3, 2))
            }
        );
        assert_eq!(duration("6").unwrap().base, 6);
        assert!(duration("x").is_err());
        assert!(duration("q/5").is_err());
    }

    #[test]
    fn retune_lines() {
        let score = mc_app::new_score("t", 1).unwrap();
        let Ok(Step::Command(Command::Guitar(GuitarCommand::Retune {
            instrument, mode, ..
        }))) = parse(&score, "retune 0 drop-d 2 keep-pitches")
        else {
            panic!("expected a retune command");
        };
        assert_eq!(instrument.tuning[5], "D2");
        assert_eq!((instrument.capo, mode), (2, RetuneMode::KeepPitches));
        assert!(parse(&score, "retune 0 E4,B3,G3,D3,A2,E2 keep-frets").is_ok());
        assert!(parse(&score, "retune 0 drop-d sideways").is_err());
        assert!(parse(&score, "retune 0").is_err());
    }
}
