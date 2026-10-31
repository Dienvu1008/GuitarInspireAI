//! Command handlers, one module per context. The dispatcher in `session.rs` only routes.

use crate::command::{BeatAddress, DurationSpec, InstrumentSpecInput, ScoreCommand, TrackCommand};
use crate::{CommandError, Constraints};
use mc_score::{BeatRef, DomainEvent, FrettedInstrument, InstrumentSpec, Score};
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning};
use mc_theory::{BaseValue, NoteValue, Rational, Tempo, Tuplet};

pub(crate) type Events = Vec<DomainEvent>;

pub(crate) fn duration(d: DurationSpec) -> Result<NoteValue, CommandError> {
    let tuplet = d.tuplet.map(|(a, n)| Tuplet::new(a, n)).transpose()?;
    Ok(NoteValue::new(
        BaseValue::from_exponent(d.base)?,
        d.dots,
        tuplet,
    )?)
}

pub(crate) fn instrument(
    i: &InstrumentSpecInput,
    c: &Constraints,
) -> Result<InstrumentSpec, CommandError> {
    c.strings(i.tuning.len())?;
    c.fret_count(i.fret_count)?;
    let strings = i
        .tuning
        .iter()
        .map(|p| p.parse())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(InstrumentSpec::Fretted(FrettedInstrument {
        tuning: Tuning::new(strings)?,
        fret_count: i.fret_count,
        capo: Capo(i.capo),
    }))
}

fn beat(a: BeatAddress) -> BeatRef {
    BeatRef {
        track: a.track,
        bar: a.bar,
        voice: a.voice,
        beat: a.beat,
    }
}

fn rational(v: (i64, i64)) -> Result<Rational, CommandError> {
    Ok(Rational::new(v.0, v.1)?)
}

pub(crate) mod score_handlers {
    use super::*;

    pub(crate) fn handle(
        score: &mut Score,
        cmd: ScoreCommand,
        c: &Constraints,
    ) -> Result<Events, CommandError> {
        Ok(match cmd {
            ScoreCommand::InsertMasterBars {
                at,
                count,
                template,
            } => score.insert_master_bars(at, count, template)?,
            ScoreCommand::RemoveMasterBars { at, count } => score.remove_master_bars(at, count)?,
            ScoreCommand::SetMasterBar { index, bar } => score.set_master_bar(index, bar)?,
            ScoreCommand::SetTempo { bar, offset, bpm } => {
                let bpm = rational(bpm)?;
                c.tempo(bpm)?;
                score.set_tempo(bar, rational(offset)?, Tempo::new(bpm)?)?
            }
            ScoreCommand::RemoveTempo { bar, offset } => {
                score.remove_tempo(bar, rational(offset)?)?
            }
            ScoreCommand::InsertBeat {
                track,
                bar,
                voice,
                index,
                duration: d,
            } => score.insert_beat(track, bar, voice, index, duration(d)?)?.1,
            ScoreCommand::RemoveBeat { beat: b } => score.remove_beat(beat(b))?,
            ScoreCommand::SetBeatDuration {
                beat: b,
                duration: d,
            } => score.set_beat_value(beat(b), duration(d)?)?,
            ScoreCommand::SetNote {
                beat: b,
                string,
                fret,
            } => {
                score
                    .set_note(
                        beat(b),
                        FretPosition {
                            string: StringIndex::new(string)?,
                            fret: Fret(fret),
                        },
                    )?
                    .1
            }
            ScoreCommand::RemoveNote { beat: b, note } => score.remove_note(beat(b), note)?,
            ScoreCommand::SetTie { beat: b, note, tie } => score.set_tie(beat(b), note, tie)?,
            ScoreCommand::SetNoteEffects {
                beat: b,
                note,
                effects,
            } => score.set_note_effects(beat(b), note, effects)?,
            ScoreCommand::AddVoice { track, bar } => score.add_voice(track, bar)?.1,
            ScoreCommand::RemoveVoice { track, bar, voice } => {
                score.remove_voice(track, bar, voice)?
            }
            ScoreCommand::ShiftFrets { track, bars, delta } => {
                score.shift_frets(track, bars.0..bars.1, delta)?
            }
        })
    }
}

pub(crate) mod track_handlers {
    use super::*;

    pub(crate) fn handle(
        score: &mut Score,
        cmd: TrackCommand,
        c: &Constraints,
    ) -> Result<Events, CommandError> {
        Ok(match cmd {
            TrackCommand::Add {
                name,
                instrument: i,
            } => {
                c.track_name(&name)?;
                score.add_track(&name, instrument(&i, c)?)?.1
            }
            TrackCommand::Remove { track } => score.remove_track(track)?,
            TrackCommand::Rename { track, name } => {
                c.track_name(&name)?;
                score.rename_track(track, &name)?
            }
            TrackCommand::SetInstrument {
                track,
                instrument: i,
            } => score.set_instrument(track, instrument(&i, c)?)?,
            TrackCommand::Move { track, to } => score.move_track(track, to)?,
        })
    }
}
