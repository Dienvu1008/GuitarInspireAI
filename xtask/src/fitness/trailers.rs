//! FF-19: commits that change protected paths must carry
//! `Architecture-Change: ADR-NNN` referencing an existing ADR file (GR-13, GR-14).

use super::{commits_to_check, git};
use std::fs;
use std::path::Path;

const PROTECTED_LIST: &str = "architecture/protected-paths.txt";
const ADR_DIR: &str = "docs/architecture/decisions";
const TRAILER: &str = "Architecture-Change:";

pub fn run(root: &Path, base: Option<&str>) -> Result<(), String> {
    let violations = check(root, base)?;
    if violations.is_empty() {
        println!("FF-19 trailers: ok");
        Ok(())
    } else {
        Err(violations.join("\n"))
    }
}

/// Returns one message per offending commit.
pub fn check(root: &Path, base: Option<&str>) -> Result<Vec<String>, String> {
    let prefixes = effective_protected(root, base)?;
    let mut violations = Vec::new();
    for sha in commits_to_check(root, base)? {
        let touched: Vec<String> = changed_files(root, &sha)?
            .into_iter()
            .filter(|f| is_protected(f, &prefixes))
            .collect();
        if touched.is_empty() {
            continue;
        }
        let message = git(root, &["log", "-1", "--format=%B", &sha])?;
        let refs = adr_refs(&message);
        let short = &sha[..sha.len().min(10)];
        if refs.is_empty() {
            violations.push(format!(
                "commit {short} changes protected paths without an '{TRAILER} ADR-NNN' trailer: {}",
                touched.join(", ")
            ));
        } else if let Some(missing) = refs.iter().find(|id| !adr_exists(root, id)) {
            violations.push(format!(
                "commit {short} references {missing}, but no {ADR_DIR}/{missing}-*.md exists"
            ));
        }
    }
    Ok(violations)
}

/// Union of the current list, the list at `base`, and the list file itself.
/// Using the base list prevents a commit from removing an entry and changing
/// the formerly protected file without a trailer.
pub fn effective_protected(root: &Path, base: Option<&str>) -> Result<Vec<String>, String> {
    let current = fs::read_to_string(root.join(PROTECTED_LIST))
        .map_err(|e| format!("cannot read {PROTECTED_LIST}: {e}"))?;
    let mut prefixes = parse_list(&current);
    if let Some(b) = base
        && let Ok(old) = git(root, &["show", &format!("{b}:{PROTECTED_LIST}")])
    {
        prefixes.extend(parse_list(&old));
    }
    prefixes.push(PROTECTED_LIST.to_string());
    prefixes.sort();
    prefixes.dedup();
    Ok(prefixes)
}

