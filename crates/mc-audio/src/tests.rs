//! Callback-core tests, run offline (no device): sound, position, tempo factor, seek,
//! end of timeline, timeline swaps, buffer-size independence and the panic path.

use crate::OfflineEngine;
use mc_app::fixtures::{Shape, reference};
use mc_playback::{CompileOptions, PerformanceTimeline, TempoFactor, Tick, compile};

const SF2: &[u8] = include_bytes!("../../../assets/soundfonts/GuitarSteel.sf2");
const RATE: u32 = 48_000;

fn timeline(bars: usize) -> PerformanceTimeline {
    let score = reference(Shape {
        tracks: 2,
        bars,
        beats: 8,
        notes_per_beat: 2,
    });
    compile(&score, &CompileOptions::new()).unwrap().timeline
}

fn engine_with(bars: usize) -> OfflineEngine {
    let mut e = OfflineEngine::new(RATE);
    e.install_soundfont(SF2).unwrap();
    e.load_timeline(timeline(bars));
    e
}

fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0, |a, &x| a.max(x.abs()))
}

#[test]
fn without_a_synth_the_output_is_silent() {
    let mut e = OfflineEngine::new(RATE);
    e.load_timeline(timeline(2));
    e.play();
    assert_eq!(peak(&e.render(4800)), 0.0);
}

#[test]
fn playing_produces_sound_and_advances_the_position() {
    let mut e = engine_with(4);
    e.play();
    let out = e.render(RATE as usize); // one second
    assert!(peak(&out) > 0.01, "audible output");
    let s = e.status();
    assert!(s.playing && !s.finished);
    // 120 BPM: one second is half a whole note.
    let tpw = timeline(4).ticks_per_whole().get() as i64;
    assert!(
        (s.tick - tpw / 2).abs() <= tpw / 100,
        "tick {} not near {}",
        s.tick,
        tpw / 2
    );
}

#[test]
fn the_tempo_factor_scales_the_speed() {
    let tick_after_one_second = |hundredths: u16| {
        let mut e = engine_with(8);
        e.set_tempo_factor(TempoFactor::new(hundredths).unwrap());
        e.play();
        e.render(RATE as usize);
        e.status().tick
    };
    let (slow, normal, fast) = (
        tick_after_one_second(50),
        tick_after_one_second(100),
        tick_after_one_second(200),
    );
    let close = |a: i64, b: i64| (a - b).abs() <= (b / 50).max(2);
    assert!(close(normal, slow * 2), "{slow} {normal}");
    assert!(close(fast, normal * 2), "{normal} {fast}");
}

#[test]
fn changing_the_factor_while_playing_keeps_the_position() {
    let mut e = engine_with(8);
    e.play();
    e.render(RATE as usize / 2);
    let before = e.status().tick;
    e.set_tempo_factor(TempoFactor::new(200).unwrap());
    e.render(1);
    let after = e.status().tick;
    assert!(
        (after - before).abs() <= 2,
        "re-anchoring must not jump: {before} -> {after}"
    );
}

#[test]
fn seek_moves_the_position_and_stop_releases_the_notes() {
    let mut e = engine_with(8);
    let tl = timeline(8);
    let bar4 = tl.first_played_start_of_master_bar(3).unwrap();
    e.seek(bar4);
    e.render(1);
    assert_eq!(e.status().tick, bar4.0);
    e.play();
    assert!(peak(&e.render(RATE as usize / 2)) > 0.01);
    e.stop();
    // Commands are read by the callback, so the stop takes effect on the next block.
    e.render(1);
    assert!(!e.status().playing);
    // The synth's reverb keeps a tail after the notes are released, so the output does
    // not drop to zero at once. It must decay and no new notes may start: the level
    // after a few seconds is far below the level right after the stop.
    let just_after = peak(&e.render(2400));
    e.render(RATE as usize * 6);
    let later = peak(&e.render(4800));
    assert!(
        later < just_after * 0.05 && later < 1e-3,
        "just after {just_after}, later {later}"
    );
}

#[test]
fn the_timeline_ends_and_playback_stops() {
    let mut e = engine_with(1); // one bar: 2 seconds at 120 BPM
    e.play();
    e.render(RATE as usize * 3);
    let s = e.status();
    assert!(s.finished && !s.playing);
    assert_eq!(s.tick, timeline(1).end().0);
}

#[test]
fn swapping_the_timeline_keeps_playing_from_the_same_tick() {
    let mut e = engine_with(8);
    e.play();
    let mut last = 0;
    for i in 0..100 {
        e.render(480);
        if i % 2 == 0 {
            e.load_timeline(timeline(8));
        }
        let tick = e.status().tick;
        assert!(
            tick >= last,
            "cursor must not go backwards: {last} -> {tick}"
        );
        last = tick;
        e.drain_retired();
    }
    let s = e.status();
    assert_eq!(s.swaps, 51, "the initial load plus 50 swaps");
    assert_eq!(s.return_ring_full, 0);
    assert!(s.playing);
}

#[test]
fn any_buffer_size_works_and_gives_the_same_audio() {
    let render_with = |chunk: usize| {
        let mut e = engine_with(2);
        e.play();
        let total = RATE as usize; // one second
        let mut out = Vec::with_capacity(total * 2);
        let mut done = 0;
        while done < total {
            let n = chunk.min(total - done);
            out.extend(e.render(n));
            done += n;
        }
        out
    };
    let reference = render_with(480);
    for chunk in [1, 7, 64, 479, 1024, 9000] {
        let other = render_with(chunk);
        assert_eq!(reference.len(), other.len());
        let worst = reference
            .iter()
            .zip(&other)
            .fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
        assert!(worst < 1e-4, "chunk {chunk}: max difference {worst}");
    }
}

#[test]
fn rendering_is_deterministic() {
    let run = || {
        let mut e = engine_with(2);
        e.play();
        e.render(RATE as usize)
    };
    assert_eq!(run(), run());
}

#[test]
fn a_panic_in_the_callback_outputs_silence_and_raises_the_flag() {
    let mut e = engine_with(2);
    e.play();
    assert!(peak(&e.render(2400)) > 0.0);
    e.rt_mut().inject_panic = true;
    let mut out = vec![1.0f32; 960];
    e.process(&mut out, 2);
    assert!(out.iter().all(|&x| x == 0.0), "silence on the panic");
    assert!(e.status().panicked);
    assert!(!e.status().playing);
    // Every later callback stays silent; the engine does not panic again.
    e.rt_mut().inject_panic = false;
    let mut again = vec![1.0f32; 960];
    e.process(&mut again, 2);
    assert!(again.iter().all(|&x| x == 0.0));
}

#[test]
fn mono_and_multichannel_outputs_are_filled() {
    let mut e = engine_with(2);
    e.play();
    let mut mono = vec![0.0f32; 4800];
    e.process(&mut mono, 1);
    let mut six = vec![0.0f32; 4800 * 6];
    e.process(&mut six, 6);
    assert!(peak(&six) > 0.0);
    for frame in six.chunks(6) {
        assert!(
            frame[2..].iter().all(|&x| x == 0.0),
            "extra channels silent"
        );
    }
}

#[test]
fn tick_type_is_exposed_for_the_position_stream() {
    let e = engine_with(1);
    assert_eq!(e.status().tick, 0);
    let _ = Tick(0);
}
