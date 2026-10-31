//! Spike S2: is cpal (WASAPI) + rustysynth + rtrb real-time safe and fast enough?
//! Modes:
//!   offline                      A1, A3, A6 (offline), A7, negative control
//!   device <seconds> <load>      A2, A4, A5, A6 on the default output device
//!   bufsize                      A5 buffer size behaviour on WASAPI

mod engine;
mod guard;
mod sf2;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use engine::{Cmd, now_ns};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[global_allocator]
static ALLOC: guard::CountingAlloc = guard::CountingAlloc;

fn main() {
    engine::init_clock();
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("offline") => offline(),
        Some("device") => {
            let secs = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(30);
            let load = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
            device(secs, load);
        }
        Some("bufsize") => bufsize(),
        _ => eprintln!("usage: s2-audio offline | device <seconds> <load-threads> | bufsize"),
    }
}

fn offline() {
    // Negative control: the guard must detect an allocation in a forbidden scope.
    let before = guard::violations();
    guard::forbid(|| drop(std::hint::black_box(vec![1u8; 16])));
    println!("negative_control_violations={}", guard::violations() - before);
    guard::VIOLATIONS.store(0, Relaxed);

    // A1, A3, A6 offline: full reference workload rendered in 480-frame blocks with
    // allocation forbidden, while another thread sends timeline swaps through rtrb.
    let sr = 48_000;
    let tl = engine::workload(sr, 150, 1);
    let events = tl.events.len();
    let len = tl.len;
    let (mut rt, mut cmd, mut ret) = engine::new_rt(sr, tl);
    cmd.push(Cmd::Play { requested_ns: now_ns() }).ok().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let sent = Arc::new(AtomicU64::new(0));
    let control = {
        let (stop, sent) = (stop.clone(), sent.clone());
        std::thread::spawn(move || {
            let mut seed = 2;
            let mut freed = 0u64;
            while !stop.load(Relaxed) {
                std::thread::sleep(Duration::from_millis(20));
                if sent.load(Relaxed) < 100 && cmd.slots() > 0 {
                    let t = engine::workload(sr, 150, seed);
                    seed += 1;
                    if cmd.push(Cmd::Swap(Box::new(t))).is_ok() {
                        sent.fetch_add(1, Relaxed);
                    }
                }
                while let Ok(old) = ret.pop() {
                    drop(old);
                    freed += 1;
                }
            }
            freed
        })
    };
    let mut out = vec![0.0f32; 480 * 2];
    let blocks = len / 480;
    let t0 = Instant::now();
    let mut worst = Duration::ZERO;
    let mut peak = 0.0f32;
    let mut times: Vec<(u32, u64)> = Vec::with_capacity(blocks as usize); // (micros, block index)
    for b in 0..blocks {
        let b0 = Instant::now();
        guard::forbid(|| rt.process(&mut out, 2));
        let d = b0.elapsed();
        worst = worst.max(d);
        times.push((d.as_micros() as u32, b));
        peak = out.iter().fold(peak, |p, s| p.max(s.abs()));
    }
    let mut sorted: Vec<u32> = times.iter().map(|t| t.0).collect();
    sorted.sort_unstable();
    println!(
        "block_us_p50={} p99={} p999={} over_5ms={} over_10ms={}",
        sorted[sorted.len() / 2],
        sorted[sorted.len() * 99 / 100],
        sorted[sorted.len() * 999 / 1000],
        sorted.iter().filter(|&&t| t > 5_000).count(),
        sorted.iter().filter(|&&t| t > 10_000).count()
    );
    times.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    for (us, b) in times.iter().take(5) {
        println!("slow_block index={b} t={:.2}s us={us}", *b as f64 * 0.01);
    }
    let elapsed = t0.elapsed();
    stop.store(true, Relaxed);
    let freed = control.join().unwrap();
    let audio_secs = len as f64 / sr as f64;
    println!("workload_events={events} audio_seconds={audio_secs:.0} blocks={blocks}");
    println!("render_seconds={:.2} realtime_factor={:.1}", elapsed.as_secs_f64(), audio_secs / elapsed.as_secs_f64());
    println!("worst_block_us={} block_period_us=10000", worst.as_micros());
    println!("peak_abs_sample={peak:.3}");
    println!("rt_alloc_violations={} last_violation_size={}", guard::violations(), guard::LAST_VIOLATION_SIZE.load(Relaxed));
    println!(
        "swaps_sent={} swaps_applied={} returned_and_freed_off_thread={freed} return_ring_full={} cursor_regressions={}",
        sent.load(Relaxed),
        engine::SWAPS_APPLIED.load(Relaxed),
        engine::RETURN_RING_FULL.load(Relaxed),
        engine::CURSOR_REGRESSIONS.load(Relaxed)
    );

    // A7: pitch of a rendered A4 at 44.1 and 48 kHz (effects off). Needs the pure-sine font.
    if let Some(path) = engine::external_sf2() {
        println!("external_soundfont={path} (pitch check skipped: needs the generated sine font)");
        return;
    }
    for rate in [44_100u32, 48_000] {
        let mut s = engine::synth(rate, false);
        s.note_on(0, 69, 100);
        let n = rate as usize * 2;
        let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
        s.render(&mut l, &mut r);
        let hz = estimate_hz(&l[rate as usize..], rate);
        println!("pitch_rate={rate} hz={hz:.4} cents_error={:.3}", 1200.0 * (hz / 440.0).log2());
    }
}

