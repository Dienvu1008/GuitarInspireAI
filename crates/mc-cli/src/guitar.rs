//! Guitar query commands: `chord`, `identify`, `scale`, `tunings`. Thin formatting
//! over `mc_app::guitar`; every musical decision is made by the core.

use mc_app::guitar::{
    self, BassRule, Identification, IdentifyOptions, IdentifyStatus, SearchOptions, TuningPreset,
    VoicingConstraints, interval_name,
};

pub const USAGE: &str = "\
  chord <symbol> [options]             list voicings, for example: chord Am7 --tuning drop-d
  identify <shape> [options]           name a shape (low string first), for example: identify x32010
  scale <root> <scale> [--frets 0-12] [--tuning T] [--capo N]
                                       scale notes per string
  tunings                              list tuning presets
  options: --tuning <preset|E4,B3,...> --capo N --limit N --span N --fingers N
           --min N --max N --no-open --no-omit --inversions";

/// Parsed `--flag [value]` options and positional arguments.
struct Args {
    positional: Vec<String>,
    values: Vec<(String, String)>,
    switches: Vec<String>,
}

const SWITCHES: [&str; 3] = ["--no-open", "--no-omit", "--inversions"];

impl Args {
    fn parse(args: &[String]) -> Result<Args, String> {
        let mut out = Args {
            positional: Vec::new(),
            values: Vec::new(),
            switches: Vec::new(),
        };
        let mut it = args.iter();
        while let Some(a) = it.next() {
            if SWITCHES.contains(&a.as_str()) {
                out.switches.push(a.clone());
            } else if a.starts_with("--") {
                let v = it.next().ok_or_else(|| format!("missing value for {a}"))?;
                out.values.push((a.clone(), v.clone()));
            } else {
                out.positional.push(a.clone());
            }
        }
        Ok(out)
    }

    fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn number<T: std::str::FromStr>(&self, name: &str, default: T) -> Result<T, String> {
        self.value(name)
            .map(|v| v.parse().map_err(|_| format!("invalid {name} '{v}'")))
            .transpose()
            .map(|v| v.unwrap_or(default))
    }

    fn switch(&self, name: &str) -> bool {
        self.switches.iter().any(|s| s == name)
    }

    fn tuning(&self) -> Result<(String, mc_app::guitar::Tuning), String> {
        let name = self.value("--tuning").unwrap_or("standard");
        let t = guitar::parse_tuning(name).map_err(|e| e.to_string())?;
        Ok((name.to_string(), t))
    }

    fn first(&self, what: &str) -> Result<&str, String> {
        self.positional
            .first()
            .map(String::as_str)
            .ok_or_else(|| format!("missing {what}"))
    }
}

fn octave_name(semitone: i32) -> String {
    format!(
        "{}{}",
        guitar::note_name(semitone),
        semitone.div_euclid(12) - 1
    )
}

pub fn chord(args: &[String]) -> Result<(), String> {
    let a = Args::parse(args)?;
    let symbol = a.first("chord symbol")?;
    let (tuning_name, tuning) = a.tuning()?;
    let capo = a.number("--capo", 0u8)?;
    let d = VoicingConstraints::default();
    let constraints = VoicingConstraints {
        max_span: a.number("--span", d.max_span)?,
        max_fingers: a.number("--fingers", d.max_fingers)?,
        allow_open_strings: !a.switch("--no-open"),
        min_fret: a.number("--min", d.min_fret)?,
        max_fret: a.number("--max", d.max_fret)?,
        allow_omitted_fifth: !a.switch("--no-omit"),
        min_sounding: d.min_sounding,
        bass: if a.switch("--inversions") {
            BassRule::AnyChordTone
        } else {
            BassRule::ChordBass
        },
    };
    let options = SearchOptions {
        constraints,
        max_results: a.number("--limit", SearchOptions::default().max_results)?,
    };
    let (chord, found) =
        guitar::chord_voicings(symbol, &tuning, capo, &options).map_err(|e| e.to_string())?;
    println!(
        "{} ({}) in {tuning_name}, capo {capo}: {} voicing(s)",
        guitar::format_chord(&chord),
        guitar::chord_tones(&chord).join(" "),
        found.len()
    );
    for v in &found {
        let frets = match (v.lowest_fretted(), v.highest_fretted()) {
            (Some(lo), Some(hi)) if lo == hi => format!("fret {lo}"),
            (Some(lo), Some(hi)) => format!("frets {lo}-{hi}"),
            _ => "open".to_string(),
        };
        println!(
            "  {:<18} {:<11} fingers {}",
            v.to_string(),
            frets,
            v.fingers()
        );
    }
    Ok(())
}

