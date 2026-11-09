//! Aggregate operations. Every method checks all preconditions before mutating, so a
//! returned error leaves the score unchanged. Mutation uses copy-on-write
//! (`Arc::make_mut`): only the edited track spine and bars are copied.

use crate::model::{
    BarContent, Beat, InstrumentSpec, MasterBar, Note, NoteEffects, NoteKind, Stroke, TempoChange,
    Track, Voice,
};
use crate::validate::{check_instrument, check_note, tempo_offset_valid, tie_is_valid};
use crate::{BeatId, DomainError, DomainEvent, NoteId, Score, TrackId, repeats};
use mc_theory::fretted::{Fret, FretPosition};
use mc_theory::{NoteValue, Rational, Tempo};
use std::ops::Range;
use std::sync::Arc;

/// Address of a beat: written location plus the beat's stable id (checked).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeatRef {
    pub track: TrackId,
    pub bar: usize,
    pub voice: usize,
    pub beat: BeatId,
}

/// Position of a beat inside a voice run: written bar and beat index within that bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeatSlot {
    pub bar: usize,
    pub index: usize,
}

/// A note of a clipboard fragment: plain values, no ids (ADR-030).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FragmentNote {
    pub position: FretPosition,
    pub tie: bool,
    pub effects: NoteEffects,
}

/// A beat of a clipboard fragment: its written value, its notes and its stroke, no ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentBeat {
    pub value: NoteValue,
    pub notes: Vec<FragmentNote>,
    pub stroke: Option<Stroke>,
}

struct Located {
    track: usize,
    bar: usize,
    voice: usize,
    beat: usize,
}

type Events = Vec<DomainEvent>;

impl Score {
    // ------------------------------------------------------------------ tracks

    pub fn add_track(
        &mut self,
        name: &str,
        instrument: InstrumentSpec,
    ) -> Result<(TrackId, Events), DomainError> {
        let InstrumentSpec::Fretted(inst) = &instrument;
        check_instrument(inst).map_err(|m| DomainError::invariant("H-10", m))?;
        let id = TrackId::from_raw(self.allocate_id());
        let empty = Arc::new(BarContent::empty());
        let bars = (0..self.master_bars.len())
            .map(|_| Arc::clone(&empty))
            .collect();
        self.tracks.push(Arc::new(Track {
            id,
            name: name.to_string(),
            instrument,
            bars,
        }));
        Ok((id, vec![DomainEvent::TrackAdded { track: id }]))
    }

    pub fn remove_track(&mut self, id: TrackId) -> Result<Events, DomainError> {
        let t = self.track_index(id)?;
        if self.tracks.len() == 1 {
            return Err(DomainError::CannotRemoveLast("track"));
        }
        self.tracks.remove(t);
        Ok(vec![DomainEvent::TrackRemoved { track: id }])
    }

    pub fn rename_track(&mut self, id: TrackId, name: &str) -> Result<Events, DomainError> {
        let t = self.track_index(id)?;
        Arc::make_mut(&mut self.tracks[t]).name = name.to_string();
        Ok(vec![DomainEvent::TrackChanged { track: id }])
    }

    /// Changes tuning, fret count or capo. Every existing note must stay valid
    /// (H-03, H-04, H-10); moving notes on retune is a Guitar service (Phase 2).
    pub fn set_instrument(
        &mut self,
        id: TrackId,
        instrument: InstrumentSpec,
    ) -> Result<Events, DomainError> {
        let t = self.track_index(id)?;
        let InstrumentSpec::Fretted(inst) = &instrument;
        check_instrument(inst).map_err(|m| DomainError::invariant("H-10", m))?;
        for (b, bar) in self.tracks[t].bars.iter().enumerate() {
            for voice in &bar.voices {
                for beat in &voice.beats {
                    for note in &beat.notes {
                        check_note(inst, note).map_err(|(rule, m)| {
                            DomainError::invariant(
                                rule,
                                format!("bar {b}, note {}: {m}", note.id.raw()),
                            )
                        })?;
                    }
                }
            }
        }
        Arc::make_mut(&mut self.tracks[t]).instrument = instrument;
        Ok(vec![DomainEvent::TrackChanged { track: id }])
    }

