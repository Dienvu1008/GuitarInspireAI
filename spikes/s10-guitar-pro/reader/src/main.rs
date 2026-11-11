mod cursor;
mod gp;
mod model;
mod write;

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
    for (bar, text) in &song.lyrics {
        if !text.is_empty() {
            println!(
                "  lyrics (track {}) from bar {bar}: {text:?}",
                song.lyrics_track
            );
        }
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
            let passes: Vec<String> = (0..8)
                .filter(|i| h.alternative & (1 << i) != 0)
                .map(|i| (i + 1).to_string())
                .collect();
            p.push(format!("volta {}", passes.join("+")));
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
            if track.percussion {
                " (percussion)"
            } else {
                ""
            },
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
                        if let Some(ch) = &b.chord {
                            s.push_str(&format!(
                                " chord {:?} fret {} {:?}",
                                ch.name, ch.first_fret, ch.frets
                            ));
                        }
                        if let Some(t) = &b.text {
                            s.push_str(&format!(" text {t:?}"));
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

/// The model without what the writer does not keep (unsupported effects).
fn kept(mut s: Song) -> Song {
    for t in s.measures.iter_mut() {
        for m in t.iter_mut() {
            for v in m.voices.iter_mut() {
                for b in v.iter_mut() {
                    b.unsupported.clear();
                    for n in b.notes.iter_mut() {
                        n.unsupported.clear();
                    }
                }
            }
        }
    }
    s
}

/// Read, write with our writer, read again: the kept model must not change.
fn roundtrip(paths: &[String], out: &str) {
    std::fs::create_dir_all(out).expect("out dir");
    let (mut same, mut differ) = (0, 0);
    for path in paths {
        let data = std::fs::read(path).expect("read");
        let first = match gp::read(&mut Cursor::new(&data)) {
            Ok(s) => kept(s),
            Err(e) => {
                println!("{path}: cannot read: {} at {}", e.what, e.at);
                continue;
            }
        };
        let bytes = write::write(&first);
        let name = std::path::Path::new(path).file_name().unwrap();
        let target = std::path::Path::new(out).join(name);
        std::fs::write(&target, &bytes).expect("write");
        match gp::read(&mut Cursor::new(&bytes)) {
            Ok(again) if kept(again.clone()) == first => {
                same += 1;
                println!(
                    "{}: same model after our write ({} bytes)",
                    target.display(),
                    bytes.len()
                );
            }
            Ok(_) => {
                differ += 1;
                println!("{}: MODEL DIFFERS", target.display());
            }
            Err(e) => {
                differ += 1;
                println!(
                    "{}: our file does not read back: {} at {}",
                    target.display(),
                    e.what,
                    e.at
                );
            }
        }
    }
    println!("round trip: {same} same, {differ} differ");
}

/// Mutates random bytes of each file `rounds` times (deterministic xorshift) and
/// counts how the reader ends: read, typed error, or panic.
fn mutate(paths: &[String], rounds: usize) {
    std::panic::set_hook(Box::new(|_| {}));
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (mut ok, mut errors, mut panics) = (0usize, 0usize, 0usize);
    for path in paths {
        let original = std::fs::read(path).expect("read");
        for _ in 0..rounds {
            let mut data = original.clone();
            let edits = 1 + (next() % 8) as usize;
            for _ in 0..edits {
                let at = (next() % data.len() as u64) as usize;
                data[at] = next() as u8;
            }
            if next() % 4 == 0 {
                let keep = (next() % data.len() as u64) as usize;
                data.truncate(keep);
            }
            let r = std::panic::catch_unwind(|| gp::read(&mut Cursor::new(&data)).is_ok());
            match r {
                Ok(true) => ok += 1,
                Ok(false) => errors += 1,
                Err(_) => panics += 1,
            }
        }
    }
    println!(
        "mutations: {} read, {} typed errors, {} panics",
        ok, errors, panics
    );
}
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--mutate") {
        mutate(&args[2..], args[1].parse().expect("rounds"));
        return;
    }
    if args.first().map(String::as_str) == Some("--roundtrip") {
        roundtrip(&args[2..], &args[1]);
        return;
    }
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
