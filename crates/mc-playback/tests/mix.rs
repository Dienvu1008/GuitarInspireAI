//! ADR-032 value objects (X7) and the channel assignment the application uses to find
//! a track's channel without compiling.

use mc_playback::{
    CompileOptions, OutOfRange, Pan, PlaybackWarning, TrackMix, Volume, assign_channels, compile,
};
use mc_score::{FrettedInstrument, InstrumentSpec, MasterBar, Score};
use mc_theory::Tempo;
use proptest::prelude::*;

fn score_with(tracks: usize) -> Score {
    let guitar = || InstrumentSpec::Fretted(FrettedInstrument::standard_guitar());
    let mut s = Score::new(
        "T1",
        guitar(),
        MasterBar::default(),
        Tempo::from_bpm(120).unwrap(),
    )
    .unwrap();
    for i in 1..tracks {
        s.add_track(&format!("T{}", i + 1), guitar()).unwrap();
    }
    s
}

#[test]
fn x7_volume_accepts_exactly_0_to_127() {
    for v in 0..=127u8 {
        assert_eq!(Volume::try_new(v).unwrap().get(), v);
    }
    for v in 128..=255u8 {
        assert_eq!(
            Volume::try_new(v),
            Err(OutOfRange {
                kind: "Volume",
                value: v as i64
            })
        );
    }
}

#[test]
fn x7_pan_accepts_exactly_minus_63_to_63_and_maps_to_cc_1_to_127() {
    for v in -63..=63i16 {
        let p = Pan::try_new(v).unwrap();
        assert_eq!(p.get() as i16, v);
        assert_eq!(p.cc() as i16, 64 + v);
        assert!((1..=127).contains(&p.cc()), "CC stays inside 1..=127");
    }
    for v in [-64i16, 64, 100, -100, i16::MIN, i16::MAX] {
        assert_eq!(
            Pan::try_new(v),
            Err(OutOfRange {
                kind: "Pan",
                value: v as i64
            })
        );
    }
}

#[test]
fn x1_the_default_mix_is_what_playback_always_sounded_like() {
    // The synthesizer starts at volume 100 and pan centre (CC 64); the default mix must
    // equal that, so playback with no mix change sounds as before (ADR-032 X1).
    let d = TrackMix::default();
    assert_eq!(d.volume.get(), 100);
    assert_eq!(d.pan.get(), 0);
    assert_eq!(d.pan.cc(), 64);
    assert_eq!(d.volume, Volume::DEFAULT);
    assert_eq!(d.pan, Pan::CENTRE);
}

proptest! {
    /// The application finds a track's channel with `assign_channels`; it must be the
    /// assignment the compiled timeline uses, for any number of tracks (X6 depends on it).
    #[test]
    fn assign_channels_is_the_compiled_assignment(tracks in 1usize..40) {
        let s = score_with(tracks);
        let assigned = assign_channels(&s);
        let out = compile(&s, &CompileOptions::new()).unwrap();
        prop_assert_eq!(assigned.as_slice(), out.timeline.channels());
        prop_assert_eq!(assigned.len(), tracks);
        // Never channel 9, and the first 15 tracks have distinct channels.
        prop_assert!(assigned.iter().all(|c| c.channel.get() != 9));
        let mut firsts: Vec<u8> = assigned.iter().take(15).map(|c| c.channel.get()).collect();
        firsts.sort_unstable();
        firsts.dedup();
        prop_assert_eq!(firsts.len(), tracks.min(15));
        // Tracks beyond 15 reuse a channel and the compiler warns about exactly them.
        let warned = out.warnings.iter().find_map(|w| match w {
            PlaybackWarning::ChannelShared { tracks } => Some(tracks.len()),
            _ => None,
        });
        prop_assert_eq!(warned.unwrap_or(0), tracks.saturating_sub(15));
    }
}