    /// Replaces the instrument and moves every note on string `s` by `fret_deltas[s - 1]`
    /// frets, all or nothing. Every shifted note is validated against the new instrument
    /// (H-03, H-04, H-10). Ties stay valid (H-06) because tied notes share a string and
    /// therefore move by the same delta. `None` keeps all frets (same as
    /// `set_instrument`). The deltas come from the Guitar context's retune planning.
    pub fn retune_track(
        &mut self,
        id: TrackId,
        instrument: InstrumentSpec,
        fret_deltas: Option<&[i16]>,
    ) -> Result<Events, DomainError> {
        let Some(deltas) = fret_deltas else {
            return self.set_instrument(id, instrument);
        };
        let t = self.track_index(id)?;
        let InstrumentSpec::Fretted(inst) = &instrument;
        check_instrument(inst).map_err(|m| DomainError::invariant("H-10", m))?;
        let delta_for = |note: &Note| -> Result<i16, DomainError> {
            let s = note.position().string.get();
            deltas.get(s as usize - 1).copied().ok_or_else(|| {
                DomainError::invariant("H-03", format!("no fret delta for string {s}"))
            })
        };
        for (b, bar) in self.tracks[t].bars.iter().enumerate() {
            for voice in &bar.voices {
                for beat in &voice.beats {
                    for note in &beat.notes {
                        let shifted = shifted_note(note, delta_for(note)?).ok_or_else(|| {
                            DomainError::invariant(
                                "H-04",
                                format!(
                                    "bar {b}, note {}: fret below 0 or above 255",
                                    note.id.raw()
                                ),
                            )
                        })?;
                        check_note(inst, &shifted).map_err(|(rule, m)| {
                            DomainError::invariant(
                                rule,
                                format!("bar {b}, note {}: {m}", note.id.raw()),
                            )
                        })?;
                    }
                }
            }
        }
        let bar_count = self.master_bars.len();
        let track = Arc::make_mut(&mut self.tracks[t]);
        track.instrument = instrument;
        for bar in track.bars.iter_mut() {
            if bar
                .voices
                .iter()
                .all(|v| v.beats.iter().all(|b| b.notes.is_empty()))
            {
                continue;
            }
            for voice in Arc::make_mut(bar).voices.iter_mut() {
                for beat in voice.beats.iter_mut() {
                    for note in beat.notes.iter_mut() {
                        let d = deltas[note.position().string.get() as usize - 1];
                        *note = shifted_note(note, d).expect("checked above");
                    }
                }
            }
        }
        Ok(vec![
            DomainEvent::TrackChanged { track: id },
            DomainEvent::BarContentChanged {
                track: id,
                bars: 0..bar_count,
            },
        ])
    }

    pub fn move_track(&mut self, id: TrackId, to: usize) -> Result<Events, DomainError> {
        let t = self.track_index(id)?;
        if to >= self.tracks.len() {
            return Err(DomainError::InvalidPosition(format!("track index {to}")));
        }
        let track = self.tracks.remove(t);
        self.tracks.insert(to, track);
        Ok(vec![DomainEvent::TracksReordered])
    }

    // ------------------------------------------------------------- master bars

    pub fn insert_master_bars(
        &mut self,
        at: usize,
        count: usize,
        template: MasterBar,
    ) -> Result<Events, DomainError> {
        let len = self.master_bars.len();
        if at > len || count == 0 {
            return Err(DomainError::InvalidPosition(format!(
                "insert {count} bars at {at} of {len}"
            )));
        }
        let mut candidate = self.master_bars.as_ref().clone();
        candidate.splice(at..at, std::iter::repeat_n(template, count));
        check_repeats(&candidate)?;

        self.master_bars = Arc::new(candidate);
        let mut tm = self.tempo_map.as_ref().clone();
        for change in tm.iter_mut() {
            let initial = change.bar == 0 && change.offset.is_zero();
            if change.bar >= at && !initial {
                change.bar += count;
            }
        }
        self.tempo_map = Arc::new(tm);
        let empty = Arc::new(BarContent::empty());
        for track in self.tracks.iter_mut() {
            let track = Arc::make_mut(track);
            track
                .bars
                .splice(at..at, (0..count).map(|_| Arc::clone(&empty)));
        }
        let mut events = vec![
            DomainEvent::MasterBarsInserted { at, count },
            DomainEvent::TempoMapChanged,
        ];
        for t in 0..self.tracks.len() {
            events.extend(self.ties_removed_event(t, at + count..at + count + 1));
        }
        Ok(events)
    }

