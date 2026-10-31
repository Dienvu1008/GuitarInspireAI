//! `mc-cli`: command-line harness for the Music Core (Phase 1, no UI).
//!
//! Project files use the product extension `.giproj` (Q-08). The extension is an
//! adapter-level convention only: new output paths without an extension get it
//! appended, but any path is accepted for reading and writing, because a file is
//! identified by its manifest (`format`, `format_version`), not by its name.

mod guitar;
mod script;

use mc_app::infra::{FsProjectStore, SystemClock, SystemEntropy, cleanup_temp_files};
use mc_app::ports::{Clock, ProjectStore};
use mc_app::types::{AppInfo, DiagnosticKind};
use mc_app::{DocumentId, DocumentSession, SessionConfig};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
usage: mc-cli <command> [args]

Project files use the extension .giproj; it is appended to <out> when <out> has none.

  new <out> [--bars N]                 create an empty project (one guitar track)
  fixture <small|typical|large|feature-rich> <out>
                                       write a deterministic reference score
  run <script> [--in <file>] --out <file>
                                       apply an edit script (see crates/mc-cli/src/script.rs)
  info <file>                          summary, diagnostics, unknown content
  validate <file>                      load with full validation; exit 1 on error
  dump <file>                          print score.json (pretty)
  roundtrip <file>                     load, save to memory, load again, compare
  format-fixtures <mc-project dir>     create missing FF-11 fixtures and FF-32 schema snapshots
                                       for the current format version (never overwrites)

Guitar queries (no project file needed):
";

/// Product-level project file extension (Q-08). Not used by the Music Core.
const PROJECT_EXTENSION: &str = "giproj";

/// Appends `.giproj` to an output path that has no extension; keeps any other path as given.
fn output_path(raw: &str) -> PathBuf {
    let p = PathBuf::from(raw);
    if p.extension().is_none() {
        p.with_extension(PROJECT_EXTENSION)
    } else {
        p
    }
}

fn app() -> AppInfo {
    AppInfo {
        app: "mc-cli".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// Positional arguments: everything that is not a `--flag` or the value of one.
fn positional(args: &[String], i: usize, what: &str) -> Result<String, String> {
    let mut out = Vec::new();
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
        } else if a.starts_with("--") {
            skip = true;
        } else {
            out.push(a.clone());
        }
    }
    out.get(i)
        .cloned()
        .ok_or_else(|| format!("missing {what}\n\n{USAGE}"))
}

fn open(path: &Path) -> Result<DocumentSession, String> {
    let (s, report) = DocumentSession::open(
        DocumentId(1),
        &FsProjectStore::default(),
        path,
        SessionConfig::default(),
        &SystemClock,
    )
    .map_err(|e| format!("{}: {e}", path.display()))?;
    if !report.unknown_content.is_empty() {
        eprintln!(
            "warning: {} item(s) of unknown content (written by a newer version); saving over this file is disabled",
            report.unknown_content.len()
        );
    }
    Ok(s)
}