/// Frequency from interpolated positive zero crossings.
fn estimate_hz(x: &[f32], rate: u32) -> f64 {
    let mut first = None;
    let mut last = 0.0;
    let mut count = 0u64;
    for i in 1..x.len() {
        if x[i - 1] < 0.0 && x[i] >= 0.0 {
            let t = (i - 1) as f64 + (-x[i - 1] / (x[i] - x[i - 1])) as f64;
            if first.is_none() {
                first = Some(t);
            } else {
                count += 1;
            }
            last = t;
        }
    }
    count as f64 / ((last - first.unwrap()) / rate as f64)
}

#[derive(Clone, Copy, Default)]
struct Metric {
    start_ns: u64,
    dur_ns: u32,
    frames: u32,
    out_latency_us: u32,
    swaps: u32,
}

/// 0 = not attempted, 1 = registered, 2 = failed.
static MMCSS_STATE: AtomicU64 = AtomicU64::new(0);

/// Registers the calling thread with MMCSS "Pro Audio" once. `task` is a
/// preallocated NUL-terminated UTF-16 string, so this does not allocate.
#[cfg(feature = "mmcss")]
fn mmcss_once(task: &[u16]) {
    if MMCSS_STATE.load(Relaxed) == 0 {
        let mut index = 0u32;
        let handle = unsafe { windows_sys::Win32::System::Threading::AvSetMmThreadCharacteristicsW(task.as_ptr(), &mut index) };
        MMCSS_STATE.store(if handle.is_null() { 2 } else { 1 }, Relaxed);
    }
}

static XRUNS: AtomicU64 = AtomicU64::new(0);
static OTHER_ERRORS: AtomicU64 = AtomicU64::new(0);
static METRIC_RING_FULL: AtomicU64 = AtomicU64::new(0);

