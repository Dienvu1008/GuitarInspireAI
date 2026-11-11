mod cursor;
mod gp;
mod model;

use cursor::Cursor;
use model::*;

fn dur(b: &Beat) -> String {
    let base = match b.duration {
        -2 => "1",
        -1 => "2",
        0 => "4",
        1 => "8",
        2 => "16",
        3 => "32",
        4 => "64",
        _ => "?",
    };
    format!(
        "{base}{}{}",
        if b.dotted { "." } else { "" },
        b.tuplet.map(|t| format!("t{t}")).unwrap_or_default()
    )
}

fn note_text(n: &Note) -> String {
    let mut s = format!("{}/{}", n.string, n.fret);
    for (on, tag) in [
        (n.tie, "tie"),
        (n.dead, "dead"),
        (n.accent, "acc"),
        (n.heavy_accent, "heavy"),
        (n.ghost, "ghost"),
        (n.palm_mute, "pm"),
        (n.let_ring, "ring"),
        (n.staccato, "stacc"),
    ] {
        if on {
            s.push_str(&format!(" {tag}"));
        }
    }
    for u in &n.unsupported {
        s.push_str(&format!(" [{u}]"));
    }
    s
}

fn summary(song: &Song) {
    println!("  {} tempo {} key {}", song.version, song.tempo, song.key);
    for (k, v) in &song.info {
        println!("  {k}: {v:?}");
    }
    for (i, h) in song.headers.iter().enumerate() {
        let mut p = Vec::new();
        if h.numerator > 0 {
            p.push(format!("{}/{}", h.numerator, h.denominator));
        }
        if h.repeat_start {
            p.push("|:".into());
        }
        if h.repeat_end > 0 {
            p.push(format!(":| x{}", h.repeat_end));
        }
        if h.alternative > 0 {
            p.push(format!("volta {}", h.alternative));
        }
        if let Some(m) = &h.marker {
            p.push(format!("marker {m:?}"));
        }
        if let Some(k) = h.key {
            p.push(format!("key {k:?}"));
        }
        if !p.is_empty() {
            println!("  bar {}: {}", i + 1, p.join(", "));
        }
    }
    for (t, track) in song.tracks.iter().enumerate() {
        println!(
            "  track {} {:?}{} tuning {:?} capo {} frets {}",
            t + 1,
            track.name,
            if track.percussion { " (percussion)" } else { "" },
            track.tuning,
            track.capo,
            track.frets
        );
        for (m, measure) in song.measures[t].iter().enumerate() {
            for (v, voice) in measure.voices.iter().enumerate() {
                let beats: Vec<String> = voice
                    .iter()
                    .filter(|b| b.status != 0)
                    .map(|b| {
                        let mut s = dur(b);
                        if b.status == 2 {
                            s.push_str(" rest");
                        }
                        if b.stroke != (0, 0) {
                            s.push_str(&format!(" brush{:?}", b.stroke));
                        }
                        if b.pick_stroke != 0 {
                            s.push_str(&format!(" pick{}", b.pick_stroke));
                        }
                        if let Some(t) = b.tempo_change {
                            s.push_str(&format!(" tempo={t}"));
                        }
                        for u in &b.unsupported {
                            s.push_str(&format!(" [{u}]"));
                        }
                        if !b.notes.is_empty() {
                            let n: Vec<String> = b.notes.iter().map(note_text).collect();
                            s.push_str(&format!(" ({})", n.join(", ")));
                        }
                        s
                    })
                    .collect();
                if !beats.is_empty() {
                    println!("    bar {} voice {}: {}", m + 1, v + 1, beats.join(" | "));
                }
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let trace = args.iter().any(|a| a == "--trace");
    let mut ok = 0;
    let mut failed = 0;
    for path in args.iter().filter(|a| !a.starts_with("--")) {
        let data = std::fs::read(path).expect("read");
        let mut c = Cursor::new(&data);
        c.trace = trace;
        println!("{path} ({} bytes)", data.len());
        match gp::read(&mut c) {
            Ok(song) => {
                ok += 1;
                summary(&song);
            }
            Err(e) => {
                failed += 1;
                println!("  ERROR at {}: {}", e.at, e.what);
            }
        }
    }
    println!("read {ok}, failed {failed}");
}
