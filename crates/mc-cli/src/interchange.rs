//! Interchange commands: MusicXML export (ADR-033), MIDI export and import (ADR-023)
//! and Guitar Pro export and import (ADR-036). They exist for the CI schema check and
//! the MuseScore and TuxGuitar procedures; the app uses the same adapters through
//! `mc-ffi`.

use mc_app::types::Score;
use std::path::Path;

pub const USAGE: &str = "
Interchange (<source> is a .giproj file, fixture:<small|typical|large|feature-rich|empty>,
midi:<file.mid> to import a Standard MIDI File first, or gp:<file.gp3|.gp4|.gp5> to
import a Guitar Pro file first and print its import report):
  export-musicxml <source> <out.musicxml>  write MusicXML 4.0 and print the report
  export-midi <source> <out.mid>           write a Standard MIDI File and print the report
  export-gp <source> <out.gp5|.gp4|.gp3>   write Guitar Pro (version from the extension)
                                           and print the report
";

fn source(spec: &str) -> Result<Score, String> {
    if let Some(name) = spec.strip_prefix("fixture:") {
        return Ok(match name {
            "small" => mc_app::fixtures::reference(mc_app::fixtures::SMALL),
            "typical" => mc_app::fixtures::reference(mc_app::fixtures::TYPICAL),
            "large" => mc_app::fixtures::reference(mc_app::fixtures::LARGE),
            "feature-rich" => mc_app::fixtures::feature_rich(),
            "techniques" => mc_app::fixtures::techniques(),
            "empty" => mc_app::new_score("Track 1", 4).map_err(|e| e.to_string())?,
            other => return Err(format!("unknown fixture '{other}'")),
        });
    }
    if let Some(path) = spec.strip_prefix("midi:") {
        let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        let (score, report) = mc_midi::import_smf(&bytes).map_err(|e| e.to_string())?;
        eprintln!(
            "imported {path}: {} track(s), {} note(s), {} issue(s)",
            report.tracks_imported,
            report.notes_imported,
            report.issues.len()
        );
        return Ok(score);
    }
    if let Some(path) = spec.strip_prefix("gp:") {
        let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        let (score, r) = mc_guitarpro::import_gp(&bytes).map_err(|e| e.to_string())?;
        eprintln!(
            "imported {path} ({}): {} track(s), {} bar(s), {} of {} note(s), {} dropped",
            r.version,
            r.tracks_imported,
            r.bars,
            r.notes_imported,
            r.notes_in_file,
            r.notes_dropped
        );
        for issue in &r.issues {
            eprintln!("  {issue}");
        }
        return Ok(score);
    }
    Ok(crate::open(Path::new(spec))?.snapshot())
}

pub fn export_gp(spec: &str, out: &str) -> Result<(), String> {
    let ext = Path::new(out)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let version = mc_guitarpro::GpVersion::from_extension(ext)
        .ok_or_else(|| format!("{out}: the extension must be .gp5, .gp4 or .gp3"))?;
    let score = source(spec)?;
    let (bytes, r) = mc_guitarpro::export_gp(&score, version).map_err(|e| e.to_string())?;
    std::fs::write(out, &bytes).map_err(|e| format!("{out}: {e}"))?;
    println!(
        "wrote {out} ({:?}): {} track(s), {} bar(s), {} note(s) written, {} dropped",
        r.version, r.tracks, r.bars, r.notes_written, r.notes_dropped
    );
    for issue in &r.issues {
        println!("  {issue}");
    }
    Ok(())
}

pub fn export_musicxml(spec: &str, out: &str) -> Result<(), String> {
    let score = source(spec)?;
    let start = std::time::Instant::now();
    let (xml, r) = mc_musicxml::export_musicxml(&score).map_err(|e| e.to_string())?;
    let elapsed = start.elapsed();
    std::fs::write(out, xml.as_bytes()).map_err(|e| format!("{out}: {e}"))?;
    println!(
        "wrote {out}: {} part(s), {} measure(s), {} note(s) in {:.1} ms; padded voices {}, \
         overfull measures {}, tuplets without bracket {}, tempo rounded {}, characters removed {}, \
         techniques approximated {}",
        r.parts,
        r.measures,
        r.notes,
        elapsed.as_secs_f64() * 1e3,
        r.padded_voices,
        r.overfull_measures,
        r.tuplets_without_bracket,
        r.tempo_rounded,
        r.characters_removed,
        r.techniques_approximated
    );
    Ok(())
}

pub fn export_midi(spec: &str, out: &str) -> Result<(), String> {
    let score = source(spec)?;
    let timeline = mc_app::playback::compile_for_export(&score)?;
    let (bytes, r) = mc_midi::export_smf(&score, &timeline).map_err(|e| e.to_string())?;
    std::fs::write(out, &bytes).map_err(|e| format!("{out}: {e}"))?;
    println!(
        "wrote {out}: {} track(s), {} note(s), {} rounded tick(s), {} tempo clamp(s)",
        r.tracks, r.notes, r.rounded_ticks, r.tempo_clamped
    );
    Ok(())
}