    pub fn remove_master_bars(&mut self, at: usize, count: usize) -> Result<Events, DomainError> {
        let len = self.master_bars.len();
        if count == 0 || at + count > len {
            return Err(DomainError::InvalidPosition(format!(
                "remove {count} bars at {at} of {len}"
            )));
        }
        if count == len {
            return Err(DomainError::CannotRemoveLast("master bar"));
        }
        let mut candidate = self.master_bars.as_ref().clone();
        candidate.drain(at..at + count);
        check_repeats(&candidate)?;

        // The tempo heard at the first bar after the removed range is kept there.
        let tempo_after = self.tempo_in_effect(at + count, Rational::ZERO);
        let mut tm: Vec<TempoChange> = self
            .tempo_map
            .iter()
            .filter(|c| c.bar < at || c.bar >= at + count)
            .map(|c| {
                if c.bar >= at + count {
                    TempoChange {
                        bar: c.bar - count,
                        ..*c
                    }
                } else {
                    *c
                }
            })
            .collect();
        let new_len = len - count;
        if at < new_len {
            let current = tempo_in_effect(&tm, at, Rational::ZERO);
            if current != Some(tempo_after.expect("map starts at 0")) || tm.is_empty() {
                let pos = tm.partition_point(|c| (c.bar, c.offset) < (at, Rational::ZERO));
                if tm
                    .get(pos)
                    .is_some_and(|c| c.bar == at && c.offset.is_zero())
                {
                    tm[pos].tempo = tempo_after.expect("map starts at 0");
                } else {
                    tm.insert(
                        pos,
                        TempoChange {
                            bar: at,
                            offset: Rational::ZERO,
                            tempo: tempo_after.expect("map starts at 0"),
                        },
                    );
                }
            }
        }
        self.master_bars = Arc::new(candidate);
        self.tempo_map = Arc::new(tm);
        for track in self.tracks.iter_mut() {
            Arc::make_mut(track).bars.drain(at..at + count);
        }
        let mut events = vec![
            DomainEvent::MasterBarsRemoved { at, count },
            DomainEvent::TempoMapChanged,
        ];
        if at < new_len {
            for t in 0..self.tracks.len() {
                events.extend(self.ties_removed_event(t, at..at + 1));
            }
        }
        Ok(events)
    }

    /// Replaces the properties of one master bar (time signature, key, repeats, marker).
    pub fn set_master_bar(&mut self, index: usize, bar: MasterBar) -> Result<Events, DomainError> {
        if index >= self.master_bars.len() {
            return Err(DomainError::NotFound(format!("master bar {index}")));
        }
        if let Some(c) = self
            .tempo_map
            .iter()
            .find(|c| c.bar == index && !tempo_offset_valid(&bar, c.offset))
        {
            return Err(DomainError::invariant(
                "H-11",
                format!(
                    "tempo change at offset {} would fall outside the bar",
                    c.offset
                ),
            ));
        }
        let mut candidate = self.master_bars.as_ref().clone();
        candidate[index] = bar;
        check_repeats(&candidate)?;
        self.master_bars = Arc::new(candidate);
        Ok(vec![DomainEvent::MasterBarsChanged {
            bars: index..index + 1,
        }])
    }

