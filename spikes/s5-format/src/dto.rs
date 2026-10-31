//! Persistence DTO prototypes. `full` captures unknown keys at every level (U1);
//! `plain` is identical without capture, to measure the cost of `flatten`.

use serde_json::Value;
use std::collections::BTreeMap;

pub type Extra = BTreeMap<String, Value>;

pub mod full {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Score {
        pub master_bars: Vec<MasterBar>,
        pub tracks: Vec<Track>,
        #[serde(flatten)]
        pub extra: Extra,
    }
    #[derive(Serialize, Deserialize, Clone)]
    pub struct MasterBar {
        pub time_signature: [u8; 2],
        pub key: i8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tempo: Option<[u32; 2]>,
        #[serde(flatten)]
        pub extra: Extra,
    }
    #[derive(Serialize, Deserialize, Clone)]
    pub struct Track {
        pub id: String,
        pub name: String,
        pub tuning: Vec<String>,
        pub fret_count: u8,
        pub capo: u8,
        pub bars: Vec<Bar>,
        #[serde(flatten)]
        pub extra: Extra,
    }
    #[derive(Serialize, Deserialize, Clone)]
    pub struct Bar {
        pub voices: Vec<Voice>,
        #[serde(flatten)]
        pub extra: Extra,
    }
    #[derive(Serialize, Deserialize, Clone)]
    pub struct Voice {
        pub beats: Vec<Beat>,
        #[serde(flatten)]
        pub extra: Extra,
    }
    #[derive(Serialize, Deserialize, Clone)]
    pub struct Beat {
        pub id: String,
        pub duration: [u32; 2],
        pub dots: u8,
        pub notes: Vec<Note>,
        #[serde(flatten)]
        pub extra: Extra,
    }
    #[derive(Serialize, Deserialize, Clone)]
    pub struct Note {
        pub id: String,
        pub string: u8,
        pub fret: u8,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub effects: Vec<Effect>,
        #[serde(flatten)]
        pub extra: Extra,
    }

    /// Extensible enum, adjacently tagged: `{"kind": "...", "data": ...}`.
    /// Unknown kinds are captured with their raw JSON instead of failing.
    #[derive(Clone, Debug, PartialEq)]
    pub enum Effect {
        Tie,
        LetRing,
        Dead,
        Unknown { kind: String, raw: Value },
    }

    impl Serialize for Effect {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let v = match self {
                Effect::Tie => serde_json::json!({ "kind": "tie" }),
                Effect::LetRing => serde_json::json!({ "kind": "let_ring" }),
                Effect::Dead => serde_json::json!({ "kind": "dead" }),
                Effect::Unknown { raw, .. } => raw.clone(),
            };
            v.serialize(s)
        }
    }

    impl<'de> Deserialize<'de> for Effect {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let raw = Value::deserialize(d)?;
            let kind = raw
                .get("kind")
                .and_then(Value::as_str)
                .ok_or_else(|| serde::de::Error::custom("effect without kind"))?
                .to_string();
            Ok(match kind.as_str() {
                "tie" => Effect::Tie,
                "let_ring" => Effect::LetRing,
                "dead" => Effect::Dead,
                _ => Effect::Unknown { kind, raw },
            })
        }
    }

    /// U1: JSON paths of all unknown content in the tree.
    pub fn unknown_paths(s: &Score) -> Vec<String> {
        let mut out = Vec::new();
        let mut effects = Vec::new();
        let mut note = |path: &str, extra: &Extra| {
            for k in extra.keys() {
                out.push(format!("{path}.{k}"));
            }
        };
        note("$", &s.extra);
        for (i, m) in s.master_bars.iter().enumerate() {
            note(&format!("$.master_bars[{i}]"), &m.extra);
        }
        for (t, tr) in s.tracks.iter().enumerate() {
            let tp = format!("$.tracks[{t}]");
            note(&tp, &tr.extra);
            for (b, bar) in tr.bars.iter().enumerate() {
                let bp = format!("{tp}.bars[{b}]");
                note(&bp, &bar.extra);
                for (v, voice) in bar.voices.iter().enumerate() {
                    let vp = format!("{bp}.voices[{v}]");
                    note(&vp, &voice.extra);
                    for (k, beat) in voice.beats.iter().enumerate() {
                        let kp = format!("{vp}.beats[{k}]");
                        note(&kp, &beat.extra);
                        for (n, nt) in beat.notes.iter().enumerate() {
                            let np = format!("{kp}.notes[{n}]");
                            note(&np, &nt.extra);
                            for (e, eff) in nt.effects.iter().enumerate() {
                                if let Effect::Unknown { kind, .. } = eff {
                                    effects.push(format!("{np}.effects[{e}]:{kind}"));
                                }
                            }
                        }
                    }
                }
            }
        }
        out.extend(effects);
        out
    }
}

