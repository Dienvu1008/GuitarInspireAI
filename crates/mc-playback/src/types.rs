//! Value objects for the performance timeline (ADR-024). Each newtype validates its
//! range in its constructor and keeps its field private, so an out-of-range MIDI note,
//! channel, velocity, program or tempo factor cannot be built from outside this crate.

use mc_theory::SpelledPitch;

/// A value fell outside the allowed range of a timeline newtype.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutOfRange {
    /// Which type rejected the value, for example "MidiNote".
    pub kind: &'static str,
    pub value: i64,
}

impl std::fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} out of range: {}", self.kind, self.value)
    }
}

impl std::error::Error for OutOfRange {}

/// MIDI key number, 0..=127 (domain-invariants section 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MidiNote(u8);

impl MidiNote {
    pub fn try_new(v: u8) -> Result<Self, OutOfRange> {
        if v <= 127 {
            Ok(MidiNote(v))
        } else {
            Err(OutOfRange {
                kind: "MidiNote",
                value: v as i64,
            })
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

/// The sounding pitch of `p` as a MIDI note. `SpelledPitch::semitone` already follows
/// MIDI numbering (C-1 = 0, C4 = 60, A4 = 69). Pitches outside 0..=127 are rejected
/// (D-05); the caller turns that into a warning and omits the note.
impl TryFrom<SpelledPitch> for MidiNote {
    type Error = OutOfRange;

    fn try_from(p: SpelledPitch) -> Result<Self, OutOfRange> {
        let midi = p.semitone();
        u8::try_from(midi)
            .ok()
            .and_then(|v| Self::try_new(v).ok())
            .ok_or(OutOfRange {
                kind: "MidiNote",
                value: midi as i64,
            })
    }
}

/// MIDI channel 0..=15, never 9 (channel 10 is General MIDI percussion; ADR-024 item 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Channel(u8);

impl Channel {
    pub fn try_new(v: u8) -> Result<Self, OutOfRange> {
        if v <= 15 && v != 9 {
            Ok(Channel(v))
        } else {
            Err(OutOfRange {
                kind: "Channel",
                value: v as i64,
            })
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

/// MIDI velocity 1..=127. Fixed values are used (ADR-024): [`Velocity::NORMAL`] for
/// every note, [`Velocity::DEAD`] for dead notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Velocity(u8);

impl Velocity {
    pub const NORMAL: Velocity = Velocity(96);
    pub const DEAD: Velocity = Velocity(40);

    pub fn try_new(v: u8) -> Result<Self, OutOfRange> {
        if (1..=127).contains(&v) {
            Ok(Velocity(v))
        } else {
            Err(OutOfRange {
                kind: "Velocity",
                value: v as i64,
            })
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

/// General MIDI program number, 0..=127. Playback uses one program (ADR-024).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GmProgram(u8);

impl GmProgram {
    /// General MIDI "Acoustic Guitar (steel)" (0-based 25; GM number 26).
    pub const ACOUSTIC_GUITAR_STEEL: GmProgram = GmProgram(25);

    pub fn try_new(v: u8) -> Result<Self, OutOfRange> {
        if v <= 127 {
            Ok(GmProgram(v))
        } else {
            Err(OutOfRange {
                kind: "GmProgram",
                value: v as i64,
            })
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

/// User tempo factor in hundredths, 25..=200 in steps of 5 (Q6, 0.25x..=2.0x). It is a
/// render-time scalar applied to tick-to-sample conversion, not part of the timeline,
/// so changing it never recompiles (audio-dsp-architecture section 3 item 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TempoFactor(u16);

impl TempoFactor {
    pub const NORMAL: TempoFactor = TempoFactor(100);

    pub fn new(hundredths: u16) -> Result<Self, OutOfRange> {
        if (25..=200).contains(&hundredths) && hundredths.is_multiple_of(5) {
            Ok(TempoFactor(hundredths))
        } else {
            Err(OutOfRange {
                kind: "TempoFactor",
                value: hundredths as i64,
            })
        }
    }

    pub const fn hundredths(self) -> u16 {
        self.0
    }
}

impl Default for TempoFactor {
    fn default() -> Self {
        TempoFactor::NORMAL
    }
}

/// An integer tick on the performance timeline (a fraction `1/ticks_per_whole` of a
/// whole note). Non-negative in a compiled timeline. Maps to Dart `int` (i64).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick(pub i64);

/// Ticks per whole note: the resolution of a timeline (audio-dsp section 3a rule 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TickRate(u32);

impl TickRate {
    pub fn new(ticks_per_whole: u32) -> Self {
        TickRate(ticks_per_whole.max(1))
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_theory::SpelledPitch;
    use std::str::FromStr;

    #[test]
    fn newtypes_reject_out_of_range() {
        assert!(MidiNote::try_new(128).is_err());
        assert_eq!(MidiNote::try_new(127).unwrap().get(), 127);
        assert!(Channel::try_new(9).is_err(), "channel 10 is percussion");
        assert!(Channel::try_new(16).is_err());
        assert_eq!(Channel::try_new(15).unwrap().get(), 15);
        assert!(Velocity::try_new(0).is_err());
        assert!(GmProgram::try_new(128).is_err());
    }

    #[test]
    fn tempo_factor_is_a_multiple_of_five_in_range() {
        assert!(TempoFactor::new(24).is_err());
        assert!(TempoFactor::new(201).is_err());
        assert!(TempoFactor::new(101).is_err(), "not a multiple of 5");
        assert_eq!(TempoFactor::new(25).unwrap().hundredths(), 25);
        assert_eq!(TempoFactor::NORMAL.hundredths(), 100);
    }

    #[test]
    fn midi_note_from_spelled_pitch() {
        assert_eq!(
            MidiNote::try_from(SpelledPitch::from_str("A4").unwrap())
                .unwrap()
                .get(),
            69
        );
        assert_eq!(
            MidiNote::try_from(SpelledPitch::from_str("C4").unwrap())
                .unwrap()
                .get(),
            60
        );
        // Low E of a guitar.
        assert_eq!(
            MidiNote::try_from(SpelledPitch::from_str("E2").unwrap())
                .unwrap()
                .get(),
            40
        );
    }

    #[test]
    fn midi_note_out_of_range_is_rejected() {
        // Below MIDI 0.
        let low = SpelledPitch::new(mc_theory::Letter::C, mc_theory::Alter::NATURAL, -2);
        assert!(MidiNote::try_from(low).is_err());
    }
}
