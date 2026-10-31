use mc_score::Violation;
use std::fmt;

/// Typed load failures. Loading never panics on malformed input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// Not a readable ZIP archive (truncated, corrupt).
    Container(String),
    /// P-06: entry name breaks the path rules or the allowed structure.
    BadEntryName(String),
    /// P-06: two entries with the same name (case-insensitive).
    DuplicateEntry(String),
    /// P-06: compression other than stored or deflate.
    UnsupportedCompression(String),
    TooManyEntries(usize),
    EntryTooLarge(String),
    /// Decompressed size exceeds the ratio limit (decompression bomb protection).
    CompressionRatio(String),
    PackageTooLarge,
    /// A required entry or a manifest-listed entry is missing.
    MissingEntry(String),
    /// A package entry is not listed in the manifest.
    UnlistedEntry(String),
    /// P-02: entry content does not match its manifest hash or size.
    HashMismatch(String),
    /// An entry is not valid JSON of the expected shape.
    Json {
        entry: String,
        message: String,
    },
    /// The manifest does not identify this format.
    NotAProject(String),
    /// P-03: written by a newer, incompatible major version.
    NewerMajorVersion {
        found: u32,
        supported: u32,
    },
    /// P-03: major version that never existed or is no longer migratable.
    UnsupportedVersion {
        major: u32,
        minor: u32,
    },
    /// P-03: the file needs features this version does not understand.
    UnknownRequiredFeatures(Vec<String>),
    /// P-05: content violates domain rules.
    Invalid(Vec<Violation>),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Container(m) => write!(f, "not a readable project package: {m}"),
            LoadError::BadEntryName(n) => write!(f, "invalid entry name '{n}'"),
            LoadError::DuplicateEntry(n) => write!(f, "duplicate entry '{n}'"),
            LoadError::UnsupportedCompression(n) => write!(f, "unsupported compression in '{n}'"),
            LoadError::TooManyEntries(n) => write!(f, "too many entries ({n})"),
            LoadError::EntryTooLarge(n) => write!(f, "entry '{n}' is too large"),
            LoadError::CompressionRatio(n) => {
                write!(f, "entry '{n}' exceeds the decompression ratio limit")
            }
            LoadError::PackageTooLarge => write!(f, "package exceeds the total size limit"),
            LoadError::MissingEntry(n) => write!(f, "missing entry '{n}'"),
            LoadError::UnlistedEntry(n) => write!(f, "entry '{n}' is not listed in the manifest"),
            LoadError::HashMismatch(n) => write!(f, "entry '{n}' is corrupted (hash mismatch)"),
            LoadError::Json { entry, message } => write!(f, "{entry}: {message}"),
            LoadError::NotAProject(m) => write!(f, "not a project file: {m}"),
            LoadError::NewerMajorVersion { found, supported } => {
                write!(
                    f,
                    "this project needs a newer version of the app (format {found}.x; this app reads {supported}.x)"
                )
            }
            LoadError::UnsupportedVersion { major, minor } => {
                write!(f, "unsupported format version {major}.{minor}")
            }
            LoadError::UnknownRequiredFeatures(v) => write!(
                f,
                "this project needs features this app does not support: {}",
                v.join(", ")
            ),
            LoadError::Invalid(v) => {
                write!(f, "project content is invalid ({} problems)", v.len())?;
                if let Some(first) = v.first() {
                    write!(f, "; first: {first}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for LoadError {}
