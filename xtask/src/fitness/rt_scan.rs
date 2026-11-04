//! FF-17 (GR-10, ADR-007, ADR-024): lint scan over `crates/mc-audio/src/rt/`, the audio
//! callback core. Rejects constructs that allocate, lock, block, do I/O, log or format
//! inside the real-time path. This complements FF-16 (`assert_no_alloc` at test run
//! time) with a static check that runs without executing anything.
//!
//! Scope: every `.rs` file directly under `crates/mc-audio/src/rt/` (recursively, so a
//! future submodule is covered). Comments and string literals are stripped first so a
//! forbidden word inside a doc comment or a message string does not trigger a false
//! positive; this is a word-boundary scan, not a parser, so it can miss constructs
//! reached only through a type alias or a macro. The review checklist in
//! docs/architecture/audio-dsp-architecture.md section 2 covers what this cannot see
//! (for example lock-free but blocking spins, or FFI calls hidden behind a trait).

use std::fs;
use std::path::{Path, PathBuf};

const RT_DIR: &str = "crates/mc-audio/src/rt";

/// `(pattern, reason)`. Patterns are matched as identifiers/paths, not substrings, so
/// `Vec::new` does not match `vec_arena` and `format!` does not match `formatted`.
const FORBIDDEN: &[(&str, &str)] = &[
    (
        "Mutex",
        "locks may block; use atomics or wait-free structures",
    ),
    (
        "RwLock",
        "locks may block; use atomics or wait-free structures",
    ),
    ("println!", "no I/O or logging in the real-time path"),
    ("print!", "no I/O or logging in the real-time path"),
    ("eprintln!", "no I/O or logging in the real-time path"),
    ("eprint!", "no I/O or logging in the real-time path"),
    ("format!", "formatting allocates"),
    ("vec!", "allocates"),
    ("Vec::push", "may reallocate"),
    (
        "Vec::new",
        "allocates when first used; preallocate outside the callback",
    ),
    ("Box::new", "allocates"),
    ("String::new", "allocates when first used"),
    ("String::from", "allocates"),
    ("std::fs", "no file I/O in the real-time path"),
    ("std::net", "no network I/O in the real-time path"),
    ("thread::sleep", "blocks"),
    ("tracing::", "logging may allocate or block"),
    ("log::", "logging may allocate or block"),
];

