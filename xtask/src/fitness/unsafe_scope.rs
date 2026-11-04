//! FF-34 (ADR-022, ADR-027): unsafe code is confined to two reviewed places.
//!
//! - Every crate under `crates/` except the two exception crates, and `xtask`, uses
//!   `[lints] workspace = true`, so the workspace `unsafe_code = "forbid"` applies, and
//!   has no crate-local lint table.
//! - An exception crate has its own lint table equal to `[workspace.lints]` except
//!   `unsafe_code = "deny"`; the only extra key allowed is listed per exception
//!   (`unexpected_cfgs` for `mc-ffi`, needed for `cfg(frb_expand)` by the generator;
//!   none for `mc-audio`).
//! - The only lint attributes naming `unsafe_code` in hand-written sources are
//!   `#[allow(unsafe_code)]` directly on `mod frb_generated;` in
//!   `crates/mc-ffi/src/lib.rs` (ADR-022) and directly on `mod mmcss;` in
//!   `crates/mc-audio/src/lib.rs` (ADR-027). The generated file is checked by FF-12.
//! - ADR-027 scope: `crates/mc-audio/src/mmcss.rs` has exactly one `unsafe` token outside
//!   comments, and a `// SAFETY:` comment sits directly above it.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// One reviewed place where `allow(unsafe_code)` is permitted.
struct Exception {
    krate: &'static str,
    lib: &'static str,
    /// The declaration the attribute must sit directly on.
    module_decl: &'static str,
    /// Lint keys allowed in the crate's table besides the workspace keys.
    extra_lints: &'static [&'static str],
    adr: &'static str,
}

const EXCEPTIONS: &[Exception] = &[
    Exception {
        krate: "mc-ffi",
        lib: "crates/mc-ffi/src/lib.rs",
        module_decl: "mod frb_generated;",
        extra_lints: &["rust.unexpected_cfgs"],
        adr: "ADR-022",
    },
    Exception {
        krate: "mc-audio",
        lib: "crates/mc-audio/src/lib.rs",
        module_decl: "mod mmcss;",
        extra_lints: &[],
        adr: "ADR-027",
    },
];

const GENERATED: &str = "frb_generated.rs";
/// The one hand-written file allowed to contain `unsafe` (ADR-027).
const MMCSS_FILE: &str = "crates/mc-audio/src/mmcss.rs";

