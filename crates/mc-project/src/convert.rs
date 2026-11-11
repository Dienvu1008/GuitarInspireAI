//! DTO <-> domain conversion. Loading always ends in `Score::from_parts`, which checks
//! every hard invariant (P-05, GR-09).

use crate::dto::*;
use mc_score::{
    BarContent, Beat, BeatId, FrettedInstrument, InstrumentSpec, MasterBar, Note, NoteEffects,
    NoteId, NoteKind, Score, ScoreParts, Stroke, TempoChange, TrackData, TrackId, Violation, Voice,
};
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning};
use mc_theory::{BaseValue, KeySignature, Mode, NoteValue, Rational, Tempo, TimeSignature, Tuplet};

pub fn to_dto(score: &Score) -> ScoreDto {
    ScoreDto {
        master_bars: score.master_bars().iter().map(master_bar_dto).collect(),
        tempo_map: score
            .tempo_map()
            .iter()
            .map(|c| TempoChangeDto {
                bar: c.bar as u32,
                offset: [c.offset.numerator(), c.offset.denominator()],
                bpm: [c.tempo.bpm().numerator(), c.tempo.bpm().denominator()],
                extra: Extra::new(),
            })
            .collect(),
        tracks: score
            .tracks()
            .map(|t| {
                let InstrumentSpec::Fretted(inst) = t.instrument();
                TrackDto {
                    id: t.id().raw(),
                    name: t.name().to_string(),
                    instrument: InstrumentDto {
                        tuning: inst
                            .tuning
                            .strings()
                            .iter()
                            .map(|p| p.to_string())
                            .collect(),
                        fret_count: inst.fret_count,
                        capo: inst.capo.0,
                        extra: Extra::new(),
                    },
                    bars: t
                        .bars()
                        .map(|b| BarDto {
                            voices: b
                                .voices
                                .iter()
                                .map(|v| VoiceDto {
                                    beats: v.beats.iter().map(beat_dto).collect(),
                                    extra: Extra::new(),
                                })
                                .collect(),
                            extra: Extra::new(),
                        })
                        .collect(),
                    extra: Extra::new(),
                }
            })
            .collect(),
        extra: Extra::new(),
    }
}

fn master_bar_dto(m: &MasterBar) -> MasterBarDto {
    MasterBarDto {
        time_signature: [m.time_signature.numerator(), m.time_signature.denominator()],
        key: KeyDto {
            fifths: m.key.fifths(),
            mode: match m.key.mode() {
                Mode::Major => ModeDto::Major,
                Mode::Minor => ModeDto::Minor,
            },
            extra: Extra::new(),
        },
        pickup: m.pickup,
        repeat_start: m.repeat_start,
        repeat_end: m.repeat_end,
        alternate_endings: m.alternate_endings.clone(),
        marker: m.marker.clone(),
        extra: Extra::new(),
    }
}

fn beat_dto(b: &Beat) -> BeatDto {
    BeatDto {
        id: b.id.raw(),
        value: value_dto(b.value),

        notes: b
            .notes
            .iter()
            .map(|n| {
                let p = n.position();
                NoteDto {
                    id: n.id.raw(),
                    string: p.string.get(),
                    fret: p.fret.0,
                    tie: n.tie,
                    let_ring: n.effects.let_ring,
                    dead: n.effects.dead,
                    accent: n.effects.accent,
                    palm_mute: n.effects.palm_mute,
                    string_percussive: n.effects.string_percussive,
                    extra: Extra::new(),
                }
            })
            .collect(),
        stroke: b.stroke.map(|s| match s {
            Stroke::Down => StrokeDto::Down,
            Stroke::Up => StrokeDto::Up,
        }),
        extra: Extra::new(),
    }
}

/// Collects construction errors with their JSON path instead of stopping at the first.
pub(crate) struct Ctx {
    pub(crate) violations: Vec<Violation>,
}

impl Ctx {
    pub(crate) fn fail(&mut self, path: String, message: impl ToString) {
        self.violations.push(Violation {
            rule: "P-05",
            location: path,
            message: message.to_string(),
        });
    }

    pub(crate) fn rational(&mut self, path: String, v: [i64; 2]) -> Option<Rational> {
        Rational::new(v[0], v[1])
            .map_err(|e| self.fail(path, e))
            .ok()
    }
}

