//! Test-backed fitness functions (wiring checks) and advisory reports.
//!
//! - `test:<file>#<prefix>`: the test file exists and defines tests starting with the
//!   prefix; CI runs `cargo test --workspace`. Used for FF-11, FF-13, FF-14, FF-23, FF-32.
//! - `bench:<file>`: the benchmark exists; CI compiles benchmarks (FF-21).
//! - FF-24 (advisory): rule ids from domain-invariants.md without a test whose name
//!   starts with the lowercase id (for example `h05_`).
//! - FF-30 (advisory): pinned workspace dependencies with a newer release on crates.io.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Checks a `test:` or `bench:` reference. Returns an error message if it is broken.
/// For a test file next to a `fixtures/` directory, every fixture file must be named in
/// the test (embedded fixtures cannot be discovered at run time, so an unlisted file
/// would silently go untested).
pub fn check_reference(root: &Path, check: &str) -> Option<String> {
    if let Some(rest) = check.strip_prefix("test:") {
        let (file, prefix) = rest.split_once('#').unwrap_or((rest, ""));
        let text = match fs::read_to_string(root.join(file)) {
            Ok(t) => t,
            Err(_) => return Some(format!("test file {file} does not exist")),
        };
        let defines = text
            .lines()
            .any(|l| l.trim_start().starts_with(&format!("fn {prefix}")));
        if !defines {
            return Some(format!("{file} defines no test starting with '{prefix}'"));
        }
        let fixtures = root.join(file).parent().map(|d| d.join("fixtures"));
        if let Some(dir) = fixtures.filter(|d| d.is_dir()) {
            let mut files = Vec::new();
            collect_files(&dir, &mut files);
            for f in files {
                let rel = f
                    .strip_prefix(&dir)
                    .unwrap_or(&f)
                    .to_string_lossy()
                    .replace('\\', "/");
                if !text.contains(&format!("fixtures/{rel}")) {
                    return Some(format!("{file} does not reference fixture fixtures/{rel}"));
                }
            }
        }
        return None;
    }
    if let Some(file) = check.strip_prefix("bench:") {
        if !root.join(file).is_file() {
            return Some(format!("benchmark {file} does not exist"));
        }
        return None;
    }
    Some(format!("unknown check reference '{check}'"))
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for p in entries.flatten().map(|e| e.path()) {
        if p.is_dir() {
            collect_files(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for p in entries.flatten().map(|e| e.path()) {
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// Rule ids (H-xx, E-xx, T-xx, PB-xx, P-xx, D-xx, ...) in the first column of the
/// invariant tables.
pub fn rule_ids(markdown: &str) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for line in markdown.lines() {
        let cell = line
            .trim()
            .strip_prefix('|')
            .and_then(|l| l.split('|').next())
            .map(str::trim)
            .unwrap_or("");
        let Some((prefix, number)) = cell.split_once('-') else {
            continue;
        };
        if !prefix.is_empty()
            && prefix.chars().all(|c| c.is_ascii_uppercase())
            && !number.is_empty()
            && number.chars().all(|c| c.is_ascii_digit())
        {
            ids.insert(cell.to_string());
        }
    }
    ids.into_iter().collect()
}

/// Lowercase test-name prefix of a rule id: "H-05" -> "h05_", "PB-01" -> "pb01_".
pub fn test_prefix(id: &str) -> String {
    format!("{}_", id.replace('-', "").to_lowercase())
}

pub fn run_rule_coverage(root: &Path) -> Result<(), String> {
    let doc = fs::read_to_string(root.join("docs/architecture/domain-invariants.md"))
        .map_err(|e| e.to_string())?;
    let ids = rule_ids(&doc);
    let mut files = Vec::new();
    collect_rs(&root.join("crates"), &mut files);
    let mut names = Vec::new();
    for f in files {
        let text = fs::read_to_string(&f).unwrap_or_default();
        for line in text.lines() {
            if let Some(rest) = line.trim_start().strip_prefix("fn ") {
                names.push(rest.split('(').next().unwrap_or("").to_string());
            }
        }
    }
    // A test may cover several rules: `h03_h04_string_and_fret_ranges` covers H-03 and H-04.
    let (covered, missing): (Vec<_>, Vec<_>) = ids.iter().partition(|id| {
        let p = test_prefix(id);
        names
            .iter()
            .any(|n| n.starts_with(&p) || n.contains(&format!("_{p}")))
    });
    let by_family = |fam: &str| {
        missing
            .iter()
            .filter(|m| m.starts_with(&format!("{fam}-")))
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!(
        "FF-24 rule coverage (advisory): {} of {} rule ids have a test",
        covered.len(),
        ids.len()
    );
    for fam in [
        "H", "E", "T", "PB", "C", "D", "P", "G", "PT", "AU", "AI", "IM",
    ] {
        let m = by_family(fam);
        if !m.is_empty() {
            println!("  without test {fam}: {m}");
        }
    }
    Ok(())
}

/// Advisory: compares pinned `=x.y.z` versions in [workspace.dependencies] with the
/// newest release on crates.io (needs network; never fails the build).
pub fn run_freshness(root: &Path) -> Result<(), String> {
    let manifest = fs::read_to_string(root.join("Cargo.toml")).map_err(|e| e.to_string())?;
    let mut in_section = false;
    let mut outdated = 0;
    let mut checked = 0;
    for line in manifest.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_section = l == "[workspace.dependencies]";
            continue;
        }
        if !in_section || l.is_empty() || l.starts_with('#') {
            continue;
        }
        let Some((name, rest)) = l.split_once('=') else {
            continue;
        };
        let Some(pinned) = rest
            .split('"')
            .find(|s| s.starts_with('='))
            .map(|s| s.trim_start_matches('=').to_string())
        else {
            continue;
        };
        let name = name.trim();
        let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .args(["search", name, "--limit", "1"])
            .output()
            .map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout);
        let latest = text
            .lines()
            .find(|l| l.starts_with(&format!("{name} =")))
            .and_then(|l| l.split('"').nth(1))
            .unwrap_or("?")
            .to_string();
        checked += 1;
        if latest != pinned {
            outdated += 1;
            println!("  {name}: pinned {pinned}, latest {latest}");
        }
    }
    println!(
        "FF-30 dependency freshness (advisory): {outdated} of {checked} pinned dependencies have a newer release"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rule_ids_from_tables() {
        let md = "| Id | Rule |\n|---|---|\n| H-05 | x |\n| PB-01 | y |\n| GR-01 | not a table id? |\n| Rule | H-06 in text |\n";
        assert_eq!(rule_ids(md), ["GR-01", "H-05", "PB-01"]);
        assert_eq!(test_prefix("H-05"), "h05_");
        assert_eq!(test_prefix("PB-01"), "pb01_");
    }

    #[test]
    fn references_are_checked() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        assert!(check_reference(root, "test:crates/mc-app/tests/properties.rs#ff13_").is_none());
        assert!(check_reference(root, "test:crates/mc-app/tests/properties.rs#nope_").is_some());
        assert!(check_reference(root, "test:missing.rs#x").is_some());
        assert!(check_reference(root, "bench:crates/mc-app/benches/commands.rs").is_none());
    }
}
