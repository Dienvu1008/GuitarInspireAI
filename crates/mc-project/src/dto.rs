//! Persistence DTOs for format 1.x. These are the stored shape, not the domain model.
//!
//! U1 (project-format section 9): every struct, at every level, captures unknown keys
//! in `extra`. Defaults are skipped on output to keep files small; missing optional
//! fields read back as defaults. Field order is fixed, `extra` is a `BTreeMap`, and no
//! floats are used, so output is deterministic.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Extra = BTreeMap<String, serde_json::Value>;

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_zero(n: &u8) -> bool {
    *n == 0
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct ScoreDto {
    pub master_bars: Vec<MasterBarDto>,
    pub tempo_map: Vec<TempoChangeDto>,
    pub tracks: Vec<TrackDto>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct MasterBarDto {
    /// [numerator, denominator]
    pub time_signature: [u8; 2],
    pub key: KeyDto,
    #[serde(default, skip_serializing_if = "is_false")]
    pub pickup: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub repeat_start: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_end: Option<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternate_endings: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModeDto {
    Major,
    Minor,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct KeyDto {
    pub fifths: i8,
    pub mode: ModeDto,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct TempoChangeDto {
    pub bar: u32,
    /// Offset in the bar as [numerator, denominator] of a whole note.
    pub offset: [i64; 2],
    /// Quarter-note beats per minute as [numerator, denominator].
    pub bpm: [i64; 2],
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct TrackDto {
    pub id: u64,
    pub name: String,
    pub instrument: InstrumentDto,
    pub bars: Vec<BarDto>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Format 1.x stores fretted instruments only. Other instrument kinds will be a
/// required feature (old readers must refuse them, not misread them).
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstrumentDto {
    /// Open-string pitches, string 1 (highest) first, for example "E4".
    pub tuning: Vec<String>,
    pub fret_count: u8,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub capo: u8,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct BarDto {
    pub voices: Vec<VoiceDto>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct VoiceDto {
    pub beats: Vec<BeatDto>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct BeatDto {
    pub id: u64,
    pub value: ValueDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<NoteDto>,
    /// Stroke direction (format 1.1, ADR-035). Omitted when not marked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<StrokeDto>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Stroke direction of a beat (format 1.1, ADR-035).
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StrokeDto {
    Down,
    Up,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct ValueDto {
    /// Power of two: 0 = whole ... 8 = 1/256.
    pub base: u8,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub dots: u8,
    /// [actual, normal], for example [3, 2] for a triplet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tuplet: Option<[u8; 2]>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct NoteDto {
    pub id: u64,
    pub string: u8,
    pub fret: u8,
    #[serde(default, skip_serializing_if = "is_false")]
    pub tie: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub let_ring: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub dead: bool,
    /// Format 1.1 (ADR-035). Each is omitted when off.
    #[serde(default, skip_serializing_if = "is_false")]
    pub accent: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub palm_mute: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub string_percussive: bool,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct ManifestDto {
    pub format: String,
    pub format_version: VersionDto,
    #[serde(default)]
    pub required_features: Vec<String>,
    #[serde(default)]
    pub optional_features: Vec<String>,
    pub project_id: String,
    pub created_by: AppDto,
    pub saved_by: AppDto,
    pub saved_revision: u64,
    pub entries: Vec<EntryDto>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionDto {
    pub major: u32,
    pub minor: u32,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct AppDto {
    pub app: String,
    pub version: String,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct EntryDto {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    #[serde(flatten)]
    pub extra: Extra,
}

/// U1: JSON paths of every unknown key in a DTO tree.
pub trait UnknownContent {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>);
}

fn extra_paths(extra: &Extra, path: &str, out: &mut Vec<String>) {
    out.extend(extra.keys().map(|k| format!("{path}.{k}")));
}

macro_rules! leaf {
    ($t:ty) => {
        impl UnknownContent for $t {
            fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
                extra_paths(&self.extra, path, out);
            }
        }
    };
}

leaf!(KeyDto);
leaf!(TempoChangeDto);
leaf!(InstrumentDto);
leaf!(ValueDto);
leaf!(NoteDto);
leaf!(AppDto);
leaf!(EntryDto);

fn list<T: UnknownContent>(items: &[T], path: &str, field: &str, out: &mut Vec<String>) {
    for (i, item) in items.iter().enumerate() {
        item.unknown_paths(&format!("{path}.{field}[{i}]"), out);
    }
}

impl UnknownContent for MasterBarDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        self.key.unknown_paths(&format!("{path}.key"), out);
    }
}

impl UnknownContent for BeatDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        self.value.unknown_paths(&format!("{path}.value"), out);
        list(&self.notes, path, "notes", out);
    }
}

impl UnknownContent for VoiceDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        list(&self.beats, path, "beats", out);
    }
}

impl UnknownContent for BarDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        list(&self.voices, path, "voices", out);
    }
}

impl UnknownContent for TrackDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        self.instrument
            .unknown_paths(&format!("{path}.instrument"), out);
        list(&self.bars, path, "bars", out);
    }
}

impl UnknownContent for ScoreDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        list(&self.master_bars, path, "master_bars", out);
        list(&self.tempo_map, path, "tempo_map", out);
        list(&self.tracks, path, "tracks", out);
    }
}

impl UnknownContent for ManifestDto {
    fn unknown_paths(&self, path: &str, out: &mut Vec<String>) {
        extra_paths(&self.extra, path, out);
        self.created_by
            .unknown_paths(&format!("{path}.created_by"), out);
        self.saved_by
            .unknown_paths(&format!("{path}.saved_by"), out);
        list(&self.entries, path, "entries", out);
    }
}