fn device(secs: u64, load: usize) {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device");
    let supported = device.default_output_config().expect("default config");
    println!("host={:?} device={}", host.id(), device.id().map(|d| d.to_string()).unwrap_or_default());
    println!("default_config={supported:?}");
    assert_eq!(supported.sample_format(), cpal::SampleFormat::F32, "spike handles f32 only");
    let config: cpal::StreamConfig = supported.into();
    let sr = config.sample_rate;
    let ch = config.channels as usize;

    let (mut rt, mut cmd, mut ret) = engine::new_rt(sr, engine::workload(sr, 150, 1));
    let (mut mtx, mut mrx) = rtrb::RingBuffer::<Metric>::new(16_384);
    #[cfg(feature = "mmcss")]
    let task_name = std::env::var("S2_MMCSS_TASK").unwrap_or_else(|_| "Audio".to_string());
    #[cfg(feature = "mmcss")]
    println!("mmcss_task={task_name}");
    #[cfg(feature = "mmcss")]
    let task: Vec<u16> = task_name.encode_utf16().chain(Some(0)).collect();

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                #[cfg(feature = "mmcss")]
                mmcss_once(&task);
                let start = now_ns();
                guard::forbid(|| {
                    let t0 = Instant::now();
                    rt.process(data, ch);
                    let ts = info.timestamp();
                    let lat = ts.playback.duration_since(ts.callback).as_micros() as u32;
                    let m = Metric {
                        start_ns: start,
                        dur_ns: t0.elapsed().as_nanos() as u32,
                        frames: (data.len() / ch) as u32,
                        out_latency_us: lat,
                        swaps: engine::SWAPS_APPLIED.load(Relaxed) as u32,
                    };
                    if mtx.push(m).is_err() {
                        METRIC_RING_FULL.fetch_add(1, Relaxed);
                    }
                });
            },
            |err: cpal::Error| {
                if matches!(err.kind(), cpal::ErrorKind::Xrun) {
                    XRUNS.fetch_add(1, Relaxed);
                } else {
                    OTHER_ERRORS.fetch_add(1, Relaxed);
                    eprintln!("stream error: {err}");
                }
            },
            None,
        )
        .expect("build stream");
    stream.play().expect("play");

    // Simulated load: busy threads plus an allocation-churn thread.
    let stop = Arc::new(AtomicBool::new(false));
    let mut workers = Vec::new();
    for i in 0..=load {
        let stop = stop.clone();
        workers.push(std::thread::spawn(move || {
            let mut x = 1.0f64;
            while !stop.load(Relaxed) {
                if i == load {
                    let v: Vec<u8> = vec![0; 1 << 20];
                    std::hint::black_box(&v);
                    std::thread::sleep(Duration::from_millis(5));
                } else {
                    for _ in 0..10_000 {
                        x = (x * 1.000001).sin() + 1.0;
                    }
                    std::hint::black_box(x);
                }
            }
        }));
    }

    std::thread::sleep(Duration::from_millis(300));
    cmd.push(Cmd::Play { requested_ns: now_ns() }).ok().unwrap();

    let mut metrics: Vec<Metric> = Vec::with_capacity(200_000);
    let begin = Instant::now();
    let mut last_swap = Instant::now();
    let (mut swaps_sent, mut freed, mut seed) = (0u64, 0u64, 2u64);
    while begin.elapsed() < Duration::from_secs(secs) {
        std::thread::sleep(Duration::from_millis(50));
        while let Ok(m) = mrx.pop() {
            metrics.push(m);
        }
        while let Ok(old) = ret.pop() {
            drop(old);
            freed += 1;
        }
        if last_swap.elapsed() >= Duration::from_secs(6) {
            if cmd.push(Cmd::Swap(Box::new(engine::workload(sr, 150, seed)))).is_ok() {
                swaps_sent += 1;
            }
            seed += 1;
            last_swap = Instant::now();
        }
    }
    stop.store(true, Relaxed);
    for w in workers {
        w.join().unwrap();
    }
    drop(stream);
    while let Ok(m) = mrx.pop() {
        metrics.push(m);
    }
    while let Ok(old) = ret.pop() {
        drop(old);
        freed += 1;
    }
    report(&metrics, sr, secs, load, swaps_sent, freed);
}

