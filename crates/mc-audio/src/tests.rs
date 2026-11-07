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

// ---- ADR-032: volume and pan per channel (X1-X5) ----

mod mix {
    use super::*;
    use mc_playback::{Channel, Pan, Volume, assign_channels};

    /// An engine whose synthesizer has no reverb and chorus. The effects return is not
    /// panned and rings after a note ends, so a per-channel measurement (X2, X3, X4)
    /// is made on the dry signal; the default synthesizer is covered by X1 and X5.
    fn dry_engine(bars: usize) -> OfflineEngine {
        use crate::rt::RtCommand;
        use rustysynth::{Synthesizer, SynthesizerSettings};
        let mut e = OfflineEngine::new(RATE);
        let font = crate::soundfont::parse_soundfont(SF2).unwrap();
        let mut settings = SynthesizerSettings::new(RATE as i32);
        settings.enable_reverb_and_chorus = false;
        let synth = Synthesizer::new(&font, &settings).unwrap();
        e.send(RtCommand::InstallSynth(Box::new(synth)));
        e.load_timeline(timeline(bars));
        e
    }

    fn ch(n: u8) -> Channel {
        Channel::try_new(n).unwrap()
    }
    fn vol(v: u8) -> Volume {
        Volume::try_new(v).unwrap()
    }
    fn pan(v: i16) -> Pan {
        Pan::try_new(v).unwrap()
    }

    fn max_diff(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        a.iter().zip(b).fold(0.0, |m, (x, y)| m.max((x - y).abs()))
    }

    fn energy(samples: &[f32], right: bool) -> f64 {
        samples
            .chunks(2)
            .map(|f| f[usize::from(right)] as f64)
            .map(|x| x * x)
            .sum()
    }

    /// Plays the 4-bar score for `frames` frames, applying `before` to the engine first.
    fn run(frames: usize, before: impl FnOnce(&mut OfflineEngine)) -> Vec<f32> {
        let mut e = engine_with(4);
        before(&mut e);
        e.play();
        e.render(frames)
    }

    fn run_dry(frames: usize, before: impl FnOnce(&mut OfflineEngine)) -> Vec<f32> {
        let mut e = dry_engine(4);
        before(&mut e);
        e.play();
        e.render(frames)
    }

    #[test]
    fn x1_the_default_mix_changes_nothing() {
        let with_mix = run(RATE as usize, |_| {});
        let without = run(RATE as usize, |e| e.rt_mut().mix_disabled = true);
        assert_eq!(with_mix, without, "the default table must be bit-identical");
    }

    #[test]
    fn x2_volume_zero_equals_a_muted_track() {
        let s = reference(Shape {
            tracks: 2,
            bars: 4,
            beats: 8,
            notes_per_beat: 2,
        });
        let tracks: Vec<_> = assign_channels(&s);
        assert!(tracks.len() >= 2);
        let (t1, c1) = (tracks[1].track, tracks[1].channel);
        let mut options = CompileOptions::new();
        options.muted.insert(t1);
        let muted = compile(&s, &options).unwrap().timeline;

        let mut reference_engine = OfflineEngine::new(RATE);
        reference_engine.install_soundfont(SF2).unwrap();
        reference_engine.load_timeline(muted);
        reference_engine.play();
        let expected = reference_engine.render(RATE as usize);

        let silenced = run(RATE as usize, |e| {
            e.set_channel_mix(c1, vol(0), Pan::CENTRE)
        });
        assert!(
            max_diff(&expected, &silenced) < 1e-6,
            "volume 0 must sound like a muted track: {}",
            max_diff(&expected, &silenced)
        );
        assert!(peak(&silenced) > 0.01, "the other track still sounds");
    }

    #[test]
    fn x2_a_volume_drop_in_the_middle_of_a_note_is_audible() {
        let mut e = dry_engine(4);
        e.play();
        e.render(RATE as usize / 4);
        let loud = energy(&e.render(2400), false);
        e.set_channel_mix(ch(0), vol(0), Pan::CENTRE);
        e.set_channel_mix(ch(1), vol(0), Pan::CENTRE);
        e.render(480);
        let quiet = energy(&e.render(2400), false);
        assert!(quiet < loud * 0.01, "loud {loud}, quiet {quiet}");
    }

    #[test]
    fn x3_pan_moves_the_sound_between_the_sides() {
        let hard = |p: i16| {
            run_dry(RATE as usize, |e| {
                e.set_channel_mix(ch(0), vol(100), pan(p));
                e.set_channel_mix(ch(1), vol(0), Pan::CENTRE);
            })
        };
        let left = hard(-63);
        let right = hard(63);
        let centre = hard(0);
        assert!(energy(&left, true) <= energy(&left, false) * 0.01 + 1e-12);
        assert!(energy(&right, false) <= energy(&right, true) * 0.01 + 1e-12);
        let (l, r) = (energy(&centre, false), energy(&centre, true));
        assert!(
            l > 0.0 && r > 0.0 && (l / r - 1.0).abs() < 0.2,
            "centre {l} {r}"
        );
        // The centre pan equals the default table (the default pan is the centre).
        let default = run_dry(RATE as usize, |e| {
            e.set_channel_mix(ch(1), vol(0), Pan::CENTRE)
        });
        assert_eq!(centre, default);
    }

    #[test]
    fn x4_a_change_is_heard_in_the_next_callback() {
        let mut control = dry_engine(4);
        let mut changed = dry_engine(4);
        control.play();
        changed.play();
        assert_eq!(control.render(4800), changed.render(4800));
        for c in [ch(0), ch(1)] {
            changed.set_channel_mix(c, vol(0), Pan::CENTRE);
        }
        let a = control.render(480);
        let b = changed.render(480);
        assert!(max_diff(&a, &b) > 1e-4, "the very next callback differs");
        changed.render(480 * 2);
        let tail = energy(&changed.render(480), false) + energy(&changed.render(0), true);
        let reference_level =
            energy(&control.render(480 * 2), false) + energy(&control.render(480), false);
        assert!(
            tail < reference_level * 0.01 + 1e-9,
            "silent within 3 callbacks"
        );
    }

    fn silenced(e: &mut OfflineEngine) {
        for c in [0u8, 1] {
            e.set_channel_mix(ch(c), vol(0), Pan::CENTRE);
        }
    }

    fn assert_still_silent(e: &mut OfflineEngine, what: &str) {
        e.play();
        e.render(480); // the callback applies commands and the table
        let p = peak(&e.render(RATE as usize / 2));
        assert!(p < 1e-3, "after {what} the mix must still hold (peak {p})");
    }

    #[test]
    fn x5_the_mix_survives_seek_swap_reinstall_and_device_rebuild() {
        let mut e = engine_with(4);
        silenced(&mut e);
        e.seek(Tick(0));
        assert_still_silent(&mut e, "seek");

        e.load_timeline(timeline(4));
        assert_still_silent(&mut e, "a timeline swap");

        e.install_soundfont(SF2).unwrap();
        assert_still_silent(&mut e, "installing the synthesizer again");

        e.rebuild_device().unwrap();
        e.load_timeline(timeline(4));
        assert_still_silent(&mut e, "a device rebuild");

        // And the sound comes back when the mix is restored.
        for c in [0u8, 1] {
            e.set_channel_mix(ch(c), Volume::DEFAULT, Pan::CENTRE);
        }
        e.seek(Tick(0));
        e.render(480);
        assert!(peak(&e.render(RATE as usize / 2)) > 0.01);
    }
}
