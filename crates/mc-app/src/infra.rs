//! Native file-system adapters (feature `native-fs`), project-format sections 10 and 12.
//!
//! Atomic save (S5 decision, ADR-008): write a temp file in the target directory, flush
//! and `sync_all`, keep a one-generation `.bak` of the previous file, then
//! `std::fs::rename` over the target with bounded retry. Locks from antivirus or sync
//! clients usually clear within a second; after five attempts the error is reported and
//! the original stays untouched.

use crate::ports::{Clock, Entropy, ProjectStore, RecoveryEntry, RecoveryStore, StoreError};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    pub attempts: u32,
    pub first_delay: Duration,
}

impl Default for RetryPolicy {
    /// 5 attempts, 100 ms doubling: about 1.5 s in total (validated in S5).
    fn default() -> Self {
        RetryPolicy {
            attempts: 5,
            first_delay: Duration::from_millis(100),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FsProjectStore {
    pub keep_backup: bool,
    pub retry: RetryPolicy,
}

impl Default for FsProjectStore {
    fn default() -> Self {
        FsProjectStore {
            keep_backup: true,
            retry: RetryPolicy::default(),
        }
    }
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn map_err(path: &Path, e: io::Error) -> StoreError {
    match e.kind() {
        io::ErrorKind::NotFound => StoreError::NotFound(path.to_path_buf()),
        io::ErrorKind::PermissionDenied => StoreError::PermissionDenied(path.to_path_buf()),
        io::ErrorKind::ResourceBusy => StoreError::Locked(path.to_path_buf()),
        _ => StoreError::Io {
            path: path.to_path_buf(),
            message: e.to_string(),
        },
    }
}

/// Errors worth retrying. Classified by portable `ErrorKind`, not raw OS codes, because
/// the same numbers mean different things per OS (and FF-07 keeps platform `cfg` out of
/// this crate). On Windows a held target makes `rename` fail with access denied, which
/// maps to `PermissionDenied` (S5). A real permission problem is retried too and then
/// reported; that costs about 1.5 s.
fn is_lock(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::PermissionDenied | io::ErrorKind::ResourceBusy
    )
}

/// Path next to `target` with the given suffix appended to the full file name.
fn sibling(target: &Path, suffix: &str) -> PathBuf {
    let mut name = target
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(suffix);
    target.with_file_name(name)
}

/// Retries `op` while it fails with a lock error.
fn with_retry<T>(policy: RetryPolicy, mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut delay = policy.first_delay;
    let mut attempt = 1;
    loop {
        match op() {
            Ok(v) => return Ok(v),
            Err(e) if is_lock(&e) && attempt < policy.attempts => {
                std::thread::sleep(delay);
                delay *= 2;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

impl ProjectStore for FsProjectStore {
    fn read(&self, path: &Path) -> Result<Vec<u8>, StoreError> {
        with_retry(self.retry, || fs::read(path)).map_err(|e| map_err(path, e))
    }

    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = sibling(path, &format!(".tmp-{}-{n}", std::process::id()));
        let result = (|| -> io::Result<()> {
            let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
            f.write_all(bytes)?;
            f.sync_all()?;
            drop(f);
            if self.keep_backup && path.exists() {
                with_retry(self.retry, || {
                    fs::copy(path, sibling(path, ".bak")).map(|_| ())
                })?;
            }
            with_retry(self.retry, || fs::rename(&tmp, path))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result.map_err(|e| map_err(path, e))
    }
}

/// Autosaves under `<dir>/<project-id>/autosave-<millis>.recovery`, with a `source`
/// file holding the original path, and a session marker `session.lock` in `<dir>`.
/// Recovery file names are internal app data, independent of the `.giproj` product extension (Q-08).
#[derive(Clone, Debug)]
pub struct FsRecoveryStore {
    pub dir: PathBuf,
    /// Rotating autosaves kept per project (default 3).
    pub keep: usize,
}

impl FsRecoveryStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        FsRecoveryStore {
            dir: dir.into(),
            keep: 3,
        }
    }

    fn project_dir(&self, project_id: &str) -> PathBuf {
        // Project ids are UUIDs; anything else is reduced to safe characters.
        let safe: String = project_id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        self.dir.join(safe)
    }

    fn autosaves(dir: &Path) -> io::Result<Vec<(u64, PathBuf)>> {
        let mut out = Vec::new();
        for e in fs::read_dir(dir)?.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if let Some(ms) = name
                .strip_prefix("autosave-")
                .and_then(|r| r.strip_suffix(".recovery"))
                .and_then(|m| m.parse().ok())
            {
                out.push((ms, e.path()));
            }
        }
        out.sort();
        Ok(out)
    }
}

impl RecoveryStore for FsRecoveryStore {
    fn write_autosave(
        &self,
        project_id: &str,
        source: Option<&Path>,
        bytes: &[u8],
        now: u64,
    ) -> Result<PathBuf, StoreError> {
        let dir = self.project_dir(project_id);
        fs::create_dir_all(&dir).map_err(|e| map_err(&dir, e))?;
        let path = dir.join(format!("autosave-{now:013}.recovery"));
        FsProjectStore {
            keep_backup: false,
            retry: RetryPolicy::default(),
        }
        .write_atomic(&path, bytes)?;
        let src = dir.join("source");
        match source {
            Some(s) => {
                fs::write(&src, s.to_string_lossy().as_bytes()).map_err(|e| map_err(&src, e))?
            }
            None => {
                let _ = fs::remove_file(&src);
            }
        }
        let all = Self::autosaves(&dir).map_err(|e| map_err(&dir, e))?;
        for (_, old) in all.iter().rev().skip(self.keep) {
            let _ = fs::remove_file(old);
        }
        Ok(path)
    }

    fn list(&self) -> Result<Vec<RecoveryEntry>, StoreError> {
        let mut out = Vec::new();
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Ok(out);
        };
        for e in entries.flatten().filter(|e| e.path().is_dir()) {
            let dir = e.path();
            if let Some((ms, path)) = Self::autosaves(&dir)
                .map_err(|err| map_err(&dir, err))?
                .pop()
            {
                let source = fs::read_to_string(dir.join("source"))
                    .ok()
                    .map(PathBuf::from);
                out.push(RecoveryEntry {
                    project_id: e.file_name().to_string_lossy().into_owned(),
                    path,
                    source,
                    written_unix_millis: ms,
                });
            }
        }
        out.sort_by(|a, b| a.project_id.cmp(&b.project_id));
        Ok(out)
    }

    fn read(&self, entry: &RecoveryEntry) -> Result<Vec<u8>, StoreError> {
        fs::read(&entry.path).map_err(|e| map_err(&entry.path, e))
    }

    fn discard(&self, project_id: &str) -> Result<(), StoreError> {
        let dir = self.project_dir(project_id);
        match fs::remove_dir_all(&dir) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(map_err(&dir, e)),
            _ => Ok(()),
        }
    }

    fn begin_session(&self) -> Result<bool, StoreError> {
        fs::create_dir_all(&self.dir).map_err(|e| map_err(&self.dir, e))?;
        let marker = self.dir.join("session.lock");
        let unclean = marker.exists();
        File::create(&marker).map_err(|e| map_err(&marker, e))?;
        Ok(unclean)
    }

    fn end_session(&self) -> Result<(), StoreError> {
        let marker = self.dir.join("session.lock");
        match fs::remove_file(&marker) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(map_err(&marker, e)),
            _ => Ok(()),
        }
    }
}

/// Removes temp files left by a crash during save (`<name>.tmp-*` next to `target`).
pub fn cleanup_temp_files(target: &Path) -> usize {
    let (Some(dir), Some(name)) = (target.parent(), target.file_name()) else {
        return 0;
    };
    let prefix = format!("{}.tmp-", name.to_string_lossy());
    let Ok(entries) = fs::read_dir(if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    }) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(&prefix))
        .filter(|e| fs::remove_file(e.path()).is_ok())
        .count()
}

/// System clock adapter.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Entropy from the standard library's randomly seeded hasher (no extra dependency).
/// Good enough for project ids; not for cryptography.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemEntropy;

impl Entropy for SystemEntropy {
    fn random_u128(&self) -> u128 {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let half = |salt: u64| {
            let mut h = RandomState::new().build_hasher();
            h.write_u64(n ^ salt);
            h.write_u128(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0),
            );
            h.finish() as u128
        };
        (half(0x9e37_79b9) << 64) | half(0x7f4a_7c15)
    }
}