/// Strips `//` and `/* */` comments and `"..."` string contents (kept as empty strings)
/// so forbidden words inside them are not flagged. Not a full lexer: raw strings and
/// nested block comments are treated approximately, which is acceptable for a lint that
/// only narrows where a human reviews (see module docs).
fn strip_comments_and_strings(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                while let Some(&n) = chars.peek() {
                    if n == '\n' {
                        break;
                    }
                    chars.next();
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut depth = 1;
                while depth > 0 {
                    match chars.next() {
                        Some('*') if chars.peek() == Some(&'/') => {
                            chars.next();
                            depth -= 1;
                        }
                        Some('/') if chars.peek() == Some(&'*') => {
                            chars.next();
                            depth += 1;
                        }
                        Some(_) => {}
                        None => break,
                    }
                }
            }
            '"' => {
                out.push('"');
                while let Some(&n) = chars.peek() {
                    chars.next();
                    if n == '\\' {
                        chars.next();
                        continue;
                    }
                    if n == '"' {
                        break;
                    }
                }
                out.push('"');
            }
            _ => out.push(c),
        }
    }
    out
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// True if `pattern` occurs in `text` at a word boundary on both sides (so `Vec::new`
/// does not match inside `MyVec::new_thing`, and a forbidden path matches whether or not
/// it is written with a leading `::`).
fn contains_as_word(text: &str, pattern: &str) -> bool {
    let bytes = text.as_bytes();
    let plen = pattern.len();
    if plen == 0 || plen > bytes.len() {
        return false;
    }
    let first = pattern.chars().next().unwrap();
    let pat_starts_ident = is_ident_char(first);
    let mut start = 0;
    while let Some(rel) = text[start..].find(pattern) {
        let idx = start + rel;
        let before_ok = if pat_starts_ident {
            idx == 0 || !is_ident_char(text[..idx].chars().last().unwrap_or(' '))
        } else {
            true
        };
        let end = idx + plen;
        let last = pattern.chars().last().unwrap();
        let pat_ends_ident = is_ident_char(last);
        let after_ok = if pat_ends_ident {
            end >= text.len() || !is_ident_char(text[end..].chars().next().unwrap_or(' '))
        } else {
            true
        };
        if before_ok && after_ok {
            return true;
        }
        start = idx + 1;
        if start >= text.len() {
            break;
        }
    }
    false
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            collect_rs(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

pub fn run(root: &Path) -> Result<(), String> {
    let dir = root.join(RT_DIR);
    if !dir.is_dir() {
        return Err(format!("{RT_DIR} does not exist"));
    }
    let mut files = Vec::new();
    collect_rs(&dir, &mut files)?;
    files.sort();
    if files.is_empty() {
        return Err(format!("{RT_DIR} has no .rs files to check"));
    }
    let mut violations = Vec::new();
    for path in &files {
        let raw = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let cleaned = strip_comments_and_strings(&raw);
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for (pattern, reason) in FORBIDDEN {
            if contains_as_word(&cleaned, pattern) {
                violations.push(format!("{rel}: forbidden `{pattern}` ({reason})"));
            }
        }
    }
    if violations.is_empty() {
        println!("FF-17 rt-scan: ok ({} files in {RT_DIR})", files.len());
        Ok(())
    } else {
        Err(violations.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_line_and_block_comments() {
        let s = strip_comments_and_strings("let x = 1; // Vec::new\n/* Mutex */ let y = 2;");
        assert!(!s.contains("Vec::new"));
        assert!(!s.contains("Mutex"));
        assert!(s.contains("let x = 1;"));
        assert!(s.contains("let y = 2;"));
    }

    #[test]
    fn strips_string_contents_but_keeps_quotes() {
        let s = strip_comments_and_strings(r#"let s = "contains Mutex and \" escaped";"#);
        assert!(!s.contains("Mutex"));
        assert!(s.contains("\"\""));
    }

    #[test]
    fn word_boundary_matching_avoids_false_positives() {
        assert!(!contains_as_word("let my_vec_new = 1;", "Vec::new"));
        assert!(contains_as_word("let v = Vec::new();", "Vec::new"));
        assert!(!contains_as_word("formatted_output()", "format!"));
        assert!(contains_as_word("format!(\"x\")", "format!"));
        assert!(contains_as_word("println!(\"x\")", "println!"));
    }

    #[test]
    fn real_mc_audio_rt_module_is_clean() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        // This only runs meaningfully once mc-audio/src/rt exists; it is a smoke test
        // for the scanner itself using the real directory when present.
        if root.join(RT_DIR).is_dir() {
            run(root).expect("the real rt module must pass FF-17");
        }
    }

    #[test]
    fn detects_a_planted_violation() {
        let dir = std::env::temp_dir().join(format!("ff17-rt-scan-test-{}", std::process::id()));
        let nested = dir.join(RT_DIR);
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            nested.join("bad.rs"),
            "fn f() { let m = std::sync::Mutex::new(0); let _ = m; }",
        )
        .unwrap();
        let err = run(&dir).unwrap_err();
        assert!(err.contains("Mutex"), "{err}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_clean_directory_passes() {
        let dir = std::env::temp_dir().join(format!("ff17-rt-scan-clean-{}", std::process::id()));
        let nested = dir.join(RT_DIR);
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("ok.rs"), "fn f(x: i32) -> i32 { x + 1 }").unwrap();
        assert!(run(&dir).is_ok());
        fs::remove_dir_all(&dir).unwrap();
    }
}