    /// Inserts or replaces the tempo change at a position.
    pub fn set_tempo(
        &mut self,
        bar: usize,
        offset: Rational,
        tempo: Tempo,
    ) -> Result<Events, DomainError> {
        let mb = self
            .master_bars
            .get(bar)
            .ok_or_else(|| DomainError::NotFound(format!("master bar {bar}")))?;
        if !tempo_offset_valid(mb, offset) {
            return Err(DomainError::invariant(
                "H-11",
                format!("offset {offset} outside bar {bar}"),
            ));
        }
        let tm = Arc::make_mut(&mut self.tempo_map);
        let pos = tm.partition_point(|c| (c.bar, c.offset) < (bar, offset));
        match tm.get_mut(pos) {
            Some(c) if c.bar == bar && c.offset == offset => c.tempo = tempo,
            _ => tm.insert(pos, TempoChange { bar, offset, tempo }),
        }
        Ok(vec![DomainEvent::TempoMapChanged])
    }

    pub fn remove_tempo(&mut self, bar: usize, offset: Rational) -> Result<Events, DomainError> {
        if bar == 0 && offset.is_zero() {
            return Err(DomainError::invariant(
                "H-11",
                "the initial tempo cannot be removed",
            ));
        }
        let pos = self
            .tempo_map
            .iter()
            .position(|c| c.bar == bar && c.offset == offset)
            .ok_or_else(|| {
                DomainError::NotFound(format!("tempo change at bar {bar}, offset {offset}"))
            })?;
        Arc::make_mut(&mut self.tempo_map).remove(pos);
        Ok(vec![DomainEvent::TempoMapChanged])
    }

    /// Tempo in effect at a position (last change at or before it).
    pub fn tempo_in_effect(&self, bar: usize, offset: Rational) -> Option<Tempo> {
        tempo_in_effect(&self.tempo_map, bar, offset)
    }

    // ------------------------------------------------------------------- beats

    /// Inserts a rest beat at `index` in the voice and returns its id.
    pub fn insert_beat(
        &mut self,
        track: TrackId,
        bar: usize,
        voice: usize,
        index: usize,
        value: NoteValue,
    ) -> Result<(BeatId, Events), DomainError> {
        let t = self.track_index(track)?;
        let v = self.voice_ref(t, bar, voice)?;
        if index > v.beats.len() {
            return Err(DomainError::InvalidPosition(format!(
                "beat index {index} of {}",
                v.beats.len()
            )));
        }
        let id = BeatId::from_raw(self.allocate_id());
        self.bar_mut(t, bar).voices[voice].beats.insert(
            index,
            Beat {
                id,
                value,
                notes: Vec::new(),
                stroke: None,
            },
        );
        Ok((id, self.content_events(t, bar..bar + 1)))
    }

    pub fn remove_beat(&mut self, r: BeatRef) -> Result<Events, DomainError> {
        let l = self.locate(r)?;
        self.bar_mut(l.track, l.bar).voices[l.voice]
            .beats
            .remove(l.beat);
        Ok(self.content_events(l.track, l.bar..l.bar + 1))
    }

    pub fn set_beat_value(&mut self, r: BeatRef, value: NoteValue) -> Result<Events, DomainError> {
        let l = self.locate(r)?;
        self.bar_mut(l.track, l.bar).voices[l.voice].beats[l.beat].value = value;
        Ok(self.content_events(l.track, l.bar..l.bar + 1))
    }

    /// Sets or clears the stroke direction of a beat (ADR-035). Allowed on any beat,
    /// rests included; it changes no timing.
    pub fn set_beat_stroke(
        &mut self,
        r: BeatRef,
        stroke: Option<Stroke>,
    ) -> Result<Events, DomainError> {
        let l = self.locate(r)?;
        self.bar_mut(l.track, l.bar).voices[l.voice].beats[l.beat].stroke = stroke;
        Ok(self.content_events(l.track, l.bar..l.bar + 1))
    }

    // ------------------------------------------------------------------- notes