fn report(m: &[Metric], sr: u32, secs: u64, load: usize, swaps_sent: u64, freed: u64) {
    let skip = 50.min(m.len()); // ignore stream start-up callbacks
    let m = &m[skip..];
    let mut frames: Vec<u32> = m.iter().map(|x| x.frames).collect();
    frames.sort_unstable();
    let typical_frames = frames[frames.len() / 2];
    let period_us = typical_frames as f64 * 1e6 / sr as f64;
    let mut durs: Vec<u32> = m.iter().map(|x| x.dur_ns / 1000).collect();
    durs.sort_unstable();
    let pct = |v: &[u32], p: f64| v[((v.len() - 1) as f64 * p) as usize];
    let mut gaps_15 = 0;
    let mut gaps_20 = 0;
    let mut max_interval = 0.0f64;
    for w in m.windows(2) {
        let iv = (w[1].start_ns - w[0].start_ns) as f64 / 1000.0;
        max_interval = max_interval.max(iv);
        let expected = w[0].frames as f64 * 1e6 / sr as f64;
        if iv > 1.5 * expected {
            gaps_15 += 1;
        }
        if iv > 2.0 * expected {
            gaps_20 += 1;
        }
    }
    // Anomalies: callbacks over 25% of the period, intervals over 1.5x the period.
    let t_play = engine::PLAY_REQUESTED_NS.load(Relaxed);
    let at = |ns: u64| (ns.saturating_sub(t_play)) as f64 / 1e9;
    for (i, x) in m.iter().enumerate() {
        let swapped = i > 0 && m[i - 1].swaps != x.swaps;
        if x.dur_ns as f64 / 1000.0 > 0.25 * period_us {
            println!("anomaly long_callback t={:.2}s dur_us={} frames={} swap_in_callback={swapped}", at(x.start_ns), x.dur_ns / 1000, x.frames);
        }
        if i > 0 {
            let iv = (x.start_ns - m[i - 1].start_ns) as f64 / 1000.0;
            if iv > 1.5 * period_us {
                println!("anomaly late_callback t={:.2}s interval_us={iv:.0} next_frames={} swap_in_callback={swapped}", at(x.start_ns), x.frames);
            }
        }
    }
    let mut lats: Vec<u32> = m.iter().map(|x| x.out_latency_us).collect();
    lats.sort_unstable();
    let req = engine::PLAY_REQUESTED_NS.load(Relaxed);
    let first = engine::FIRST_AUDIO_NS.load(Relaxed);
    println!(
        "run_seconds={secs} load_threads={load} sample_rate={sr} priority={} mmcss_state={}",
        if cfg!(feature = "realtime") { "cpal-realtime" } else if cfg!(feature = "mmcss") { "own-mmcss" } else { "default" },
        MMCSS_STATE.load(Relaxed)
    );
    println!("callbacks={} frames_min={} frames_typical={typical_frames} frames_max={} period_us={period_us:.0}", m.len(), frames[0], frames[frames.len() - 1]);
    println!("callback_us_p50={} p99={} max={} p99_over_period={:.3}", pct(&durs, 0.5), pct(&durs, 0.99), durs[durs.len() - 1], pct(&durs, 0.99) as f64 / period_us);
    println!("intervals_over_1.5x_period={gaps_15} intervals_over_2x_period={gaps_20} max_interval_us={max_interval:.0}");
    println!("cpal_xrun_errors={} other_stream_errors={} metric_ring_full={}", XRUNS.load(Relaxed), OTHER_ERRORS.load(Relaxed), METRIC_RING_FULL.load(Relaxed));
    println!("output_latency_us_p50={} max={}", pct(&lats, 0.5), lats[lats.len() - 1]);
    println!("play_command_to_callback_us={} estimated_play_to_sound_ms={:.1}", (first - req) / 1000, ((first - req) / 1000 + pct(&lats, 0.5) as u64) as f64 / 1000.0);
    println!("rt_alloc_violations={} last_violation_size={}", guard::violations(), guard::LAST_VIOLATION_SIZE.load(Relaxed));
    println!(
        "swaps_sent={swaps_sent} swaps_applied={} returned_and_freed_off_thread={freed} return_ring_full={} cursor_regressions={} loops={}",
        engine::SWAPS_APPLIED.load(Relaxed),
        engine::RETURN_RING_FULL.load(Relaxed),
        engine::CURSOR_REGRESSIONS.load(Relaxed),
        engine::LOOPS.load(Relaxed)
    );
}

fn bufsize() {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device");
    let supported = device.default_output_config().expect("default config");
    println!("supported_buffer_size={:?}", supported.buffer_size());
    for req in [cpal::BufferSize::Default, cpal::BufferSize::Fixed(256), cpal::BufferSize::Fixed(128)] {
        let mut config: cpal::StreamConfig = supported.clone().into();
        config.buffer_size = req;
        let ch = config.channels as usize;
        let seen = Arc::new((AtomicU64::new(u64::MAX), AtomicU64::new(0)));
        let s2 = seen.clone();
        let res = device.build_output_stream(
            config,
            move |d: &mut [f32], _: &cpal::OutputCallbackInfo| {
                d.fill(0.0);
                let f = (d.len() / ch) as u64;
                s2.0.fetch_min(f, Relaxed);
                s2.1.fetch_max(f, Relaxed);
            },
            |e| eprintln!("err {e}"),
            None,
        );
        match res {
            Ok(stream) => {
                stream.play().unwrap();
                std::thread::sleep(Duration::from_secs(2));
                drop(stream);
                println!("requested={req:?} ok frames_min={} frames_max={}", seen.0.load(Relaxed), seen.1.load(Relaxed));
            }
            Err(e) => println!("requested={req:?} error={e}"),
        }
    }
}
