//! Which autosaves to offer on startup (project-format section 12, ADR-030 section 2).
//!
//! The rule lives here, not in the app (GR-05): recovery is offered only after an
//! unclean session, and only for autosaves newer than their source file. File access
//! is passed in, so the rule is tested without a file system.

use crate::ports::RecoveryEntry;
use std::path::Path;

/// Filters `entries` down to the ones to offer.
///
/// - After a clean session (`unclean == false`) nothing is offered.
/// - An untitled document (no source) is always offered: the autosave is its only copy.
/// - A source whose modification time is unknown (deleted, moved, unreadable) is
///   offered: the autosave may be the only copy left.
/// - Otherwise the entry is offered only if it was written after the source was last
///   modified; an older autosave is superseded by the saved file.
pub fn offered(
    unclean: bool,
    entries: Vec<RecoveryEntry>,
    source_modified_millis: impl Fn(&Path) -> Option<u64>,
) -> Vec<RecoveryEntry> {
    if !unclean {
        return Vec::new();
    }
    entries
        .into_iter()
        .filter(|e| match e.source.as_deref() {
            None => true,
            Some(src) => {
                source_modified_millis(src).is_none_or(|modified| e.written_unix_millis > modified)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(id: &str, source: Option<&str>, written: u64) -> RecoveryEntry {
        RecoveryEntry {
            project_id: id.into(),
            path: PathBuf::from(format!("{id}/autosave.recovery")),
            source: source.map(PathBuf::from),
            written_unix_millis: written,
        }
    }

    fn ids(v: &[RecoveryEntry]) -> Vec<&str> {
        v.iter().map(|e| e.project_id.as_str()).collect()
    }

    fn all() -> Vec<RecoveryEntry> {
        vec![
            entry("untitled", None, 100),
            entry("newer", Some("a.giproj"), 200),
            entry("older", Some("b.giproj"), 50),
            entry("same", Some("c.giproj"), 70),
            entry("missing", Some("gone.giproj"), 10),
        ]
    }

    fn mtime(p: &Path) -> Option<u64> {
        match p.to_str()? {
            "a.giproj" => Some(150),
            "b.giproj" => Some(60),
            "c.giproj" => Some(70),
            _ => None,
        }
    }

    #[test]
    fn c7_nothing_is_offered_after_a_clean_session() {
        assert!(offered(false, all(), mtime).is_empty());
    }

    #[test]
    fn c8_only_entries_newer_than_their_source_are_offered() {
        let got = offered(true, all(), mtime);
        assert_eq!(ids(&got), vec!["untitled", "newer", "missing"]);
    }
}
