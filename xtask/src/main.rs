//! Repository governance tooling.
//!
//! Usage:
//!   cargo xtask fitness all        [--base <rev>]
//!   cargo xtask fitness deps                        (FF-01)
//!   cargo xtask fitness purity                      (FF-03, config part; clippy enforces)
//!   cargo xtask fitness wasm                        (FF-04)
//!   cargo xtask fitness platform                    (FF-07)
//!   cargo xtask fitness trailers   [--base <rev>]   (FF-19)
//!   cargo xtask fitness inventory  [--base <rev>]   (FF-20)
//!   cargo xtask fitness docs-links                  (FF-33, link part)
//!
//! `--base` is the last commit already checked (for example the previous push or
//! the pull request base). Without it, only the HEAD commit is checked.

mod fitness;

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = workspace_root();
    match run(&root, &args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        }
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root")
        .to_path_buf()
}

fn run(root: &std::path::Path, args: &[String]) -> Result<(), String> {
    let (group, rest) = args.split_first().ok_or_else(usage)?;
    if group != "fitness" {
        return Err(usage());
    }
    let (cmd, rest) = rest.split_first().ok_or_else(usage)?;
    let base = parse_base(rest)?;
    let base = base.as_deref();
    match cmd.as_str() {
        "trailers" => fitness::trailers::run(root, base),
        "inventory" => fitness::inventory::run(root, base),
        "docs-links" => fitness::docs_links::run(root),
        "deps" => fitness::deps::run(root),
        "purity" => fitness::purity::run_purity(root),
        "wasm" => fitness::purity::run_wasm(root),
        "platform" => fitness::purity::run_platform(root),
        "all" => {
            let results = [
                ("FF-01 deps", fitness::deps::run(root)),
                ("FF-03 purity", fitness::purity::run_purity(root)),
                ("FF-04 wasm32", fitness::purity::run_wasm(root)),
                ("FF-07 platform", fitness::purity::run_platform(root)),
                ("FF-19 trailers", fitness::trailers::run(root, base)),
                ("FF-20 inventory", fitness::inventory::run(root, base)),
                ("FF-33 docs-links", fitness::docs_links::run(root)),
            ];
            let failed: Vec<String> = results
                .into_iter()
                .filter_map(|(name, r)| r.err().map(|e| format!("{name}: FAILED\n{e}")))
                .collect();
            if failed.is_empty() {
                Ok(())
            } else {
                Err(failed.join("\n\n"))
            }
        }
        _ => Err(usage()),
    }
}

fn parse_base(rest: &[String]) -> Result<Option<String>, String> {
    match rest {
        [] => Ok(None),
        [flag, value] if flag == "--base" => Ok(Some(value.clone())),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: cargo xtask fitness <all|deps|purity|wasm|platform|trailers|inventory|docs-links> [--base <rev>]".to_string()
}