pub fn from_dto(dto: &ScoreDto) -> Result<Score, Vec<Violation>> {
    let mut cx = Ctx {
        violations: Vec::new(),
    };

    let master_bars: Vec<MasterBar> = dto
        .master_bars
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let path = format!("$.master_bars[{i}]");
            let ts = TimeSignature::new(m.time_signature[0], m.time_signature[1])
                .map_err(|e| cx.fail(format!("{path}.time_signature"), e))
                .ok()?;
            let mode = match m.key.mode {
                ModeDto::Major => Mode::Major,
                ModeDto::Minor => Mode::Minor,
            };
            let key = KeySignature::new(m.key.fifths, mode)
                .map_err(|e| cx.fail(format!("{path}.key"), e))
                .ok()?;
            Some(MasterBar {
                time_signature: ts,
                key,
                pickup: m.pickup,
                repeat_start: m.repeat_start,
                repeat_end: m.repeat_end,
                alternate_endings: m.alternate_endings.clone(),
                marker: m.marker.clone(),
            })
        })
        .collect();

    let tempo_map: Vec<TempoChange> = dto
        .tempo_map
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let path = format!("$.tempo_map[{i}]");
            let offset = cx.rational(format!("{path}.offset"), c.offset)?;
            let bpm = cx.rational(format!("{path}.bpm"), c.bpm)?;
            let tempo = Tempo::new(bpm)
                .map_err(|e| cx.fail(format!("{path}.bpm"), e))
                .ok()?;
            Some(TempoChange {
                bar: c.bar as usize,
                offset,
                tempo,
            })
        })
        .collect();

    let tracks: Vec<TrackData> = dto
        .tracks
        .iter()
        .enumerate()
        .filter_map(|(ti, t)| {
            let path = format!("$.tracks[{ti}]");
            let tuning: Result<Vec<_>, _> = t.instrument.tuning.iter().map(|p| p.parse()).collect();
            let tuning = tuning
                .map_err(|e| cx.fail(format!("{path}.instrument.tuning"), e))
                .ok()
                .and_then(|s| {
                    Tuning::new(s)
                        .map_err(|e| cx.fail(format!("{path}.instrument.tuning"), e))
                        .ok()
                })?;
            let bars = t
                .bars
                .iter()
                .enumerate()
                .map(|(bi, b)| BarContent {
                    voices: b
                        .voices
                        .iter()
                        .enumerate()
                        .map(|(vi, v)| Voice {
                            beats: v
                                .beats
                                .iter()
                                .enumerate()
                                .filter_map(|(ki, beat)| {
                                    beat_from(
                                        &mut cx,
                                        &format!("{path}.bars[{bi}].voices[{vi}].beats[{ki}]"),
                                        beat,
                                    )
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect();
            Some(TrackData {
                id: TrackId::from_raw(t.id),
                name: t.name.clone(),
                instrument: InstrumentSpec::Fretted(FrettedInstrument {
                    tuning,
                    fret_count: t.instrument.fret_count,
                    capo: Capo(t.instrument.capo),
                }),
                bars,
            })
        })
        .collect();

    if !cx.violations.is_empty() {
        return Err(cx.violations);
    }
    Score::from_parts(ScoreParts {
        master_bars,
        tracks,
        tempo_map,
    })
}

/// A stored note value; `path` is the path of the value itself.
pub(crate) fn value_from(cx: &mut Ctx, path: &str, v: &ValueDto) -> Option<NoteValue> {
    let base = BaseValue::from_exponent(v.base)
        .map_err(|e| cx.fail(format!("{path}.base"), e))
        .ok()?;
    let tuplet = match v.tuplet {
        None => None,
        Some([a, n]) => Some(
            Tuplet::new(a, n)
                .map_err(|e| cx.fail(format!("{path}.tuplet"), e))
                .ok()?,
        ),
    };
    NoteValue::new(base, v.dots, tuplet)
        .map_err(|e| cx.fail(path.to_string(), e))
        .ok()
}

pub(crate) fn value_dto(v: NoteValue) -> ValueDto {
    ValueDto {
        base: v.base().exponent(),
        dots: v.dots(),
        tuplet: v.tuplet().map(|t| [t.actual(), t.normal()]),
        extra: Extra::new(),
    }
}

fn beat_from(cx: &mut Ctx, path: &str, b: &BeatDto) -> Option<Beat> {
    let value = value_from(cx, &format!("{path}.value"), &b.value)?;
    let notes = b
        .notes
        .iter()
        .enumerate()
        .filter_map(|(ni, n)| {
            let string = StringIndex::new(n.string)
                .map_err(|e| cx.fail(format!("{path}.notes[{ni}].string"), e))
                .ok()?;
            Some(Note {
                id: NoteId::from_raw(n.id),
                kind: NoteKind::Fretted(FretPosition {
                    string,
                    fret: Fret(n.fret),
                }),
                tie: n.tie,
                effects: NoteEffects {
                    let_ring: n.let_ring,
                    dead: n.dead,
                    accent: n.accent,
                    palm_mute: n.palm_mute,
                    string_percussive: n.string_percussive,
                },
            })
        })
        .collect();
    Some(Beat {
        id: BeatId::from_raw(b.id),
        value,
        notes,
        stroke: b.stroke.map(|s| match s {
            StrokeDto::Down => Stroke::Down,
            StrokeDto::Up => Stroke::Up,
        }),
    })
}