pub mod plain {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct Score {
        pub master_bars: Vec<MasterBar>,
        pub tracks: Vec<Track>,
    }
    #[derive(Deserialize)]
    pub struct MasterBar {
        pub time_signature: [u8; 2],
        pub key: i8,
        #[serde(default)]
        pub tempo: Option<[u32; 2]>,
    }
    #[derive(Deserialize)]
    pub struct Track {
        pub id: String,
        pub name: String,
        pub tuning: Vec<String>,
        pub fret_count: u8,
        pub capo: u8,
        pub bars: Vec<Bar>,
    }
    #[derive(Deserialize)]
    pub struct Bar {
        pub voices: Vec<Voice>,
    }
    #[derive(Deserialize)]
    pub struct Voice {
        pub beats: Vec<Beat>,
    }
    #[derive(Deserialize)]
    pub struct Beat {
        pub id: String,
        pub duration: [u32; 2],
        pub dots: u8,
        pub notes: Vec<Note>,
    }
    #[derive(Deserialize)]
    pub struct Note {
        pub id: String,
        pub string: u8,
        pub fret: u8,
        #[serde(default)]
        pub effects: Vec<serde_json::Value>,
    }
}

/// Deterministic fixture: `tracks` x `bars` bars, `beats` beats per bar, `notes` notes per beat.
pub fn fixture(tracks: usize, bars: usize, beats: usize, notes: usize) -> full::Score {
    let mut id = 0u64;
    let mut next_id = || {
        id += 1;
        format!("{id:026}")
    };
    let master_bars = (0..bars)
        .map(|i| full::MasterBar { time_signature: [4, 4], key: 0, tempo: (i == 0).then_some([120, 1]), extra: Extra::new() })
        .collect();
    let tracks = (0..tracks)
        .map(|t| full::Track {
            id: next_id(),
            name: format!("Track {}", t + 1),
            tuning: ["E4", "B3", "G3", "D3", "A2", "E2"].map(String::from).to_vec(),
            fret_count: 24,
            capo: 0,
            bars: (0..bars)
                .map(|b| full::Bar {
                    voices: vec![full::Voice {
                        beats: (0..beats)
                            .map(|k| full::Beat {
                                id: next_id(),
                                duration: [1, beats as u32],
                                dots: 0,
                                notes: (0..notes)
                                    .map(|n| full::Note {
                                        id: next_id(),
                                        string: (n % 6 + 1) as u8,
                                        fret: ((b + k + n) % 13) as u8,
                                        effects: if (b + k) % 17 == 0 { vec![full::Effect::LetRing] } else { vec![] },
                                        extra: Extra::new(),
                                    })
                                    .collect(),
                                extra: Extra::new(),
                            })
                            .collect(),
                        extra: Extra::new(),
                    }],
                    extra: Extra::new(),
                })
                .collect(),
            extra: Extra::new(),
        })
        .collect();
    full::Score { master_bars, tracks, extra: Extra::new() }
}
