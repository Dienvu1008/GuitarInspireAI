//! ZIP container: safe reading with limits on actual bytes (project-format section 11)
//! and deterministic writing. Works on byte slices only; files are opened by adapters.

use crate::LoadError;
use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Reading limits. Defaults are justified in project-format section 11 and S5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_entries: usize,
    pub max_entry_bytes: u64,
    pub max_total_bytes: u64,
    /// Maximum decompressed / compressed ratio per entry (compressed size floored at 4 KB).
    pub max_ratio: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_entries: 10_000,
            max_entry_bytes: 256 << 20,
            max_total_bytes: 2 << 30,
            max_ratio: 100,
        }
    }
}

pub const MANIFEST: &str = "manifest.json";
pub const SCORE: &str = "score.json";
/// The pattern library (format 1.2, ADR-034 section 8). Written only when the
/// document has a library to store.
pub const LIBRARY: &str = "patterns/library.json";
/// Prefixes reserved for later format versions. Format 1.2 writes `patterns/library.json`;
/// the other prefixes are still unused.
pub const RESERVED_PREFIXES: &[&str] = &["assets/", "patterns/", "analysis/", "history/"];

/// Entry name rules: relative, forward slashes, no empty, `.` or `..` segments, no
/// drive letters, and only known top-level names or reserved prefixes.
pub fn check_name(name: &str) -> Result<(), LoadError> {
    let malformed = name.is_empty()
        || name.starts_with('/')
        || name.contains('\\')
        || name.contains(':')
        || name
            .split('/')
            .any(|seg| seg.is_empty() || seg == "." || seg == "..");
    let known =
        name == MANIFEST || name == SCORE || RESERVED_PREFIXES.iter().any(|p| name.starts_with(p));
    if malformed || !known {
        Err(LoadError::BadEntryName(name.to_string()))
    } else {
        Ok(())
    }
}

/// Reads all entries into memory: (name, bytes) in archive order.
pub fn read_entries(bytes: &[u8], limits: &Limits) -> Result<Vec<(String, Vec<u8>)>, LoadError> {
    let mut zip =
        ZipArchive::new(Cursor::new(bytes)).map_err(|e| LoadError::Container(e.to_string()))?;
    if zip.len() > limits.max_entries {
        return Err(LoadError::TooManyEntries(zip.len()));
    }
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    let mut out = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let mut f = zip
            .by_index(i)
            .map_err(|e| LoadError::Container(e.to_string()))?;
        let name = f
            .name()
            .map_err(|e| LoadError::BadEntryName(e.to_string()))?
            .to_string();
        check_name(&name)?;
        if f.is_dir() || f.encrypted() || f.is_symlink() {
            return Err(LoadError::BadEntryName(name));
        }
        if !seen.insert(name.to_lowercase()) {
            return Err(LoadError::DuplicateEntry(name));
        }
        if !matches!(
            f.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(LoadError::UnsupportedCompression(name));
        }
        let ratio_cap = f
            .compressed_size()
            .max(4096)
            .saturating_mul(limits.max_ratio);
        let cap = limits.max_entry_bytes.min(ratio_cap);
        let mut buf = Vec::new();
        let mut chunk = [0u8; 64 * 1024];
        loop {
            let n = f
                .read(&mut chunk)
                .map_err(|e| LoadError::Container(format!("{name}: {e}")))?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            total += n as u64;
            if buf.len() as u64 > cap {
                return Err(if cap == ratio_cap {
                    LoadError::CompressionRatio(name)
                } else {
                    LoadError::EntryTooLarge(name)
                });
            }
            if total > limits.max_total_bytes {
                return Err(LoadError::PackageTooLarge);
            }
        }
        out.push((name, buf));
    }
    Ok(out)
}

/// Writes entries in the given order with fixed timestamps and permissions, so equal
/// input gives equal bytes for a given zip/deflate version.
pub fn write_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o644);
    for (name, data) in entries {
        w.start_file(*name, opts).expect("writing to memory");
        w.write_all(data).expect("writing to memory");
    }
    w.finish().expect("writing to memory").into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p06_entry_name_rules() {
        for ok in [
            "manifest.json",
            "score.json",
            "assets/a.wav",
            "patterns/p1.json",
        ] {
            assert!(check_name(ok).is_ok(), "{ok}");
        }
        for bad in [
            "../x",
            "/score.json",
            "C:/x",
            "assets\\a",
            "assets/./a",
            "assets//a",
            "assets/../score.json",
            "evil.exe",
            "SCORE.JSON",
            "",
        ] {
            assert!(check_name(bad).is_err(), "{bad}");
        }
    }
}
