//! Windows atomic replace experiments (project-format section 10).
//! A lock held by antivirus, OneDrive or another app is simulated by a second
//! handle opened without FILE_SHARE_DELETE (and, separately, with full sharing).

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, REPLACEFILE_WRITE_THROUGH, ReplaceFileW};

#[derive(Clone, Copy, Debug)]
pub enum Method {
    StdRename,
    TempfilePersist,
    ReplaceFileW,
}

fn wide(p: &Path) -> Vec<u16> {
    p.as_os_str().encode_wide().chain(Some(0)).collect()
}

fn write_temp(dir: &Path, data: &[u8]) -> PathBuf {
    let p = dir.join(format!(".song.tmp-{}", std::process::id()));
    let mut f = File::create(&p).unwrap();
    f.write_all(data).unwrap();
    f.sync_all().unwrap();
    p
}

/// One replace attempt. Returns the OS error code on failure.
pub fn replace_once(method: Method, dir: &Path, target: &Path, data: &[u8]) -> Result<(), i32> {
    match method {
        Method::StdRename => {
            let tmp = write_temp(dir, data);
            let r = fs::rename(&tmp, target).map_err(|e| e.raw_os_error().unwrap_or(-1));
            if r.is_err() {
                let _ = fs::remove_file(&tmp);
            }
            r
        }
        Method::TempfilePersist => {
            let mut t = tempfile::NamedTempFile::new_in(dir).unwrap();
            t.write_all(data).unwrap();
            t.as_file().sync_all().unwrap();
            t.persist(target).map(|_| ()).map_err(|e| e.error.raw_os_error().unwrap_or(-1))
        }
        Method::ReplaceFileW => {
            let tmp = write_temp(dir, data);
            let (a, b) = (wide(target), wide(&tmp));
            let ok = unsafe { ReplaceFileW(a.as_ptr(), b.as_ptr(), std::ptr::null(), REPLACEFILE_WRITE_THROUGH, std::ptr::null(), std::ptr::null()) };
            if ok == 0 {
                let code = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                let _ = fs::remove_file(&tmp);
                Err(code)
            } else {
                Ok(())
            }
        }
    }
}

/// Bounded retry with backoff: 5 attempts over about 1.5 s.
pub fn replace_with_retry(method: Method, dir: &Path, target: &Path, data: &[u8]) -> (Result<(), i32>, u32) {
    let mut delay = Duration::from_millis(100);
    for attempt in 1..=5 {
        match replace_once(method, dir, target, data) {
            Ok(()) => return (Ok(()), attempt),
            Err(code) if attempt == 5 => return (Err(code), attempt),
            Err(_) => {
                std::thread::sleep(delay);
                delay *= 2;
            }
        }
    }
    unreachable!()
}

pub struct CaseResult {
    pub method: Method,
    pub case: &'static str,
    pub result: Result<(), i32>,
    pub attempts: u32,
    pub original_intact_on_failure: bool,
    pub temp_files_left: usize,
    pub creation_time_preserved: Option<bool>,
    pub millis: u128,
}

fn temp_files(dir: &Path, target: &Path) -> usize {
    fs::read_dir(dir).unwrap().flatten().filter(|e| e.path() != target).count()
}

pub fn run_case(method: Method, case: &'static str) -> CaseResult {
    let dir = std::env::temp_dir().join(format!("s5-replace-{method:?}-{case}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let target = dir.join("song.gia");
    fs::write(&target, b"ORIGINAL").unwrap();
    let created_before = fs::metadata(&target).unwrap().created().ok();
    std::thread::sleep(Duration::from_millis(20));

    let share = match case {
        "lock_no_share_delete" | "lock_released_after_400ms" => Some(FILE_SHARE_READ),
        "lock_full_sharing" => Some(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE),
        _ => None,
    };
    let mut holder = share.map(|s| OpenOptions::new().read(true).share_mode(s).open(&target).unwrap());
    let releaser = (case == "lock_released_after_400ms").then(|| {
        let h = holder.take();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(400));
            drop(h);
        })
    });

    let t0 = Instant::now();
    let (result, attempts) = if case.starts_with("lock_no") || case.starts_with("lock_released") {
        replace_with_retry(method, &dir, &target, b"NEW CONTENT")
    } else {
        (replace_once(method, &dir, &target, b"NEW CONTENT"), 1)
    };
    let millis = t0.elapsed().as_millis();
    drop(holder.take()); // release a lock still held, before inspecting the result
    if let Some(r) = releaser {
        r.join().unwrap();
    }
    let content = fs::read(&target).unwrap_or_default();
    let created_after = fs::metadata(&target).ok().and_then(|m| m.created().ok());
    let res = CaseResult {
        method,
        case,
        result,
        attempts,
        original_intact_on_failure: result.is_ok() || content == b"ORIGINAL",
        temp_files_left: temp_files(&dir, &target),
        creation_time_preserved: match (result, created_before, created_after) {
            (Ok(()), Some(a), Some(b)) => Some(a == b),
            _ => None,
        },
        millis,
    };
    let _ = fs::remove_dir_all(&dir);
    res
}
