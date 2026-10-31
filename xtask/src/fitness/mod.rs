//! Architecture fitness functions active in Phase 0.
//! Catalogue: docs/architecture/fitness-functions.md. Inventory: architecture/fitness-inventory.toml.

pub mod deps;
pub mod docs_links;
pub mod inventory;
pub mod purity;
pub mod reports;
pub mod trailers;

use std::path::Path;
use std::process::Command;

/// Runs git in `repo` and returns stdout, or an error with stderr.
pub fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Commits to check: `base..HEAD` when `base` names a commit, otherwise HEAD only.
/// An all-zero base (first push of a branch) counts as missing.
pub fn commits_to_check(repo: &Path, base: Option<&str>) -> Result<Vec<String>, String> {
    let usable = base.filter(|b| {
        !b.is_empty()
            && !b.chars().all(|c| c == '0')
            && git(repo, &["cat-file", "-e", &format!("{b}^{{commit}}")]).is_ok()
    });
    let out = match usable {
        Some(b) => git(repo, &["rev-list", &format!("{b}..HEAD")])?,
        None => git(repo, &["rev-list", "-1", "HEAD"])?,
    };
    Ok(out
        .lines()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .collect())
}

#[cfg(test)]
pub mod testutil {
    use super::git;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Temporary git repository, removed on drop.
    pub struct TempRepo {
        pub path: PathBuf,
    }

    impl TempRepo {
        pub fn new(name: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("xtask-{name}-{}-{nanos}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            git(&path, &["-c", "init.defaultBranch=main", "init", "-q"]).unwrap();
            TempRepo { path }
        }

        pub fn write(&self, rel: &str, content: &str) {
            let p = self.path.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, content).unwrap();
        }

        pub fn remove(&self, rel: &str) {
            fs::remove_file(self.path.join(rel)).unwrap();
        }

        /// Commits all changes and returns the commit id.
        pub fn commit(&self, message: &str) -> String {
            git(&self.path, &["add", "-A"]).unwrap();
            git(
                &self.path,
                &[
                    "-c",
                    "user.name=xtask-test",
                    "-c",
                    "user.email=xtask-test@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                    "commit",
                    "-q",
                    "--allow-empty",
                    "-m",
                    message,
                ],
            )
            .unwrap();
            git(&self.path, &["rev-parse", "HEAD"])
                .unwrap()
                .trim()
                .to_string()
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            clear_readonly(&self.path);
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    // Git marks object files read-only on Windows, which blocks remove_dir_all.
    fn clear_readonly(dir: &Path) {
        if let Ok(entries) = fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    clear_readonly(&p);
                } else if let Ok(meta) = fs::metadata(&p) {
                    let mut perms = meta.permissions();
                    #[allow(clippy::permissions_set_readonly_false)]
                    perms.set_readonly(false);
                    let _ = fs::set_permissions(&p, perms);
                }
            }
        }
    }
}
