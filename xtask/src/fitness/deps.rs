//! FF-01: every workspace crate is declared in architecture/allowed-deps.toml and
//! depends only on the internal and external crates allowed for it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const MATRIX: &str = "architecture/allowed-deps.toml";
const CRATES_DIR: &str = "crates";

#[derive(Debug, Default, Clone)]
pub struct Rule {
    pub layer: String,
    pub internal: BTreeSet<String>,
    pub external: BTreeSet<String>,
    pub dev_external: BTreeSet<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    /// Dependencies from [dependencies], [build-dependencies] and target-specific variants.
    pub normal: BTreeSet<String>,
    /// Dependencies from [dev-dependencies] and target-specific variants.
    pub dev: BTreeSet<String>,
}

pub fn run(root: &Path) -> Result<(), String> {
    let rules = parse_matrix(&read(root, MATRIX)?)?;
    let manifests = workspace_manifests(root)?;
    let errors = check(&rules, &manifests);
    if errors.is_empty() {
        println!(
            "FF-01 deps: ok ({} crates checked, {} declared)",
            manifests.len(),
            rules.len()
        );
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    fs::read_to_string(root.join(rel)).map_err(|e| format!("cannot read {rel}: {e}"))
}

/// Reads crates/*/Cargo.toml (production crates). xtask is tooling and not part of the matrix.
pub fn workspace_manifests(root: &Path) -> Result<Vec<Manifest>, String> {
    let dir = root.join(CRATES_DIR);
    let Ok(entries) = fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let mut paths: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("Cargo.toml").is_file())
        .collect();
    paths.sort();
    for p in paths {
        let text = fs::read_to_string(p.join("Cargo.toml"))
            .map_err(|e| format!("{}: {e}", p.display()))?;
        let m = parse_manifest(&text).map_err(|e| format!("{}: {e}", p.display()))?;
        let dir_name = p.file_name().unwrap().to_string_lossy().into_owned();
        if m.name != dir_name {
            return Err(format!(
                "crate in {CRATES_DIR}/{dir_name} is named '{}'; directory and package name must match",
                m.name
            ));
        }
        out.push(m);
    }
    Ok(out)
}

pub fn check(rules: &BTreeMap<String, Rule>, manifests: &[Manifest]) -> Vec<String> {
    let mut errors = Vec::new();
    for m in manifests {
        let Some(rule) = rules.get(&m.name) else {
            errors.push(format!("{} is not declared in {MATRIX}", m.name));
            continue;
        };
        for dep in m.normal.iter().chain(&m.dev) {
            if dep == "xtask" {
                errors.push(format!(
                    "{} depends on xtask (tooling must not be a dependency)",
                    m.name
                ));
            }
        }
        for dep in &m.normal {
            let internal = rules.contains_key(dep);
            if internal && !rule.internal.contains(dep) {
                errors.push(format!(
                    "{} -> {dep}: internal dependency not allowed",
                    m.name
                ));
            } else if !internal && !rule.external.contains(dep) {
                errors.push(format!(
                    "{} -> {dep}: external dependency not allowed ({} layer)",
                    m.name, rule.layer
                ));
            }
        }
        for dep in &m.dev {
            let internal = rules.contains_key(dep);
            if internal && !rule.internal.contains(dep) {
                errors.push(format!(
                    "{} -> {dep} (dev): internal dependency not allowed",
                    m.name
                ));
            } else if !internal && !rule.external.contains(dep) && !rule.dev_external.contains(dep)
            {
                errors.push(format!(
                    "{} -> {dep} (dev): external dev-dependency not allowed",
                    m.name
                ));
            }
        }
    }
    errors
}

