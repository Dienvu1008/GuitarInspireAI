//! Native project format (`mc-project`), docs/architecture/project-format.md.
//!
//! Byte-oriented and file-system free (FF-03): `load` takes package bytes, `save`
//! returns package bytes. Atomic writes, backups and recovery belong to the file store
//! adapter in `mc-app`. The product extension `.giproj` (Q-08) is not part of this crate: a file is identified by its manifest `format` and `format_version`.

mod convert;
pub mod dto;
mod error;
mod package;
mod project_id;

pub use convert::{from_dto, to_dto};
pub use error::LoadError;
pub use package::{
    Limits, MANIFEST, RESERVED_PREFIXES, SCORE, check_name, read_entries, write_entries,
};
pub use project_id::ProjectId;

use dto::{AppDto, EntryDto, Extra, ManifestDto, ScoreDto, UnknownContent, VersionDto};
use mc_score::Score;
use sha2::{Digest, Sha256};

/// Format identifier stored in every manifest.
pub const FORMAT: &str = "guitarinspire.project";
/// Version written by this build. Bump minor for additive changes, major for
/// incompatible ones (project-format section 5; FF-32 enforces it).
/// 1.1 (ADR-035): optional note techniques (accent, palm mute, string-percussive) and
/// beat stroke direction.
pub const FORMAT_VERSION: FormatVersion = FormatVersion { major: 1, minor: 1 };
/// Required features this build understands. Unknown ones refuse the file (P-03).
pub const KNOWN_REQUIRED_FEATURES: &[&str] = &[];
/// Optional features this build understands. Unknown ones are unknown content (U1).
pub const KNOWN_OPTIONAL_FEATURES: &[&str] = &[];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FormatVersion {
    pub major: u32,
    pub minor: u32,
}

/// Who wrote a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppInfo {
    pub app: String,
    pub version: String,
}

/// Information the caller provides when saving.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveInfo {
    pub project_id: ProjectId,
    /// Preserved from the loaded file; for a new project, the same as `saved_by`.
    pub created_by: AppInfo,
    pub saved_by: AppInfo,
    pub saved_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadWarning {
    /// U1: content this build does not understand. Saving would drop it, so the
    /// application must refuse to overwrite the original (U2).
    UnknownContent { paths: Vec<String> },
    /// Written by a newer minor version of the same major format.
    NewerMinorVersion { found: FormatVersion },
}

#[derive(Debug, Clone)]
pub struct LoadedProject {
    pub score: Score,
    pub format_version: FormatVersion,
    pub project_id: String,
    pub created_by: AppInfo,
    pub saved_by: AppInfo,
    pub saved_revision: u64,
    pub warnings: Vec<LoadWarning>,
}

impl LoadedProject {
    /// U1 result: JSON paths and entry names of unknown content, if any.
    pub fn unknown_content(&self) -> &[String] {
        self.warnings
            .iter()
            .find_map(|w| match w {
                LoadWarning::UnknownContent { paths } => Some(paths.as_slice()),
                _ => None,
            })
            .unwrap_or(&[])
    }
}