    /// Sets the fret on a string in a beat: replaces the note on that string (keeping
    /// its id and effects) or inserts a new one. One note per string per beat (H-05)
    /// holds by construction.
    pub fn set_note(
        &mut self,
        r: BeatRef,
        position: FretPosition,
    ) -> Result<(NoteId, Events), DomainError> {
        let l = self.locate(r)?;
        let inst = self.tracks[l.track].fretted();
        let probe = Note {
            id: NoteId::from_raw(0),
            kind: NoteKind::Fretted(position),
            tie: false,
            effects: NoteEffects::default(),
        };
        check_note(inst, &probe).map_err(|(rule, m)| DomainError::invariant(rule, m))?;
        let existing = self
            .beat_at(&l)
            .notes
            .iter()
            .position(|n| n.position().string == position.string);
        let id = match existing {
            Some(i) => {
                let note = &mut self.bar_mut(l.track, l.bar).voices[l.voice].beats[l.beat].notes[i];
                note.kind = NoteKind::Fretted(position);
                note.id
            }
            None => {
                let id = NoteId::from_raw(self.allocate_id());
                let notes = &mut self.bar_mut(l.track, l.bar).voices[l.voice].beats[l.beat].notes;
                let pos = notes.partition_point(|n| n.position().string < position.string);
                notes.insert(
                    pos,
                    Note {
                        id,
                        kind: NoteKind::Fretted(position),
                        tie: false,
                        effects: NoteEffects::default(),
                    },
                );
                id
            }
        };
        Ok((id, self.content_events(l.track, l.bar..l.bar + 1)))
    }

    pub fn remove_note(&mut self, r: BeatRef, note: NoteId) -> Result<Events, DomainError> {
        let l = self.locate(r)?;
        let i = self.note_index(&l, note)?;
        self.bar_mut(l.track, l.bar).voices[l.voice].beats[l.beat]
            .notes
            .remove(i);
        Ok(self.content_events(l.track, l.bar..l.bar + 1))
    }

    /// Sets or clears the tie from the preceding note. Setting an invalid tie is
    /// rejected (H-06) rather than silently dropped.
    pub fn set_tie(&mut self, r: BeatRef, note: NoteId, tie: bool) -> Result<Events, DomainError> {
        let l = self.locate(r)?;
        let i = self.note_index(&l, note)?;
        if tie
            && !tie_is_valid(
                &self.tracks[l.track],
                l.bar,
                l.voice,
                l.beat,
                &self.beat_at(&l).notes[i],
            )
        {
            return Err(DomainError::invariant(
                "H-06",
                "no note with the same string and fret in the preceding beat",
            ));
        }
        self.bar_mut(l.track, l.bar).voices[l.voice].beats[l.beat].notes[i].tie = tie;
        Ok(self.content_events(l.track, l.bar..l.bar + 1))
    }

    pub fn set_note_effects(
        &mut self,
        r: BeatRef,
        note: NoteId,
        effects: NoteEffects,
    ) -> Result<Events, DomainError> {
        let l = self.locate(r)?;
        let i = self.note_index(&l, note)?;
        self.bar_mut(l.track, l.bar).voices[l.voice].beats[l.beat].notes[i].effects = effects;
        Ok(self.content_events(l.track, l.bar..l.bar + 1))
    }

    // ------------------------------------------------------------------ voices

    pub fn add_voice(
        &mut self,
        track: TrackId,
        bar: usize,
    ) -> Result<(usize, Events), DomainError> {
        let t = self.track_index(track)?;
        self.bar_ref(t, bar)?;
        let voices = &mut self.bar_mut(t, bar).voices;
        voices.push(Voice::default());
        let index = voices.len() - 1;
        Ok((index, self.content_events(t, bar..bar + 1)))
    }

    pub fn remove_voice(
        &mut self,
        track: TrackId,
        bar: usize,
        voice: usize,
    ) -> Result<Events, DomainError> {
        let t = self.track_index(track)?;
        let b = self.bar_ref(t, bar)?;
        if voice >= b.voices.len() {
            return Err(DomainError::NotFound(format!("voice {voice} in bar {bar}")));
        }
        if b.voices.len() == 1 {
            return Err(DomainError::invariant(
                "H-09",
                "a bar keeps at least one voice",
            ));
        }
        self.bar_mut(t, bar).voices.remove(voice);
        Ok(self.content_events(t, bar..bar + 1))
    }