fn parse_list(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

pub fn is_protected(path: &str, prefixes: &[String]) -> bool {
    let path = path.replace('\\', "/");
    prefixes.iter().any(|p| {
        if p.ends_with('/') {
            path.starts_with(p.as_str())
        } else {
            path == *p
        }
    })
}

fn changed_files(root: &Path, sha: &str) -> Result<Vec<String>, String> {
    // -m lists merge changes against each parent; --root handles the first commit.
    let out = git(
        root,
        &[
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "-r",
            "-m",
            "--root",
            sha,
        ],
    )?;
    let mut files: Vec<String> = out
        .lines()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

/// ADR ids from trailer lines, for example `Architecture-Change: ADR-015, ADR-020`.
pub fn adr_refs(message: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for line in message.lines() {
        let Some(rest) = line.trim().strip_prefix(TRAILER) else {
            continue;
        };
        for token in rest.split(|c: char| c == ',' || c.is_whitespace()) {
            let digits = token.strip_prefix("ADR-").unwrap_or("");
            if digits.len() == 3 && digits.chars().all(|c| c.is_ascii_digit()) {
                ids.push(token.to_string());
            }
        }
    }
    ids
}

pub fn adr_exists(root: &Path, id: &str) -> bool {
    let prefix = format!("{id}-");
    fs::read_dir(root.join(ADR_DIR))
        .map(|entries| {
            entries.flatten().any(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                name.starts_with(&prefix) && name.ends_with(".md")
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fitness::testutil::TempRepo;

    fn setup(name: &str) -> (TempRepo, String) {
        let repo = TempRepo::new(name);
        repo.write(PROTECTED_LIST, "# comment\narchitecture/\ndeny.toml\n");
        repo.write(
            "docs/architecture/decisions/ADR-015-license.md",
            "# ADR-015\n",
        );
        repo.write("src/lib.rs", "// code\n");
        let base = repo.commit("base");
        (repo, base)
    }

    #[test]
    fn protected_change_without_trailer_fails() {
        let (repo, base) = setup("no-trailer");
        repo.write("deny.toml", "[licenses]\n");
        repo.commit("loosen license policy");
        let v = check(&repo.path, Some(&base)).unwrap();
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("deny.toml"));
    }

    #[test]
    fn protected_change_with_valid_trailer_passes() {
        let (repo, base) = setup("trailer-ok");
        repo.write("architecture/allowed-deps.toml", "x\n");
        repo.commit("add matrix\n\nArchitecture-Change: ADR-015\n");
        assert!(check(&repo.path, Some(&base)).unwrap().is_empty());
    }

    #[test]
    fn trailer_with_unknown_adr_fails() {
        let (repo, base) = setup("trailer-missing-adr");
        repo.write("deny.toml", "x\n");
        repo.commit("change\n\nArchitecture-Change: ADR-099\n");
        let v = check(&repo.path, Some(&base)).unwrap();
        assert_eq!(v.len(), 1);
        assert!(v[0].contains("ADR-099"));
    }

    #[test]
    fn unprotected_change_passes() {
        let (repo, base) = setup("unprotected");
        repo.write("src/lib.rs", "// changed\n");
        repo.commit("code change");
        assert!(check(&repo.path, Some(&base)).unwrap().is_empty());
    }

    #[test]
    fn emptying_the_protected_list_cannot_bypass_the_check() {
        let (repo, base) = setup("bypass");
        repo.write(PROTECTED_LIST, "");
        repo.write("deny.toml", "loosened\n");
        repo.commit("disable protection and loosen policy");
        let v = check(&repo.path, Some(&base)).unwrap();
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(
            v[0].contains("deny.toml") && v[0].contains(PROTECTED_LIST),
            "{v:?}"
        );
    }

    #[test]
    fn deleting_a_protected_file_is_a_protected_change() {
        let (repo, base) = setup("delete");
        repo.write("architecture/allowed-deps.toml", "x\n");
        let with_file = repo.commit("add\n\nArchitecture-Change: ADR-015\n");
        assert!(check(&repo.path, Some(&base)).unwrap().is_empty());
        repo.remove("architecture/allowed-deps.toml");
        repo.commit("remove matrix");
        let v = check(&repo.path, Some(&with_file)).unwrap();
        assert_eq!(v.len(), 1, "{v:?}");
    }

    #[test]
    fn without_base_only_head_is_checked() {
        let (repo, _) = setup("head-only");
        repo.write("deny.toml", "x\n");
        repo.commit("bad");
        repo.write("src/lib.rs", "// fine\n");
        repo.commit("fine");
        assert!(check(&repo.path, None).unwrap().is_empty());
    }

    #[test]
    fn trailer_parsing() {
        assert_eq!(
            adr_refs("x\n\nArchitecture-Change: ADR-001, ADR-020\n"),
            ["ADR-001", "ADR-020"]
        );
        assert!(adr_refs("Architecture-Change: ADR-1").is_empty());
        assert!(adr_refs("mentions ADR-001 in body").is_empty());
    }

    #[test]
    fn prefix_matching() {
        let p = vec!["architecture/".to_string(), "deny.toml".to_string()];
        assert!(is_protected("architecture/x.toml", &p));
        assert!(is_protected("deny.toml", &p));
        assert!(!is_protected("deny.toml.bak", &p));
        assert!(!is_protected("docs/architecture/x.md", &p));
    }
}