/// Parses the matrix: `[crate]` sections with `layer`, `phase`, and single-line string arrays.
pub fn parse_matrix(text: &str) -> Result<BTreeMap<String, Rule>, String> {
    let mut rules: BTreeMap<String, Rule> = BTreeMap::new();
    let mut current: Option<String> = None;
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if rules.insert(name.to_string(), Rule::default()).is_some() {
                return Err(format!("{MATRIX}:{}: duplicate section [{name}]", n + 1));
            }
            current = Some(name.to_string());
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("{MATRIX}:{}: expected key = value", n + 1))?;
        let rule = current
            .as_ref()
            .and_then(|c| rules.get_mut(c))
            .ok_or_else(|| format!("{MATRIX}:{}: key outside a section", n + 1))?;
        let value = value.trim();
        match key.trim() {
            "layer" => {
                rule.layer = value.trim_matches('"').to_string();
                if !matches!(rule.layer.as_str(), "domain" | "application" | "adapter") {
                    return Err(format!(
                        "{MATRIX}:{}: unknown layer '{}'",
                        n + 1,
                        rule.layer
                    ));
                }
            }
            "phase" => {
                value
                    .parse::<u32>()
                    .map_err(|_| format!("{MATRIX}:{}: phase must be an integer", n + 1))?;
            }
            "internal" => {
                rule.internal =
                    parse_array(value).map_err(|e| format!("{MATRIX}:{}: {e}", n + 1))?
            }
            "external" => {
                rule.external =
                    parse_array(value).map_err(|e| format!("{MATRIX}:{}: {e}", n + 1))?
            }
            "dev_external" => {
                rule.dev_external =
                    parse_array(value).map_err(|e| format!("{MATRIX}:{}: {e}", n + 1))?
            }
            other => return Err(format!("{MATRIX}:{}: unknown key '{other}'", n + 1)),
        }
    }
    for (name, rule) in &rules {
        if rule.layer.is_empty() {
            return Err(format!("[{name}] has no layer"));
        }
        if let Some(bad) = rule.internal.iter().find(|d| !rules.contains_key(*d)) {
            return Err(format!(
                "[{name}] internal dependency '{bad}' is not declared in the matrix"
            ));
        }
    }
    Ok(rules)
}