    // -------------------------------------------------------------------- bulk

    /// Moves every note in the bar range by `delta` frets, all or nothing (H-04, H-10).
    pub fn shift_frets(
        &mut self,
        track: TrackId,
        bars: Range<usize>,
        delta: i16,
    ) -> Result<Events, DomainError> {
        let t = self.track_index(track)?;
        if bars.start >= bars.end || bars.end > self.master_bars.len() {
            return Err(DomainError::InvalidPosition(format!("bar range {bars:?}")));
        }
        let inst = self.tracks[t].fretted().clone();
        for b in bars.clone() {
            for voice in &self.tracks[t].bars[b].voices {
                for beat in &voice.beats {
                    for note in &beat.notes {
                        let shifted = shifted_note(note, delta).ok_or_else(|| {
                            DomainError::invariant(
                                "H-04",
                                format!("bar {b}: fret below 0 or above 255"),
                            )
                        })?;
                        check_note(&inst, &shifted).map_err(|(rule, m)| {
                            DomainError::invariant(rule, format!("bar {b}: {m}"))
                        })?;
                    }
                }
            }
        }
        for b in bars.clone() {
            for voice in self.bar_mut(t, b).voices.iter_mut() {
                for beat in voice.beats.iter_mut() {
                    for note in beat.notes.iter_mut() {
                        *note = shifted_note(note, delta).expect("checked above");
                    }
                }
            }
        }
        Ok(self.content_events(t, bars))
    }

    // --------------------------------------------------------------- beat runs

    /// Copies the beats of one voice from `start` to `end` (both inclusive, in written
    /// order, possibly across bars) as plain values without ids (ADR-030).
    pub fn copy_beats(
        &self,
        track: TrackId,
        voice: usize,
        start: BeatSlot,
        end: BeatSlot,
    ) -> Result<Vec<FragmentBeat>, DomainError> {
        let t = self.track_index(track)?;
        let runs = self.beat_run(t, voice, start, end)?;
        let mut out = Vec::new();
        for (bar, range) in runs {
            for beat in &self.tracks[t].bars[bar].voices[voice].beats[range] {
                out.push(FragmentBeat {
                    value: beat.value,
                    notes: beat
                        .notes
                        .iter()
                        .map(|n| FragmentNote {
                            position: n.position(),
                            tie: n.tie,
                            effects: n.effects,
                        })
                        .collect(),
                    stroke: beat.stroke,
                });
            }
        }
        Ok(out)
    }

    /// Inserts a copied run of beats at `index` of a voice, all or nothing. Strings and
    /// frets are kept as given, not pitches, so pasting into another tuning changes the
    /// sound. Every note is checked against the target track (H-03, H-04, H-05, H-10);
    /// ties whose preceding note is missing after the paste are cleared and reported
    /// (H-06). Bar capacity is a diagnostic and never blocks (Q-09). Never creates bars.
    pub fn paste_beats(
        &mut self,
        track: TrackId,
        bar: usize,
        voice: usize,
        index: usize,
        beats: &[FragmentBeat],
    ) -> Result<Events, DomainError> {
        let t = self.track_index(track)?;
        if beats.is_empty() {
            return Err(DomainError::InvalidPosition("nothing to paste".into()));
        }
        let len = self.voice_ref(t, bar, voice)?.beats.len();
        if index > len {
            return Err(DomainError::InvalidPosition(format!(
                "beat index {index} of {len}"
            )));
        }
        let inst = self.tracks[t].fretted().clone();
        for (k, beat) in beats.iter().enumerate() {
            let mut strings = Vec::with_capacity(beat.notes.len());
            for n in &beat.notes {
                let probe = Note {
                    id: NoteId::from_raw(0),
                    kind: NoteKind::Fretted(n.position),
                    tie: false,
                    effects: n.effects,
                };
                check_note(&inst, &probe).map_err(|(rule, m)| {
                    DomainError::invariant(rule, format!("pasted beat {k}: {m}"))
                })?;
                if strings.contains(&n.position.string) {
                    return Err(DomainError::invariant(
                        "H-05",
                        format!(
                            "pasted beat {k}: two notes on string {}",
                            n.position.string.get()
                        ),
                    ));
                }
                strings.push(n.position.string);
            }
        }
        // Everything is valid: from here on the paste cannot fail.
        let mut built = Vec::with_capacity(beats.len());
        for beat in beats {
            let id = BeatId::from_raw(self.allocate_id());
            let mut notes = Vec::with_capacity(beat.notes.len());
            for n in &beat.notes {
                notes.push(Note {
                    id: NoteId::from_raw(self.allocate_id()),
                    kind: NoteKind::Fretted(n.position),
                    tie: n.tie,
                    effects: n.effects,
                });
            }
            notes.sort_by_key(|n| n.position().string.get());
            built.push(Beat {
                id,
                value: beat.value,
                notes,
                stroke: beat.stroke,
            });
        }
        let target = &mut self.bar_mut(t, bar).voices[voice].beats;
        drop(target.splice(index..index, built));
        Ok(self.content_events(t, bar..bar + 1))
    }

