//! FF-33 (link part): every relative Markdown link in docs/ and .kiro/steering/
//! resolves to an existing file. Mermaid syntax is checked by tools/mermaid-check.

use std::fs;
use std::path::{Path, PathBuf};

const ROOTS: &[&str] = &["docs", ".kiro/steering"];

pub fn run(root: &Path) -> Result<(), String> {
    let mut files = Vec::new();
    for r in ROOTS {
        collect_md(&root.join(r), &mut files);
    }
    let mut broken = Vec::new();
    let mut links = 0usize;
    for file in &files {
        let text = fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
        for target in relative_links(&text) {
            links += 1;
            let path = target.split('#').next().unwrap_or("");
            if !path.is_empty() && !file.parent().unwrap().join(path).exists() {
                let rel = file.strip_prefix(root).unwrap_or(file);
                broken.push(format!("{} -> {target}", rel.display()));
            }
        }
    }
    if broken.is_empty() {
        println!(
            "FF-33 docs-links: ok ({} files, {links} relative links)",
            files.len()
        );
        Ok(())
    } else {
        Err(format!("broken links:\n{}", broken.join("\n")))
    }
}

fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect_md(&p, out);
        } else if p.extension().is_some_and(|e| e == "md") {
            out.push(p);
        }
    }
}

/// Targets of `[text](target)` links outside fenced code blocks, excluding
/// absolute URLs and pure anchors.
pub fn relative_links(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut rest = line;
        while let Some(i) = rest.find("](") {
            rest = &rest[i + 2..];
            let Some(end) = rest.find(')') else { break };
            let target = rest[..end]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            rest = &rest[end..];
            let external = ["http://", "https://", "mailto:", "kiro-artifact:"]
                .iter()
                .any(|p| target.starts_with(p));
            if !target.is_empty() && !external && !target.starts_with('#') {
                out.push(target);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_relative_links_only() {
        let md = "See [a](a.md), [b](../b.md#x), [web](https://x.y) and [anchor](#top).\n\
                  ```\n[code](ignored.md)\n```\n";
        assert_eq!(relative_links(md), ["a.md", "../b.md#x"]);
    }

    #[test]
    fn repository_docs_have_no_broken_links() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        run(root).unwrap();
    }
}
