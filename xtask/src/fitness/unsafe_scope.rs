//! FF-34 (ADR-022): unsafe code is confined to the generated FFI module.
//!
//! - Every crate under `crates/` except `mc-ffi`, and `xtask`, uses
//!   `[lints] workspace = true`, so the workspace `unsafe_code = "forbid"` applies, and
//!   has no crate-local lint table.
//! - `mc-ffi` has its own lint table equal to `[workspace.lints]` except
//!   `unsafe_code = "deny"`; the only extra key allowed is `unexpected_cfgs` (needed for
//!   `cfg(frb_expand)` by the generator).
//! - The only lint attribute naming `unsafe_code` in hand-written sources is
//!   `#[allow(unsafe_code)]` directly on `mod frb_generated;` in `crates/mc-ffi/src/lib.rs`.
//!   The generated file itself is checked by FF-12, not here.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const FFI_CRATE: &str = "mc-ffi";
const FFI_LIB: &str = "crates/mc-ffi/src/lib.rs";
const GENERATED: &str = "frb_generated.rs";

pub fn run(root: &Path) -> Result<(), String> {
    let workspace = fs::read_to_string(root.join("Cargo.toml")).map_err(|e| e.to_string())?;
    let mut manifests = Vec::new();
    let mut sources = Vec::new();
    let mut dirs: Vec<PathBuf> = fs::read_dir(root.join("crates"))
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("Cargo.toml").is_file())
        .collect();
    dirs.sort();
    for dir in &dirs {
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let text = fs::read_to_string(dir.join("Cargo.toml")).map_err(|e| e.to_string())?;
        manifests.push((name, text));
        collect_rs(dir, &mut sources);
    }
    let xtask = fs::read_to_string(root.join("xtask/Cargo.toml")).map_err(|e| e.to_string())?;
    manifests.push(("xtask".to_string(), xtask));
    collect_rs(&root.join("xtask"), &mut sources);
    let sources: Vec<(String, String)> = sources
        .into_iter()
        .filter(|p| p.file_name().is_some_and(|n| n != GENERATED))
        .map(|p| {
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            (rel, fs::read_to_string(&p).unwrap_or_default())
        })
        .collect();
    let mut errors = check_manifests(&workspace, &manifests);
    errors.extend(check_sources(&sources));
    if errors.is_empty() {
        println!(
            "FF-34 unsafe scope: ok ({} crates, {} source files)",
            manifests.len(),
            sources.len()
        );
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for p in entries.flatten().map(|e| e.path()) {
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// `key -> value` of the `[<prefix>.rust]` and `[<prefix>.clippy]` tables, keyed
/// `rust.<key>` / `clippy.<key>`, plus `workspace` from a plain `[<prefix>]` table.
fn lint_table(manifest: &str, prefix: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut section = String::new();
    for raw in manifest.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(s) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = s.trim().to_string();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let group = if section == prefix {
            ""
        } else if let Some(g) = section.strip_prefix(&format!("{prefix}.")) {
            g
        } else {
            continue;
        };
        let key = if group.is_empty() {
            k.trim().to_string()
        } else {
            format!("{group}.{}", k.trim())
        };
        out.insert(key, v.trim().replace(' ', ""));
    }
    out
}

pub fn check_manifests(workspace: &str, crates: &[(String, String)]) -> Vec<String> {
    let mut errors = Vec::new();
    let ws = lint_table(workspace, "workspace.lints");
    if ws.get("rust.unsafe_code").map(String::as_str) != Some("\"forbid\"") {
        errors.push("Cargo.toml: [workspace.lints.rust] must set unsafe_code = \"forbid\"".into());
    }
    for (name, manifest) in crates {
        let lints = lint_table(manifest, "lints");
        if name != FFI_CRATE {
            let uses_workspace = lints.get("workspace").map(String::as_str) == Some("true");
            if !uses_workspace || lints.len() != 1 {
                errors.push(format!(
                    "{name}/Cargo.toml: must use `[lints] workspace = true` and no crate-local lint table (ADR-022)"
                ));
            }
            continue;
        }
        if lints.contains_key("workspace") {
            errors.push(format!(
                "crates/{name}/Cargo.toml: must restate the workspace lints, not inherit them (ADR-022)"
            ));
        }
        for (key, value) in &ws {
            let expected = if key == "rust.unsafe_code" {
                "\"deny\""
            } else {
                value.as_str()
            };
            if lints.get(key).map(String::as_str) != Some(expected) {
                errors.push(format!(
                    "crates/{name}/Cargo.toml: lint {key} must be {expected} (workspace table, ADR-022)"
                ));
            }
        }
        for key in lints.keys() {
            if !ws.contains_key(key) && key != "rust.unexpected_cfgs" {
                errors.push(format!(
                    "crates/{name}/Cargo.toml: lint {key} is not in the workspace table (ADR-022)"
                ));
            }
        }
    }
    errors
}

/// Lint attributes that mention `unsafe_code` (allow, expect, warn, inner or outer).
fn mentions_unsafe_lint(code: &str) -> bool {
    let code = code.split("//").next().unwrap_or("").trim_start();
    code.starts_with('#')
        && code.contains("unsafe_code")
        && ["allow(", "expect(", "warn(", "deny("]
            .iter()
            .any(|a| code.contains(a))
}

pub fn check_sources(files: &[(String, String)]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut permitted = 0;
    for (path, text) in files {
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if !mentions_unsafe_lint(line) {
                continue;
            }
            let next_code = lines[i + 1..]
                .iter()
                .map(|l| l.trim())
                .find(|l| !l.is_empty() && !l.starts_with("//"));
            let is_permitted = path == FFI_LIB
                && line.trim() == "#[allow(unsafe_code)]"
                && next_code == Some("mod frb_generated;");
            if is_permitted {
                permitted += 1;
            } else {
                errors.push(format!(
                    "{path}:{}: lint attribute on unsafe_code is only allowed on `mod frb_generated;` in {FFI_LIB} (ADR-022)",
                    i + 1
                ));
            }
        }
    }
    if permitted > 1 {
        errors.push(format!("{FFI_LIB}: more than one allow(unsafe_code)"));
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    const WS: &str = "[workspace.lints.rust]\nunsafe_code = \"forbid\"\nmissing_debug_implementations = \"warn\"\n\n[workspace.lints.clippy]\ndbg_macro = \"warn\"\n";
    const FFI: &str = "[package]\nname = \"mc-ffi\"\n\n[lints.rust]\nunsafe_code = \"deny\"\nmissing_debug_implementations = \"warn\"\nunexpected_cfgs = { level = \"warn\" }\n\n[lints.clippy]\ndbg_macro = \"warn\"\n";
    const DOMAIN: &str = "[package]\nname = \"mc-score\"\n\n[lints]\nworkspace = true\n";

    fn c(name: &str, text: &str) -> (String, String) {
        (name.to_string(), text.to_string())
    }

    #[test]
    fn manifests_follow_adr_022() {
        assert!(check_manifests(WS, &[c("mc-score", DOMAIN), c("mc-ffi", FFI)]).is_empty());
        // A domain crate with its own lint table, or without workspace lints, fails.
        let own = "[lints.rust]\nunsafe_code = \"deny\"\n";
        assert_eq!(check_manifests(WS, &[c("mc-score", own)]).len(), 1);
        assert_eq!(
            check_manifests(WS, &[c("mc-score", "[package]\n")]).len(),
            1
        );
        // mc-ffi drifting from the workspace table fails.
        let drift = FFI.replace("dbg_macro = \"warn\"", "dbg_macro = \"allow\"");
        assert_eq!(check_manifests(WS, &[c("mc-ffi", &drift)]).len(), 1);
        let allow_all = FFI.replace("\"deny\"", "\"allow\"");
        assert_eq!(check_manifests(WS, &[c("mc-ffi", &allow_all)]).len(), 1);
        let extra = format!("{FFI}needless_return = \"allow\"\n");
        assert_eq!(check_manifests(WS, &[c("mc-ffi", &extra)]).len(), 1);
        // The workspace itself must keep forbid.
        assert!(!check_manifests(&WS.replace("forbid", "deny"), &[]).is_empty());
    }

    #[test]
    fn only_the_generated_module_may_allow_unsafe() {
        let ok = "pub mod api;\n\n// ADR-022\n#[allow(unsafe_code)]\nmod frb_generated;\n";
        assert!(check_sources(&[c(FFI_LIB, ok)]).is_empty());
        for bad in [
            "#[allow(unsafe_code)]\nmod api;\n",
            "#![allow(unsafe_code)]\n",
            "#[expect(unsafe_code)]\nfn f() {}\n",
            "#[allow(clippy::x, unsafe_code)]\nmod frb_generated;\n",
        ] {
            assert_eq!(check_sources(&[c(FFI_LIB, bad)]).len(), 1, "{bad}");
        }
        assert_eq!(
            check_sources(&[c("crates/mc-score/src/lib.rs", ok)]).len(),
            1,
            "the allow is only valid in mc-ffi"
        );
        assert!(
            check_sources(&[c(
                "crates/mc-score/src/lib.rs",
                "// allow(unsafe_code) in a comment\n"
            )])
            .is_empty()
        );
    }

    #[test]
    fn repository_passes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        run(root).unwrap();
    }
}
