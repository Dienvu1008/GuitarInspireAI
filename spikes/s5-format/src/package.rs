//! ZIP package write and safe read (project-format sections 2, 3, 8, 11).

use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

pub struct Limits {
    pub max_entries: usize,
    pub max_entry_bytes: u64,
    pub max_total_bytes: u64,
    pub max_ratio: u64,
}

impl Default for Limits {
    fn default() -> Self {
        // Ratio 100:1: the large fixture's score.json compresses about 17.5:1 (measured), so 100 leaves margin.
        Limits { max_entries: 10_000, max_entry_bytes: 256 << 20, max_total_bytes: 2 << 30, max_ratio: 100 }
    }
}

#[derive(Debug)]
pub enum LoadError {
    Zip(String),
    BadName(String),
    Duplicate(String),
    Method(String),
    TooManyEntries(usize),
    TooLarge(String, u64),
    Ratio(String, u64),
    Hash(String),
    MissingEntry(String),
    Json(String),
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn write(score_json: &[u8]) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format": "guitarinspire.project",
        "format_version": { "major": 1, "minor": 0 },
        "required_features": [],
        "optional_features": [],
        "entries": [{ "path": "score.json", "sha256": hex(&Sha256::digest(score_json)), "size": score_json.len() }]
    });
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    let deflate = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    w.start_file("manifest.json", deflate).unwrap();
    w.write_all(&serde_json::to_vec(&manifest).unwrap()).unwrap();
    w.start_file("score.json", deflate).unwrap();
    w.write_all(score_json).unwrap();
    w.finish().unwrap().into_inner()
}

/// Entry name rules: relative, forward slashes, no `.` or `..`, no drive letters,
/// only known top-level names or prefixes.
pub fn check_name(name: &str) -> Result<(), LoadError> {
    let bad = name.is_empty()
        || name.starts_with('/')
        || name.contains('\\')
        || name.contains(':')
        || name.split('/').any(|c| c.is_empty() || c == "." || c == "..");
    let known = matches!(name, "manifest.json" | "score.json")
        || ["assets/", "patterns/", "analysis/", "history/"].iter().any(|p| name.starts_with(p));
    if bad || !known { Err(LoadError::BadName(name.to_string())) } else { Ok(()) }
}

/// Reads every entry into memory with limits counted on actual decompressed bytes.
pub fn read(bytes: &[u8], limits: &Limits) -> Result<Vec<(String, Vec<u8>)>, LoadError> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|e| LoadError::Zip(e.to_string()))?;
    if zip.len() > limits.max_entries {
        return Err(LoadError::TooManyEntries(zip.len()));
    }
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| LoadError::Zip(e.to_string()))?;
        // zip 9 validates names itself and returns an error for some malformed names.
        let name = f.name().map_err(|e| LoadError::BadName(format!("zip crate rejected name: {e}")))?.to_string();
        check_name(&name)?;
        if f.is_dir() || f.encrypted() {
            return Err(LoadError::BadName(name));
        }
        if !seen.insert(name.to_lowercase()) {
            return Err(LoadError::Duplicate(name));
        }
        if !matches!(f.compression(), CompressionMethod::Stored | CompressionMethod::Deflated) {
            return Err(LoadError::Method(name));
        }
        let ratio_cap = f.compressed_size().max(4096).saturating_mul(limits.max_ratio);
        let cap = limits.max_entry_bytes.min(ratio_cap);
        let mut buf = Vec::new();
        let mut chunk = [0u8; 64 * 1024];
        loop {
            let n = f.read(&mut chunk).map_err(|e| LoadError::Zip(e.to_string()))?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            total += n as u64;
            if buf.len() as u64 > cap {
                return Err(if cap == ratio_cap { LoadError::Ratio(name, buf.len() as u64) } else { LoadError::TooLarge(name, buf.len() as u64) });
            }
            if total > limits.max_total_bytes {
                return Err(LoadError::TooLarge("<total>".into(), total));
            }
        }
        out.push((name, buf));
    }
    Ok(out)
}

/// Full open: safe read, manifest hash verification, returns score.json bytes.
pub fn open(bytes: &[u8]) -> Result<Vec<u8>, LoadError> {
    let entries = read(bytes, &Limits::default())?;
    let get = |n: &str| entries.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
    let manifest: serde_json::Value = serde_json::from_slice(&get("manifest.json").ok_or(LoadError::MissingEntry("manifest.json".into()))?)
        .map_err(|e| LoadError::Json(e.to_string()))?;
    let score = get("score.json").ok_or(LoadError::MissingEntry("score.json".into()))?;
    let expected = manifest["entries"][0]["sha256"].as_str().unwrap_or("");
    if hex(&Sha256::digest(&score)) != expected {
        return Err(LoadError::Hash("score.json".into()));
    }
    Ok(score)
}

/// Builds a ZIP with arbitrary entry names (malicious fixtures).
pub fn raw_zip(entries: &[(&str, &[u8])], method: CompressionMethod) -> Vec<u8> {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default().compression_method(method);
    for (name, data) in entries {
        w.start_file(*name, opts).unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap().into_inner()
}