pub fn identify(args: &[String]) -> Result<(), String> {
    let a = Args::parse(args)?;
    let shape = a.first("shape")?;
    let (tuning_name, tuning) = a.tuning()?;
    let capo = a.number("--capo", 0u8)?;
    let options = IdentifyOptions {
        allow_omitted_fifth: !a.switch("--no-omit"),
    };
    let id = guitar::identify_shape(shape, &tuning, capo, options).map_err(|e| e.to_string())?;
    print_identification(shape, &tuning_name, capo, &id);
    Ok(())
}

fn print_identification(shape: &str, tuning: &str, capo: u8, id: &Identification) {
    let notes: Vec<String> = id.notes.iter().map(|n| octave_name(n.semitone)).collect();
    println!("{shape} in {tuning}, capo {capo}");
    println!(
        "notes: {}{}",
        if notes.is_empty() {
            "none".to_string()
        } else {
            notes.join(" ")
        },
        id.bass
            .map(|b| format!(" (bass {})", octave_name(b.semitone)))
            .unwrap_or_default()
    );
    match id.status {
        IdentifyStatus::Insufficient { distinct } => {
            println!(
                "insufficient pitch content: {distinct} distinct pitch class(es); a chord name needs at least {}",
                guitar::MIN_DISTINCT_PITCH_CLASSES
            );
            return;
        }
        IdentifyStatus::NoMatch => {
            println!("no chord in the Phase 2 vocabulary has exactly these notes");
            return;
        }
        IdentifyStatus::Identified => {}
    }
    for (i, c) in id.candidates.iter().enumerate() {
        let tones: Vec<String> = c
            .tones
            .iter()
            .map(|t| {
                format!(
                    "{}={}{}",
                    t.name,
                    interval_name(&t.interval),
                    if t.sounding { "" } else { " (missing)" }
                )
            })
            .collect();
        println!(
            "{}. {}{}",
            i + 1,
            c.symbol,
            if i == 0 { " (preferred)" } else { "" }
        );
        println!("   tones: {}", tones.join(", "));
        for line in &c.explanation {
            println!("   - {line}");
        }
    }
}

pub fn scale(args: &[String]) -> Result<(), String> {
    let a = Args::parse(args)?;
    let root = a.first("root")?;
    let name = a
        .positional
        .get(1)
        .ok_or_else(|| format!("missing scale ({})", guitar::scale_names().join(", ")))?;
    let (tuning_name, tuning) = a.tuning()?;
    let capo = a.number("--capo", 0u8)?;
    let window = a.value("--frets").unwrap_or("0-12");
    let (lo, hi) = window
        .split_once('-')
        .and_then(|(l, h)| Some((l.parse::<u8>().ok()?, h.parse::<u8>().ok()?)))
        .ok_or_else(|| format!("invalid --frets '{window}' (expected lo-hi)"))?;
    let notes =
        guitar::scale_positions(root, name, &tuning, capo, lo..=hi).map_err(|e| e.to_string())?;
    println!("{root} {name} in {tuning_name}, capo {capo}, frets {lo}-{hi} (fret:degree)");
    for s in 1..=tuning.string_count() {
        let cells: Vec<String> = notes
            .iter()
            .filter(|n| n.position.string.get() == s)
            .map(|n| format!("{}:{}", n.position.fret.0, n.degree))
            .collect();
        println!("  string {s}: {}", cells.join(" "));
    }
    Ok(())
}

pub fn tunings() {
    for p in TuningPreset::ALL {
        println!("{:<15} {}", p.name(), p.pitches().join(" "));
    }
}