fn exception_for(krate: &str) -> Option<&'static Exception> {
    EXCEPTIONS.iter().find(|e| e.krate == krate)
}

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
    errors.extend(check_mmcss(&sources));
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
        let Some(exception) = exception_for(name) else {
            let uses_workspace = lints.get("workspace").map(String::as_str) == Some("true");
            if !uses_workspace || lints.len() != 1 {
                errors.push(format!(
                    "{name}/Cargo.toml: must use `[lints] workspace = true` and no crate-local lint table (ADR-022, ADR-027)"
                ));
            }
            continue;
        };
        let adr = exception.adr;
        if lints.contains_key("workspace") {
            errors.push(format!(
                "crates/{name}/Cargo.toml: must restate the workspace lints, not inherit them ({adr})"
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
                    "crates/{name}/Cargo.toml: lint {key} must be {expected} (workspace table, {adr})"
                ));
            }
        }
        for key in lints.keys() {
            if !ws.contains_key(key) && !exception.extra_lints.contains(&key.as_str()) {
                errors.push(format!(
                    "crates/{name}/Cargo.toml: lint {key} is not in the workspace table ({adr})"
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
    let mut permitted: BTreeMap<&str, usize> = BTreeMap::new();
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
            let exception = EXCEPTIONS.iter().find(|e| {
                path == e.lib
                    && line.trim() == "#[allow(unsafe_code)]"
                    && next_code == Some(e.module_decl)
            });
            match exception {
                Some(e) => *permitted.entry(e.krate).or_insert(0) += 1,
                None => errors.push(format!(
                    "{path}:{}: lint attribute on unsafe_code is only allowed on `mod frb_generated;` in crates/mc-ffi/src/lib.rs (ADR-022) and on `mod mmcss;` in crates/mc-audio/src/lib.rs (ADR-027)",
                    i + 1
                )),
            }
        }
    }
    for e in EXCEPTIONS {
        if permitted.get(e.krate).copied().unwrap_or(0) > 1 {
            errors.push(format!("{}: more than one allow(unsafe_code)", e.lib));
        }
    }
    errors
}

/// A source line without its `//` comment (doc comments included).
fn code_part(line: &str) -> &str {
    line.split("//").next().unwrap_or("")
}

/// Number of `unsafe` word tokens outside comments.
fn count_unsafe_tokens(text: &str) -> usize {
    let mut n = 0;
    for line in text.lines() {
        let code = code_part(line);
        let mut start = 0;
        while let Some(rel) = code[start..].find("unsafe") {
            let idx = start + rel;
            let before = code[..idx].chars().last();
            let after = code[idx + 6..].chars().next();
            let boundary = |c: Option<char>| c.is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
            if boundary(before) && boundary(after) {
                n += 1;
            }
            start = idx + 6;
        }
    }
    n
}

/// ADR-027: the MMCSS module has exactly one `unsafe` and a `// SAFETY:` comment
/// directly above the line that holds it.
pub fn check_mmcss(files: &[(String, String)]) -> Vec<String> {
    let Some((_, text)) = files.iter().find(|(p, _)| p == MMCSS_FILE) else {
        return Vec::new();
    };
    let mut errors = Vec::new();
    let count = count_unsafe_tokens(text);
    if count != 1 {
        errors.push(format!(
            "{MMCSS_FILE}: must contain exactly one `unsafe` outside comments, found {count} (ADR-027)"
        ));
    }
    let lines: Vec<&str> = text.lines().collect();
    if let Some(at) = lines.iter().position(|l| count_unsafe_tokens(l) > 0) {
        let mut has_safety = false;
        for l in lines[..at].iter().rev() {
            let t = l.trim();
            if !t.starts_with("//") {
                break;
            }
            has_safety |= t.contains("SAFETY:");
        }
        if !has_safety {
            errors.push(format!(
                "{MMCSS_FILE}:{}: the `unsafe` needs a `// SAFETY:` comment directly above it (ADR-027)",
                at + 1
            ));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    const WS: &str = "[workspace.lints.rust]\nunsafe_code = \"forbid\"\nmissing_debug_implementations = \"warn\"\n\n[workspace.lints.clippy]\ndbg_macro = \"warn\"\n";
    const FFI: &str = "[package]\nname = \"mc-ffi\"\n\n[lints.rust]\nunsafe_code = \"deny\"\nmissing_debug_implementations = \"warn\"\nunexpected_cfgs = { level = \"warn\" }\n\n[lints.clippy]\ndbg_macro = \"warn\"\n";
    const AUDIO: &str = "[package]\nname = \"mc-audio\"\n\n[lints.rust]\nunsafe_code = \"deny\"\nmissing_debug_implementations = \"warn\"\n\n[lints.clippy]\ndbg_macro = \"warn\"\n";
    const DOMAIN: &str = "[package]\nname = \"mc-score\"\n\n[lints]\nworkspace = true\n";
    const FFI_LIB: &str = "crates/mc-ffi/src/lib.rs";
    const AUDIO_LIB: &str = "crates/mc-audio/src/lib.rs";

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
    fn mc_audio_follows_adr_027() {
        let errs = check_manifests(WS, &[c("mc-audio", AUDIO)]);
        assert!(errs.is_empty(), "{errs:?}");
        // Inheriting the workspace lints would keep forbid, which cannot compile mmcss.
        let inherits = "[lints]\nworkspace = true\n";
        let errs = check_manifests(WS, &[c("mc-audio", inherits)]);
        assert!(
            errs.iter()
                .any(|e| e.contains("must restate the workspace lints")),
            "{errs:?}"
        );
        // No extra keys, not even the one mc-ffi may have.
        let with_cfgs = AUDIO.replace(
            "missing_debug_implementations = \"warn\"",
            "missing_debug_implementations = \"warn\"\nunexpected_cfgs = { level = \"warn\" }",
        );
        assert_eq!(check_manifests(WS, &[c("mc-audio", &with_cfgs)]).len(), 1);
        let drift = AUDIO.replace("dbg_macro = \"warn\"", "dbg_macro = \"allow\"");
        assert_eq!(check_manifests(WS, &[c("mc-audio", &drift)]).len(), 1);
        let allow_all = AUDIO.replace("\"deny\"", "\"allow\"");
        assert_eq!(check_manifests(WS, &[c("mc-audio", &allow_all)]).len(), 1);
    }

    #[test]
    fn only_the_reviewed_modules_may_allow_unsafe() {
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
    fn the_mmcss_allow_is_valid_only_in_mc_audio_on_that_module() {
        let ok = "mod error;\n\n// ADR-027\n#[cfg(windows)]\n#[allow(unsafe_code)]\nmod mmcss;\n";
        assert!(check_sources(&[c(AUDIO_LIB, ok)]).is_empty());
        // The same attribute on another module of mc-audio, or in another crate, fails.
        let other = "#[allow(unsafe_code)]\nmod platform;\n";
        assert_eq!(check_sources(&[c(AUDIO_LIB, other)]).len(), 1);
        assert_eq!(check_sources(&[c(FFI_LIB, ok)]).len(), 1);
        assert_eq!(
            check_sources(&[c("crates/mc-audio/src/rt/mod.rs", ok)]).len(),
            1
        );
        // mc-ffi may not use the mc-audio module name and vice versa.
        let ffi_style = "#[allow(unsafe_code)]\nmod frb_generated;\n";
        assert_eq!(check_sources(&[c(AUDIO_LIB, ffi_style)]).len(), 1);
        // Two allows in one exception crate are one too many.
        let twice = format!("{ok}{ok}");
        assert_eq!(check_sources(&[c(AUDIO_LIB, &twice)]).len(), 1);
    }

    #[test]
    fn the_mmcss_file_has_one_unsafe_with_a_safety_comment() {
        let good = "// notes\n// SAFETY: the pointer is valid.\nlet h = unsafe { call() };\n";
        assert!(check_mmcss(&[c(MMCSS_FILE, good)]).is_empty());
        // The word in comments does not count.
        let commented = format!("// this is not unsafe code\n{good}");
        assert!(check_mmcss(&[c(MMCSS_FILE, &commented)]).is_empty());
        // No SAFETY comment, a blank line between, two blocks, or none at all fail.
        let no_safety = "let h = unsafe { call() };\n";
        assert_eq!(check_mmcss(&[c(MMCSS_FILE, no_safety)]).len(), 1);
        let gap = "// SAFETY: ok\n\nlet h = unsafe { call() };\n";
        assert_eq!(check_mmcss(&[c(MMCSS_FILE, gap)]).len(), 1);
        let two = format!("{good}{good}");
        assert_eq!(check_mmcss(&[c(MMCSS_FILE, &two)]).len(), 1);
        assert_eq!(check_mmcss(&[c(MMCSS_FILE, "fn f() {}\n")]).len(), 1);
        // Identifiers that merely contain the word are not counted.
        assert!(
            check_mmcss(&[c(MMCSS_FILE, &format!("let unsafe_count = 1;\n{good}"))]).is_empty()
        );
        // Other files are not this check's concern.
        assert!(check_mmcss(&[c("crates/mc-audio/src/other.rs", "unsafe {}")]).is_empty());
    }

    #[test]
    fn repository_passes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        run(root).unwrap();
    }
}