fn save_new(session: &mut DocumentSession, out: &Path) -> Result<(), String> {
    cleanup_temp_files(out);
    session
        .save_as(&FsProjectStore::default(), out, &app())
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn usage() -> String {
    format!("{USAGE}{}", guitar::USAGE)
}

fn run(args: &[String]) -> Result<(), String> {
    let cmd = args.first().ok_or_else(usage)?.as_str();
    let rest = &args[1..];
    match cmd {
        "chord" => guitar::chord(rest)?,
        "identify" => guitar::identify(rest)?,
        "scale" => guitar::scale(rest)?,
        "tunings" => guitar::tunings(),
        "new" => {
            let out = output_path(&positional(rest, 0, "output path")?);
            let bars = flag(rest, "--bars")
                .map(|b| b.parse().map_err(|_| "invalid --bars"))
                .transpose()?
                .unwrap_or(4);
            let score = mc_app::new_score("Track 1", bars).map_err(|e| e.to_string())?;
            let mut s = DocumentSession::new(
                DocumentId(1),
                score,
                SessionConfig::default(),
                &SystemClock,
                &SystemEntropy,
            );
            save_new(&mut s, &out)?;
            println!("created {} ({bars} bars)", out.display());
        }
        "format-fixtures" => {
            format_fixtures(Path::new(&positional(rest, 0, "mc-project directory")?))?
        }
        "fixture" => {
            let score = match positional(rest, 0, "fixture name")?.as_str() {
                "small" => mc_app::fixtures::reference(mc_app::fixtures::SMALL),
                "typical" => mc_app::fixtures::reference(mc_app::fixtures::TYPICAL),
                "large" => mc_app::fixtures::reference(mc_app::fixtures::LARGE),
                "feature-rich" => mc_app::fixtures::feature_rich(),
                other => {
                    return Err(format!(
                        "unknown fixture '{other}' (small, typical, large, feature-rich)"
                    ));
                }
            };
            let out = output_path(&positional(rest, 1, "output path")?);
            let start = SystemClock.now_unix_millis();
            let mut s = DocumentSession::new(
                DocumentId(1),
                score,
                SessionConfig::default(),
                &SystemClock,
                &SystemEntropy,
            );
            save_new(&mut s, &out)?;
            println!(
                "wrote {} ({} notes) in {} ms",
                out.display(),
                s.score().note_count(),
                SystemClock.now_unix_millis() - start
            );
        }
        "run" => {
            let script_path = positional(rest, 0, "script path")?;
            let text =
                std::fs::read_to_string(&script_path).map_err(|e| format!("{script_path}: {e}"))?;
            let out = output_path(&flag(rest, "--out").ok_or("missing --out <file>")?);
            let mut s = match flag(rest, "--in") {
                Some(input) => open(Path::new(&input))?,
                None => DocumentSession::new(
                    DocumentId(1),
                    mc_app::new_score("Track 1", 4).map_err(|e| e.to_string())?,
                    SessionConfig::default(),
                    &SystemClock,
                    &SystemEntropy,
                ),
            };
            let applied = script::run(&mut s, &text)?;
            save_new(&mut s, &out)?;
            println!(
                "applied {applied} step(s); revision {}; wrote {}",
                s.revision().0,
                out.display()
            );
            print_diagnostics(&s);
        }
        "info" => {
            let path = PathBuf::from(positional(rest, 0, "file")?);
            let s = open(&path)?;
            let sc = s.score();
            println!("project id: {}", s.project_id());
            println!("master bars: {}", sc.master_bar_count());
            for (i, t) in sc.tracks().enumerate() {
                let strings: Vec<String> = t
                    .fretted()
                    .tuning
                    .strings()
                    .iter()
                    .map(|p| p.to_string())
                    .collect();
                println!(
                    "track {i}: {} [{}] frets {} capo {}",
                    t.name(),
                    strings.join(" "),
                    t.fretted().fret_count,
                    t.fretted().capo.0
                );
            }
            println!("notes: {}", sc.note_count());
            let order = sc.playback_order();
            println!(
                "played bars: {}{}",
                order.bars.len(),
                if order.truncated { " (truncated)" } else { "" }
            );
            if let Some((_, last)) = sc.bar_start_times().map_err(|e| e.to_string())?.last() {
                println!("last bar starts at {last} whole notes");
            }
            print_diagnostics(&s);
            for p in s.unknown_content() {
                println!("unknown content: {p}");
            }
        }
        "validate" => {
            let path = PathBuf::from(positional(rest, 0, "file")?);
            let s = open(&path)?;
            let v = s.score().validate();
            if !v.is_empty() {
                return Err(format!(
                    "{} invariant violation(s); first: {}",
                    v.len(),
                    v[0]
                ));
            }
            println!(
                "ok: {} passes validation ({} notes)",
                path.display(),
                s.score().note_count()
            );
        }
        "dump" => {
            let path = PathBuf::from(positional(rest, 0, "file")?);
            let bytes = FsProjectStore::default()
                .read(&path)
                .map_err(|e| e.to_string())?;
            let entries = mc_project::read_entries(&bytes, &mc_project::Limits::default())
                .map_err(|e| e.to_string())?;
            let score = entries
                .iter()
                .find(|(n, _)| n == mc_project::SCORE)
                .ok_or("no score.json")?;
            println!("{}", String::from_utf8_lossy(&score.1));
        }
        "roundtrip" => {
            let path = PathBuf::from(positional(rest, 0, "file")?);
            let s = open(&path)?;
            let again = DocumentSession::from_bytes(
                DocumentId(2),
                &s.snapshot_bytes(&app()),
                None,
                SessionConfig::default(),
                &SystemClock,
            )
            .map_err(|e| e.to_string())?
            .0;
            if again.score() != s.score() {
                return Err("round trip changed the score".into());
            }
            if mc_project::score_json(again.score()) != mc_project::score_json(s.score()) {
                return Err("round trip changed score.json bytes".into());
            }
            println!("ok: load, save and load give an identical score and identical score.json");
        }
        _ => return Err(usage()),
    }
    Ok(())
}

/// Writes `path` only if it does not exist (released fixtures are append-only).
fn write_new(path: &Path, bytes: &[u8]) -> Result<bool, String> {
    if path.exists() {
        return Ok(false);
    }
    std::fs::create_dir_all(path.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(true)
}

/// FF-11 fixtures and FF-32 schema snapshots for the current format version. Uses a
/// fixed project id and app info so fixtures are deterministic.
fn format_fixtures(crate_dir: &Path) -> Result<(), String> {
    use mc_project::{FORMAT_VERSION, ProjectId, SaveInfo};
    let version = format!("{}.{}", FORMAT_VERSION.major, FORMAT_VERSION.minor);
    let fixed = AppInfo {
        app: "GuitarInspireAI".into(),
        version: "0.1.0".into(),
    };
    let info = SaveInfo {
        project_id: ProjectId::from_parts(
            1_767_225_600_000,
            0x1234_5678_9abc_def0_1234_5678_9abc_def0,
        ),
        created_by: fixed.clone(),
        saved_by: fixed,
        saved_revision: 7,
    };
    let dir = crate_dir.join("tests/fixtures").join(format!("v{version}"));
    let mut created = Vec::new();
    for (name, score) in [
        (
            "small",
            mc_app::fixtures::reference(mc_app::fixtures::SMALL),
        ),
        ("feature-rich", mc_app::fixtures::feature_rich()),
    ] {
        let package = dir.join(format!("{name}.package"));
        if write_new(&package, &mc_project::save(&score, &info))? {
            created.push(package);
        }
        let golden = dir.join(format!("{name}.score.json"));
        if write_new(&golden, &mc_project::score_json(&score))? {
            created.push(golden);
        }
    }
    for (name, schema) in mc_project::schemas() {
        let path = crate_dir
            .join("schema")
            .join(format!("{name}-{version}.schema.json"));
        if write_new(&path, format!("{schema}\n").as_bytes())? {
            created.push(path);
        }
    }
    if created.is_empty() {
        println!("all fixtures and schemas for format {version} already exist; nothing written");
    }
    for p in created {
        println!(
            "created {} (add it to FIXTURES or SCHEMAS in tests/roundtrip.rs)",
            p.display()
        );
    }
    Ok(())
}

fn print_diagnostics(s: &DocumentSession) {
    for d in s.diagnostics() {
        let what = match d.kind {
            DiagnosticKind::Underfull { actual, nominal } => {
                format!("underfull ({actual} of {nominal})")
            }
            DiagnosticKind::Overfull { actual, nominal } => {
                format!("overfull ({actual} of {nominal})")
            }
        };
        let index = s
            .score()
            .tracks()
            .position(|t| t.id() == d.track)
            .unwrap_or(usize::MAX);
        println!(
            "diagnostic {}: track {index}, bar {}, voice {}: {what}",
            d.rule, d.bar, d.voice
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_path_appends_the_extension_only_when_missing() {
        assert_eq!(output_path("song"), PathBuf::from("song.giproj"));
        assert_eq!(output_path("dir/song"), PathBuf::from("dir/song.giproj"));
        assert_eq!(output_path("song.giproj"), PathBuf::from("song.giproj"));
        assert_eq!(
            output_path("song.zip"),
            PathBuf::from("song.zip"),
            "other extensions are kept, not refused"
        );
    }
}