fn parse_array(value: &str) -> Result<BTreeSet<String>, String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .ok_or("expected a single-line array like [\"a\", \"b\"]")?;
    Ok(inner
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// Extracts the package name and dependency names from a crate manifest.
/// Supports `name = ...`, `name.workspace = true`, `[dependencies.name]` and
/// `[target.'cfg(..)'.dependencies]` forms. Renamed dependencies are rejected
/// because they would hide the real crate name from this check.
pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    #[derive(PartialEq)]
    enum Section {
        Package,
        Normal,
        Dev,
        Other,
    }
    let mut m = Manifest::default();
    let mut section = Section::Other;
    let mut table_dep: Option<(String, bool)> = None; // ([dependencies.x] name, is_dev)
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            table_dep = None;
            let header = line.trim_matches(|c| c == '[' || c == ']');
            let last = header.rsplit('.').next().unwrap_or(header);
            let kind = |s: &str| match s {
                "dependencies" | "build-dependencies" => Some(false),
                "dev-dependencies" => Some(true),
                _ => None,
            };
            section = if header == "package" {
                Section::Package
            } else if let Some(dev) = kind(last) {
                if dev { Section::Dev } else { Section::Normal }
            } else {
                // `[dependencies.name]` or `[target.x.dev-dependencies.name]`
                let parts: Vec<&str> = header.split('.').collect();
                match parts.iter().rposition(|p| kind(p).is_some()) {
                    Some(i) if i + 2 == parts.len() => {
                        let dev = kind(parts[i]).unwrap();
                        let name = parts[i + 1].to_string();
                        if dev {
                            m.dev.insert(name.clone())
                        } else {
                            m.normal.insert(name.clone())
                        };
                        table_dep = Some((name, dev));
                        Section::Other
                    }
                    _ => Section::Other,
                }
            };
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if let Some((name, _)) = &table_dep
            && key == "package"
        {
            return Err(format!("renamed dependency '{name}' is not allowed"));
        }
        match section {
            Section::Package if key == "name" => {
                m.name = value.trim().trim_matches('"').to_string()
            }
            Section::Normal | Section::Dev => {
                let name = key.split('.').next().unwrap().trim().to_string();
                if value.contains("package") && value.contains('{') {
                    return Err(format!("renamed dependency '{name}' is not allowed"));
                }
                if section == Section::Dev {
                    m.dev.insert(name)
                } else {
                    m.normal.insert(name)
                };
            }
            _ => {}
        }
    }
    if m.name.is_empty() {
        return Err("no [package] name".to_string());
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MATRIX_TEXT: &str = r#"
[mc-theory]
layer = "domain"
phase = 1
internal = []
external = []
dev_external = ["proptest"]

[mc-score]
layer = "domain"
phase = 1
internal = ["mc-theory"]
external = []

[mc-app]
layer = "application"
phase = 1
internal = ["mc-theory", "mc-score"]
external = []
"#;

    fn manifest(text: &str) -> Manifest {
        parse_manifest(text).unwrap()
    }

    #[test]
    fn parses_all_dependency_forms() {
        let m = manifest(
            r#"
[package]
name = "mc-score"
version = "0.1.0"

[dependencies]
mc-theory = { path = "../mc-theory" }
serde.workspace = true

[dev-dependencies]
proptest = { workspace = true }

[target.'cfg(windows)'.dependencies]
windows-sys = "0.61"

[dependencies.zip]
version = "9"
"#,
        );
        assert_eq!(m.name, "mc-score");
        assert_eq!(
            m.normal,
            ["mc-theory", "serde", "windows-sys", "zip"]
                .map(String::from)
                .into()
        );
        assert_eq!(m.dev, ["proptest"].map(String::from).into());
    }

    #[test]
    fn rejects_renamed_dependencies() {
        assert!(parse_manifest("[package]\nname = \"a\"\n[dependencies]\nfoo = { package = \"bar\", version = \"1\" }\n").is_err());
    }

    #[test]
    fn allowed_edges_pass() {
        let rules = parse_matrix(MATRIX_TEXT).unwrap();
        let m = manifest(
            "[package]\nname = \"mc-score\"\n[dependencies]\nmc-theory = { path = \"../mc-theory\" }\n",
        );
        assert!(check(&rules, &[m]).is_empty());
    }

    #[test]
    fn domain_depending_on_application_fails() {
        let rules = parse_matrix(MATRIX_TEXT).unwrap();
        let m = manifest(
            "[package]\nname = \"mc-score\"\n[dependencies]\nmc-app = { path = \"../mc-app\" }\n",
        );
        let e = check(&rules, &[m]);
        assert!(e.iter().any(|x| x.contains("mc-score -> mc-app")), "{e:?}");
    }

    #[test]
    fn unlisted_external_fails_and_dev_external_is_dev_only() {
        let rules = parse_matrix(MATRIX_TEXT).unwrap();
        let normal =
            manifest("[package]\nname = \"mc-theory\"\n[dependencies]\nproptest = \"1\"\n");
        assert_eq!(check(&rules, &[normal]).len(), 1);
        let dev =
            manifest("[package]\nname = \"mc-theory\"\n[dev-dependencies]\nproptest = \"1\"\n");
        assert!(check(&rules, &[dev]).is_empty());
    }

    #[test]
    fn undeclared_crate_fails() {
        let rules = parse_matrix(MATRIX_TEXT).unwrap();
        let m = manifest("[package]\nname = \"mc-sneaky\"\n");
        assert!(check(&rules, &[m])[0].contains("not declared"));
    }

    #[test]
    fn matrix_internal_references_must_exist() {
        assert!(parse_matrix("[a]\nlayer = \"domain\"\ninternal = [\"ghost\"]\n").is_err());
    }

    #[test]
    fn repository_matrix_and_crates_pass() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        run(root).unwrap();
    }
}
