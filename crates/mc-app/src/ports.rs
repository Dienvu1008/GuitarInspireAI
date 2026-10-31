//! Ports implemented by adapters. The application layer never touches files, clocks or
//! random sources directly; it goes through these traits (hexagonal layering).

use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    NotFound(PathBuf),
    /// Another process holds the file (antivirus, sync client, editor). Retried before reporting.
    Locked(PathBuf),
    PermissionDenied(PathBuf),
    /// Any other I/O failure, with the OS message.
    Io {
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NotFound(p) => write!(f, "{} not found", p.display()),
            StoreError::Locked(p) => {
                write!(f, "{} is in use by another program; try again", p.display())
            }
            StoreError::PermissionDenied(p) => {
                write!(
                    f,
                    "access to {} was denied (it may be open in another program, or you lack permission)",
                    p.display()
                )
            }
            StoreError::Io { path, message } => write!(f, "{}: {message}", path.display()),
        }
    }
}

impl std::error::Error for StoreError {}

/// Reads and atomically writes project packages.
pub trait ProjectStore {
    fn read(&self, path: &Path) -> Result<Vec<u8>, StoreError>;
    /// Replaces `path` atomically: either the old or the new content is visible, never
    /// a partial file. Keeps a backup of the previous content if configured.
    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError>;
}

/// A recovery snapshot written by autosave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryEntry {
    pub project_id: String,
    pub path: PathBuf,
    /// Original location of the document, if it had one.
    pub source: Option<PathBuf>,
    pub written_unix_millis: u64,
}

/// Autosave and crash recovery storage in per-user app data.
pub trait RecoveryStore {
    /// Writes an autosave (rotating, keeps the newest few) and records its source.
    fn write_autosave(
        &self,
        project_id: &str,
        source: Option<&Path>,
        bytes: &[u8],
        now_unix_millis: u64,
    ) -> Result<PathBuf, StoreError>;
    /// Newest autosave per project.
    fn list(&self) -> Result<Vec<RecoveryEntry>, StoreError>;
    fn read(&self, entry: &RecoveryEntry) -> Result<Vec<u8>, StoreError>;
    /// Removes all autosaves of a project (after a successful normal save, or on discard).
    fn discard(&self, project_id: &str) -> Result<(), StoreError>;
    /// Marks the start of an app session. Returns true if the previous session did not
    /// end cleanly (its marker was still present).
    fn begin_session(&self) -> Result<bool, StoreError>;
    fn end_session(&self) -> Result<(), StoreError>;
}

pub trait Clock {
    fn now_unix_millis(&self) -> u64;
}

pub trait Entropy {
    fn random_u128(&self) -> u128;
}
