//! FF-03 (configuration part), FF-04 and FF-07. The crate lists come from the FF-01
//! matrix, so a new domain crate is covered automatically.
//!
//! - FF-03: every existing domain crate has a clippy.toml identical to
//!   architecture/clippy-domain.toml. Clippy with -D warnings (CI) enforces it.
//! - FF-04: every existing domain crate compiles for wasm32-unknown-unknown (ADR-014).
//! - FF-07: no platform `cfg` in domain or application crates (GR-11).

use super::deps::{parse_matrix, workspace_manifests};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TEMPLATE: &str = "architecture/clippy-domain.toml";
const WASM_TARGET: &str = "wasm32-unknown-unknown";

/// Existing crates of the given layers: (name, crate directory).
fn crates_in_layers(root: &Path, layers: &[&str]) -> Result<Vec<(String, PathBuf)>, String> {
    let rules = parse_matrix(
        &fs::read_to_string(root.join("architecture/allowed-deps.toml"))
            .map_err(|e| e.to_string())?,
    )?;
    Ok(workspace_manifests(root)?
        .into_iter()
        .filter(|m| {
            rules
                .get(&m.name)
                .is_some_and(|r| layers.contains(&r.layer.as_str()))
        })
        .map(|m| {
            let dir = root.join("crates").join(&m.name);
            (m.name, dir)
        })
        .collect())
}

pub fn run_purity(root: &Path) -> Result<(), String> {
    let template = fs::read_to_string(root.join(TEMPLATE))
        .map_err(|e| format!("cannot read {TEMPLATE}: {e}"))?;
    let crates = crates_in_layers(root, &["domain"])?;
    let mut errors = Vec::new();
    for (name, dir) in &crates {
        match fs::read_to_string(dir.join("clippy.toml")) {
            Err(_) => errors.push(format!(
                "{name}: crates/{name}/clippy.toml is missing (copy {TEMPLATE})"
            )),
            Ok(copy) if normalize(&copy) != normalize(&template) => errors.push(format!(
                "{name}: crates/{name}/clippy.toml differs from {TEMPLATE}"
            )),
            Ok(_) => {}
        }
    }
    if errors.is_empty() {
        println!("FF-03 purity config: ok ({} domain crates)", crates.len());
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// Line endings may differ on Windows checkouts; content may not.
fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n").trim_end().to_string()
}

pub fn run_wasm(root: &Path) -> Result<(), String> {
    let crates = crates_in_layers(root, &["domain"])?;
    if crates.is_empty() {
        println!("FF-04 wasm32: ok (no domain crates yet)");
        return Ok(());
    }
    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(root)
        .args(["check", "--locked", "--target", WASM_TARGET]);
    for (name, _) in &crates {
        cmd.args(["-p", name]);
    }
    let out = cmd.output().map_err(|e| format!("cannot run cargo: {e}"))?;
    if out.status.success() {
        let names: Vec<&str> = crates.iter().map(|c| c.0.as_str()).collect();
        println!("FF-04 wasm32: ok ({})", names.join(", "));
        Ok(())
    } else {
        Err(format!(
            "domain crates do not compile for {WASM_TARGET}:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

/// Platform conditions that belong in adapter crates only.
const PLATFORM_CFG: &[&str] = &[
    "target_os",
    "target_family",
    "windows",
    "unix",
    "target_vendor",
    "target_env",
];

pub fn run_platform(root: &Path) -> Result<(), String> {
    let crates = crates_in_layers(root, &["domain", "application"])?;
    let mut errors = Vec::new();
    for (name, dir) in &crates {
        let mut files = Vec::new();
        collect_rs(&dir.join("src"), &mut files);
        for f in files {
            let text = fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
            for (n, line) in text.lines().enumerate() {
                if let Some(hit) = platform_cfg(line) {
                    let rel = f.strip_prefix(root).unwrap_or(&f).display().to_string();
                    errors.push(format!("{rel}:{}: platform condition '{hit}' in {name}; move it to an adapter crate", n + 1));
                }
            }
        }
    }
    if errors.is_empty() {
        println!(
            "FF-07 platform cfg: ok ({} domain and application crates)",
            crates.len()
        );
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// Finds a platform predicate inside `cfg(...)` or `cfg!(...)`, ignoring comments.
pub fn platform_cfg(line: &str) -> Option<&'static str> {
    let code = line.split("//").next().unwrap_or("");
    let mut rest = code;
    while let Some(i) = rest.find("cfg") {
        rest = &rest[i + 3..];
        let after = rest.trim_start_matches('!').trim_start();
        if !after.starts_with('(') {
            continue;
        }
        let inner = &after[1..];
        let end = inner.find(')').map(|e| e + 1).unwrap_or(inner.len());
        let body = &inner[..end];
        for key in PLATFORM_CFG {
            let found = body.match_indices(key).any(|(p, _)| {
                let before = body[..p].chars().last();
                let after = body[p + key.len()..].chars().next();
                let boundary =
                    |c: Option<char>| c.is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
                boundary(before) && boundary(after)
            });
            if found {
                return Some(key);
            }
        }
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_platform_conditions() {
        assert_eq!(
            platform_cfg("#[cfg(target_os = \"windows\")]"),
            Some("target_os")
        );
        assert_eq!(platform_cfg("if cfg!(windows) {"), Some("windows"));
        assert_eq!(
            platform_cfg("#[cfg(all(unix, feature = \"x\"))]"),
            Some("unix")
        );
        assert_eq!(platform_cfg("#[cfg(test)]"), None);
        assert_eq!(platform_cfg("#[cfg(feature = \"fixtures\")]"), None);
        assert_eq!(platform_cfg("// #[cfg(windows)] in a comment"), None);
        assert_eq!(platform_cfg("let windows_count = 1; // config"), None);
    }

    #[test]
    fn repository_purity_and_platform_checks_pass() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        run_purity(root).unwrap();
        run_platform(root).unwrap();
    }
}