    /// Removes the beats of one voice from `start` to `end` (both inclusive, possibly
    /// across bars), all or nothing. Bars keep their remaining beats; no bar is removed.
    /// Ties that lose their preceding note are cleared and reported (H-06).
    pub fn remove_beats(
        &mut self,
        track: TrackId,
        voice: usize,
        start: BeatSlot,
        end: BeatSlot,
    ) -> Result<Events, DomainError> {
        let t = self.track_index(track)?;
        let runs = self.beat_run(t, voice, start, end)?;
        for (bar, range) in runs {
            drop(self.bar_mut(t, bar).voices[voice].beats.drain(range));
        }
        Ok(self.content_events(t, start.bar..end.bar + 1))
    }

    /// Resolves an inclusive run of beats in one voice into `(bar, index range)` pieces.
    /// Both ends must exist, and the voice must exist in every bar of the run.
    fn beat_run(
        &self,
        t: usize,
        voice: usize,
        start: BeatSlot,
        end: BeatSlot,
    ) -> Result<Vec<(usize, Range<usize>)>, DomainError> {
        if (start.bar, start.index) > (end.bar, end.index) || end.bar >= self.master_bars.len() {
            return Err(DomainError::InvalidPosition(format!(
                "beat run {start:?} to {end:?}"
            )));
        }
        let mut runs = Vec::new();
        for bar in start.bar..=end.bar {
            let len = self.voice_ref(t, bar, voice)?.beats.len();
            if bar == start.bar && start.index >= len {
                return Err(DomainError::InvalidPosition(format!(
                    "beat {} of {len} in bar {bar}",
                    start.index
                )));
            }
            if bar == end.bar && end.index >= len {
                return Err(DomainError::InvalidPosition(format!(
                    "beat {} of {len} in bar {bar}",
                    end.index
                )));
            }
            let from = if bar == start.bar { start.index } else { 0 };
            let to = if bar == end.bar { end.index + 1 } else { len };
            runs.push((bar, from..to));
        }
        Ok(runs)
    }

    // ----------------------------------------------------------------- helpers

    fn bar_mut(&mut self, t: usize, bar: usize) -> &mut BarContent {
        let track = Arc::make_mut(&mut self.tracks[t]);
        Arc::make_mut(&mut track.bars[bar])
    }

    fn bar_ref(&self, t: usize, bar: usize) -> Result<&BarContent, DomainError> {
        self.tracks[t]
            .bars
            .get(bar)
            .map(|b| b.as_ref())
            .ok_or_else(|| DomainError::NotFound(format!("bar {bar}")))
    }

