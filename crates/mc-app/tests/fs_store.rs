//! Native file store: atomic replace, backup, lock retry, temp cleanup, autosave and
//! crash recovery (project-format sections 10 and 12).

use mc_app::infra::{FsProjectStore, FsRecoveryStore, RetryPolicy, cleanup_temp_files};
use mc_app::ports::{ProjectStore, RecoveryStore, StoreError};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "mc-app-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn write_atomic_replaces_and_keeps_one_backup() {
    let dir = TempDir::new("atomic");
    let target = dir.0.join("song.project");
    let store = FsProjectStore::default();
    store.write_atomic(&target, b"one").unwrap();
    store.write_atomic(&target, b"two").unwrap();
    store.write_atomic(&target, b"three").unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"three");
    assert_eq!(fs::read(dir.0.join("song.project.bak")).unwrap(), b"two");
    let names: Vec<_> = fs::read_dir(&dir.0)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 2, "no temp files left: {names:?}");
    assert_eq!(store.read(&target).unwrap(), b"three");
    assert!(matches!(
        store.read(&dir.0.join("missing")),
        Err(StoreError::NotFound(_))
    ));
}

#[test]
fn failed_write_leaves_the_original_and_no_temp_file() {
    let dir = TempDir::new("fail");
    let target = dir.0.join("song.project");
    let store = FsProjectStore {
        keep_backup: false,
        retry: RetryPolicy {
            attempts: 2,
            first_delay: Duration::from_millis(10),
        },
    };
    store.write_atomic(&target, b"original").unwrap();
    // A directory in place of the target: rename cannot replace it.
    let blocked = dir.0.join("blocked");
    fs::create_dir_all(blocked.join("child")).unwrap();
    assert!(store.write_atomic(&blocked, b"new").is_err());
    assert_eq!(fs::read(&target).unwrap(), b"original");
    let leftovers = fs::read_dir(&dir.0)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
        .count();
    assert_eq!(leftovers, 0);
}

#[test]
fn crash_leftover_temp_files_are_cleaned_up() {
    let dir = TempDir::new("cleanup");
    let target = dir.0.join("song.project");
    fs::write(&target, b"x").unwrap();
    fs::write(dir.0.join("song.project.tmp-123-0"), b"partial").unwrap();
    fs::write(dir.0.join("other.project.tmp-1-0"), b"not ours").unwrap();
    assert_eq!(cleanup_temp_files(&target), 1);
    assert!(dir.0.join("other.project.tmp-1-0").exists());
}

/// S5 lock case on Windows: a handle without FILE_SHARE_DELETE blocks rename until it
/// closes. The store retries and succeeds once the handle is released.
#[cfg(windows)]
#[test]
fn locked_target_is_retried_until_released() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = TempDir::new("lock");
    let target = dir.0.join("song.project");
    fs::write(&target, b"original").unwrap();
    let holder = fs::OpenOptions::new()
        .read(true)
        .share_mode(0x1 /* FILE_SHARE_READ */)
        .open(&target)
        .unwrap();
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(350));
        drop(holder);
    });
    let store = FsProjectStore {
        keep_backup: false,
        retry: RetryPolicy::default(),
    };
    store.write_atomic(&target, b"new").unwrap();
    releaser.join().unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"new");
}

#[cfg(windows)]
#[test]
fn lock_that_outlasts_the_retries_is_reported_and_original_kept() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = TempDir::new("lock2");
    let target = dir.0.join("song.project");
    fs::write(&target, b"original").unwrap();
    let _holder = fs::OpenOptions::new()
        .read(true)
        .share_mode(0x1)
        .open(&target)
        .unwrap();
    let store = FsProjectStore {
        keep_backup: false,
        retry: RetryPolicy {
            attempts: 3,
            first_delay: Duration::from_millis(20),
        },
    };
    let err = store.write_atomic(&target, b"new").unwrap_err();
    assert!(
        matches!(err, StoreError::PermissionDenied(_) | StoreError::Locked(_)),
        "{err:?}"
    );
    drop(_holder);
    assert_eq!(fs::read(&target).unwrap(), b"original");
    let leftovers = fs::read_dir(&dir.0)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
        .count();
    assert_eq!(leftovers, 0);
}

#[test]
fn autosave_rotation_listing_and_discard() {
    let dir = TempDir::new("recovery");
    let rec = FsRecoveryStore::new(dir.0.join("recovery"));
    let src = dir.0.join("song.project");
    for t in 1..=5u64 {
        rec.write_autosave("0190-abc", Some(&src), format!("v{t}").as_bytes(), 1000 * t)
            .unwrap();
    }
    rec.write_autosave("0190-new", None, b"untitled", 42)
        .unwrap();
    let list = rec.list().unwrap();
    assert_eq!(list.len(), 2);
    let e = list.iter().find(|e| e.project_id == "0190-abc").unwrap();
    assert_eq!(e.written_unix_millis, 5000, "newest autosave");
    assert_eq!(e.source.as_deref(), Some(src.as_path()));
    assert_eq!(rec.read(e).unwrap(), b"v5");
    let kept = fs::read_dir(dir.0.join("recovery/0190-abc"))
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".recovery"))
        .count();
    assert_eq!(kept, 3, "rotation keeps 3");
    assert!(
        list.iter()
            .find(|e| e.project_id == "0190-new")
            .unwrap()
            .source
            .is_none()
    );
    rec.discard("0190-abc").unwrap();
    assert_eq!(rec.list().unwrap().len(), 1);
    rec.discard("does-not-exist").unwrap();
}

#[test]
fn session_marker_detects_an_unclean_shutdown() {
    let dir = TempDir::new("marker");
    let rec = FsRecoveryStore::new(dir.0.join("recovery"));
    assert!(!rec.begin_session().unwrap(), "first start is clean");
    rec.end_session().unwrap();
    assert!(!rec.begin_session().unwrap(), "clean exit then start");
    // No end_session: simulated crash.
    assert!(
        rec.begin_session().unwrap(),
        "previous session did not end cleanly"
    );
    rec.end_session().unwrap();
}