/// JSON schemas of the persistence DTOs of the current format version, as
/// (name, pretty JSON). Snapshots of released versions are compared by FF-32.
pub fn schemas() -> [(&'static str, String); 2] {
    let pretty = |s: &schemars::Schema| serde_json::to_string_pretty(s).expect("schemas serialize");
    [
        ("score", pretty(&schemars::schema_for!(ScoreDto))),
        ("manifest", pretty(&schemars::schema_for!(ManifestDto))),
    ]
}

/// Lowercase hex SHA-256.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Deterministic compact JSON of the score (no floats, fixed field order).
pub fn score_json(score: &Score) -> Vec<u8> {
    serde_json::to_vec(&to_dto(score)).expect("DTOs serialize infallibly")
}

/// Serializes a score into a package. Never fails for a valid `Score`.
pub fn save(score: &Score, info: &SaveInfo) -> Vec<u8> {
    let score_bytes = score_json(score);
    let app = |a: &AppInfo| AppDto {
        app: a.app.clone(),
        version: a.version.clone(),
        extra: Extra::new(),
    };
    let manifest = ManifestDto {
        format: FORMAT.to_string(),
        format_version: VersionDto {
            major: FORMAT_VERSION.major,
            minor: FORMAT_VERSION.minor,
        },
        required_features: Vec::new(),
        optional_features: Vec::new(),
        project_id: info.project_id.as_str().to_string(),
        created_by: app(&info.created_by),
        saved_by: app(&info.saved_by),
        saved_revision: info.saved_revision,
        entries: vec![EntryDto {
            path: SCORE.to_string(),
            sha256: sha256_hex(&score_bytes),
            size: score_bytes.len() as u64,
            extra: Extra::new(),
        }],
        extra: Extra::new(),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).expect("DTOs serialize infallibly");
    write_entries(&[(MANIFEST, &manifest_bytes), (SCORE, &score_bytes)])
}

/// Loads a package with default limits.
pub fn load(bytes: &[u8]) -> Result<LoadedProject, LoadError> {
    load_with_limits(bytes, &Limits::default())
}

/// Integrity validation order of project-format section 8.
pub fn load_with_limits(bytes: &[u8], limits: &Limits) -> Result<LoadedProject, LoadError> {
    let entries = read_entries(bytes, limits)?;
    let get = |name: &str| {
        entries
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b.as_slice())
    };

    let manifest_bytes = get(MANIFEST).ok_or_else(|| LoadError::MissingEntry(MANIFEST.into()))?;
    let manifest = parse_manifest(manifest_bytes)?;
    let found = FormatVersion {
        major: manifest.format_version.major,
        minor: manifest.format_version.minor,
    };
    check_version(found)?;
    let unknown_required: Vec<String> = manifest
        .required_features
        .iter()
        .filter(|f| !KNOWN_REQUIRED_FEATURES.contains(&f.as_str()))
        .cloned()
        .collect();
    if !unknown_required.is_empty() {
        return Err(LoadError::UnknownRequiredFeatures(unknown_required));
    }

    // Every listed entry exists with a matching hash and size (P-02); every package
    // entry other than the manifest is listed.
    for e in &manifest.entries {
        let data = get(&e.path).ok_or_else(|| LoadError::MissingEntry(e.path.clone()))?;
        if data.len() as u64 != e.size || sha256_hex(data) != e.sha256.to_ascii_lowercase() {
            return Err(LoadError::HashMismatch(e.path.clone()));
        }
    }
    let mut unknown = Vec::new();
    for (name, _) in &entries {
        if name == MANIFEST {
            continue;
        }
        if !manifest.entries.iter().any(|e| &e.path == name) {
            return Err(LoadError::UnlistedEntry(name.clone()));
        }
        if name != SCORE {
            unknown.push(format!("package:{name}"));
        }
    }

    let score_bytes = get(SCORE).ok_or_else(|| LoadError::MissingEntry(SCORE.into()))?;
    let dto: ScoreDto = serde_json::from_slice(score_bytes).map_err(|e| LoadError::Json {
        entry: SCORE.into(),
        message: e.to_string(),
    })?;

    manifest.unknown_paths("manifest", &mut unknown);
    unknown.extend(
        manifest
            .optional_features
            .iter()
            .filter(|f| !KNOWN_OPTIONAL_FEATURES.contains(&f.as_str()))
            .map(|f| format!("optional_feature:{f}")),
    );
    dto.unknown_paths("$", &mut unknown);

    let score = from_dto(&dto).map_err(LoadError::Invalid)?;

    let mut warnings = Vec::new();
    if found > FORMAT_VERSION {
        warnings.push(LoadWarning::NewerMinorVersion { found });
    }
    if !unknown.is_empty() {
        warnings.push(LoadWarning::UnknownContent { paths: unknown });
    }
    let app = |a: &AppDto| AppInfo {
        app: a.app.clone(),
        version: a.version.clone(),
    };
    Ok(LoadedProject {
        score,
        format_version: found,
        project_id: manifest.project_id.clone(),
        created_by: app(&manifest.created_by),
        saved_by: app(&manifest.saved_by),
        saved_revision: manifest.saved_revision,
        warnings,
    })
}

fn parse_manifest(bytes: &[u8]) -> Result<ManifestDto, LoadError> {
    // Check the format identifier before the full shape, so non-project JSON gives a clear error.
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| LoadError::Json {
        entry: MANIFEST.into(),
        message: e.to_string(),
    })?;
    match value.get("format").and_then(|v| v.as_str()) {
        Some(FORMAT) => {}
        other => return Err(LoadError::NotAProject(format!("format is {other:?}"))),
    }
    serde_json::from_value(value).map_err(|e| LoadError::Json {
        entry: MANIFEST.into(),
        message: e.to_string(),
    })
}

/// Version policy (project-format section 5). Only major 1 exists, so there is no
/// migration step yet; the first major change adds `migrate_v1_to_v2` here.
fn check_version(found: FormatVersion) -> Result<(), LoadError> {
    if found.major > FORMAT_VERSION.major {
        return Err(LoadError::NewerMajorVersion {
            found: found.major,
            supported: FORMAT_VERSION.major,
        });
    }
    if found.major < 1 {
        return Err(LoadError::UnsupportedVersion {
            major: found.major,
            minor: found.minor,
        });
    }
    Ok(())
}