    fn voice_ref(&self, t: usize, bar: usize, voice: usize) -> Result<&Voice, DomainError> {
        self.bar_ref(t, bar)?
            .voices
            .get(voice)
            .ok_or_else(|| DomainError::NotFound(format!("voice {voice} in bar {bar}")))
    }

    fn locate(&self, r: BeatRef) -> Result<Located, DomainError> {
        let t = self.track_index(r.track)?;
        let v = self.voice_ref(t, r.bar, r.voice)?;
        let beat = v.beats.iter().position(|b| b.id == r.beat).ok_or_else(|| {
            DomainError::NotFound(format!(
                "beat {} in bar {}, voice {}",
                r.beat.raw(),
                r.bar,
                r.voice
            ))
        })?;
        Ok(Located {
            track: t,
            bar: r.bar,
            voice: r.voice,
            beat,
        })
    }

    fn beat_at(&self, l: &Located) -> &Beat {
        &self.tracks[l.track].bars[l.bar].voices[l.voice].beats[l.beat]
    }

    fn note_index(&self, l: &Located, note: NoteId) -> Result<usize, DomainError> {
        self.beat_at(l)
            .notes
            .iter()
            .position(|n| n.id == note)
            .ok_or_else(|| DomainError::NotFound(format!("note {}", note.raw())))
    }

    /// H-06 maintenance: clears ties in `bars` and the bar after it whose preceding
    /// note no longer matches. Returns the ids of the notes that lost their tie.
    fn normalize_ties(&mut self, t: usize, bars: Range<usize>) -> Vec<NoteId> {
        let len = self.master_bars.len();
        let end = (bars.end + 1).min(len);
        let mut fixes = Vec::new();
        {
            let track = &self.tracks[t];
            for b in bars.start.min(len)..end {
                for (vi, voice) in track.bars[b].voices.iter().enumerate() {
                    for (k, beat) in voice.beats.iter().enumerate() {
                        for (ni, note) in beat.notes.iter().enumerate() {
                            if note.tie && !tie_is_valid(track, b, vi, k, note) {
                                fixes.push((b, vi, k, ni, note.id));
                            }
                        }
                    }
                }
            }
        }
        for &(b, vi, k, ni, _) in &fixes {
            self.bar_mut(t, b).voices[vi].beats[k].notes[ni].tie = false;
        }
        fixes.into_iter().map(|f| f.4).collect()
    }

    fn ties_removed_event(&mut self, t: usize, bars: Range<usize>) -> Option<DomainEvent> {
        let removed = self.normalize_ties(t, bars);
        (!removed.is_empty()).then(|| DomainEvent::TiesRemoved {
            track: self.tracks[t].id,
            notes: removed,
        })
    }

    fn content_events(&mut self, t: usize, bars: Range<usize>) -> Events {
        let id = self.tracks[t].id;
        let removed = self.normalize_ties(t, bars.clone());
        let end = if removed.is_empty() {
            bars.end
        } else {
            (bars.end + 1).min(self.master_bars.len())
        };
        let mut events = vec![DomainEvent::BarContentChanged {
            track: id,
            bars: bars.start..end,
        }];
        if !removed.is_empty() {
            events.push(DomainEvent::TiesRemoved {
                track: id,
                notes: removed,
            });
        }
        events
    }
}

fn check_repeats(bars: &[MasterBar]) -> Result<(), DomainError> {
    match repeats::check(bars).into_iter().next() {
        Some(v) => Err(DomainError::invariant("H-12", v.to_string())),
        None => Ok(()),
    }
}

fn tempo_in_effect(map: &[TempoChange], bar: usize, offset: Rational) -> Option<Tempo> {
    map.iter()
        .take_while(|c| (c.bar, c.offset) <= (bar, offset))
        .last()
        .map(|c| c.tempo)
}

fn shifted_note(note: &Note, delta: i16) -> Option<Note> {
    let p = note.position();
    let fret = u8::try_from(p.fret.0 as i16 + delta).ok()?;
    Some(Note {
        kind: NoteKind::Fretted(FretPosition {
            string: p.string,
            fret: Fret(fret),
        }),
        ..note.clone()
    })
}
