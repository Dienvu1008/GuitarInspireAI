//! Voicing search: all voicings of a chord that satisfy the constraints (G-02).
//!
//! A result sounds only chord tones (plus the slash bass), sounds every chord tone
//! except an omissible fifth when allowed, has the required bass as its lowest
//! pitch, and passes `VoicingConstraints::check`. Order: lowest fretted fret, then
//! more sounding strings, then fewer muted strings inside the sounding range, then
//! the per-string frets (string 1 first) for determinism.

use crate::{BassRule, Voicing, VoicingConstraints};
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning, pitch_at};
use mc_theory::{Chord, PitchClass};
use std::cmp::Reverse;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SearchOptions {
    pub constraints: VoicingConstraints,
    /// Results after sorting are cut to this many.
    pub max_results: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        SearchOptions {
            constraints: VoicingConstraints::default(),
            max_results: 50,
        }
    }
}

fn bit(pc: PitchClass) -> u16 {
    1 << pc.value()
}

struct Ctx<'a> {
    tuning: &'a Tuning,
    capo: Capo,
    c: VoicingConstraints,
    options: Vec<Vec<Option<Fret>>>,
    required: u16,
    bass: Option<PitchClass>,
    out: Vec<Voicing>,
}

/// Finds voicings of `chord`, sorted and limited to `options.max_results`.
pub fn find_voicings(
    chord: &Chord,
    tuning: &Tuning,
    capo: Capo,
    options: &SearchOptions,
) -> Vec<Voicing> {
    let c = options.constraints;
    let root_pc = chord.root.pitch_class();
    let mut allowed = 0u16;
    let mut required = 0u16;
    for i in chord.quality.intervals() {
        let pc = PitchClass::from_semitone((root_pc.value() + i.semitones) as i32);
        allowed |= bit(pc);
        if !(i.omissible_fifth && c.allow_omitted_fifth) {
            required |= bit(pc);
        }
    }
    let bass = match c.bass {
        BassRule::ChordBass => Some(chord.bass.map_or(root_pc, |b| b.pitch_class())),
        BassRule::AnyChordTone => chord.bass.map(|b| b.pitch_class()),
    };
    if let Some(b) = bass {
        allowed |= bit(b);
    }
    let lo = c.min_fret.max(1);
    let per_string = (1..=tuning.string_count())
        .map(|s| {
            let string = StringIndex::new(s).expect("1-based");
            let sounds = |f: u8| {
                let pos = FretPosition {
                    string,
                    fret: Fret(f),
                };
                let pc = PitchClass::from_semitone(pitch_at(tuning, capo, pos).expect("exists"));
                allowed & bit(pc) != 0
            };
            let mut opts = vec![None];
            if c.allow_open_strings && sounds(0) {
                opts.push(Some(Fret(0)));
            }
            opts.extend(
                (lo..=c.max_fret)
                    .filter(|f| sounds(*f))
                    .map(|f| Some(Fret(f))),
            );
            opts
        })
        .collect();
    let mut ctx = Ctx {
        tuning,
        capo,
        c,
        options: per_string,
        required,
        bass,
        out: Vec::new(),
    };
    let mut frets = Vec::with_capacity(tuning.string_count() as usize);
    dfs(&mut ctx, &mut frets, None);
    let mut out = ctx.out;
    out.sort_by_cached_key(|v| {
        (
            v.lowest_fretted().unwrap_or(0),
            Reverse(v.sounding_count()),
            inner_mutes(v),
            v.frets().to_vec(),
        )
    });
    out.truncate(options.max_results);
    out
}

fn dfs(ctx: &mut Ctx<'_>, frets: &mut Vec<Option<Fret>>, range: Option<(u8, u8)>) {
    let i = frets.len();
    if i == ctx.options.len() {
        leaf(ctx, frets);
        return;
    }
    for k in 0..ctx.options[i].len() {
        let f = ctx.options[i][k];
        let next = match f {
            Some(Fret(n)) if n > 0 => {
                let (lo, hi) = range.map_or((n, n), |(lo, hi)| (lo.min(n), hi.max(n)));
                if hi - lo + 1 > ctx.c.max_span {
                    continue;
                }
                Some((lo, hi))
            }
            _ => range,
        };
        frets.push(f);
        dfs(ctx, frets, next);
        frets.pop();
    }
}

fn leaf(ctx: &mut Ctx<'_>, frets: &[Option<Fret>]) {
    let Ok(v) = Voicing::new(frets.to_vec()) else {
        return;
    };
    if ctx.c.check(&v).is_err() {
        return;
    }
    let notes = v.sounding(ctx.tuning, ctx.capo).expect("same string count");
    let mut used = 0u16;
    for n in &notes {
        used |= bit(PitchClass::from_semitone(n.semitone));
    }
    if used & ctx.required != ctx.required {
        return;
    }
    if let Some(b) = ctx.bass {
        let lowest = notes
            .iter()
            .map(|n| n.semitone)
            .min()
            .expect("sounding notes");
        if PitchClass::from_semitone(lowest) != b {
            return;
        }
    }
    ctx.out.push(v);
}

/// Muted strings between the outermost sounding strings.
fn inner_mutes(v: &Voicing) -> usize {
    let f = v.frets();
    match (
        f.iter().position(Option::is_some),
        f.iter().rposition(Option::is_some),
    ) {
        (Some(a), Some(b)) => f[a..=b].iter().filter(|x| x.is_none()).count(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_theory::chord_symbol;

    fn shapes(symbol: &str, tuning: &Tuning, options: &SearchOptions) -> Vec<String> {
        let chord = chord_symbol::parse(symbol).unwrap();
        find_voicings(&chord, tuning, Capo(0), options)
            .iter()
            .map(|v| v.to_string())
            .collect()
    }

    #[test]
    fn finds_common_open_and_barre_shapes() {
        let t = Tuning::standard_guitar();
        let all = SearchOptions {
            max_results: usize::MAX,
            ..SearchOptions::default()
        };
        let c = shapes("C", &t, &all);
        assert_eq!(c[0], "x32010");
        assert!(c.contains(&"x35553".to_string()));
        let g = shapes("G", &t, &all);
        assert!(g.contains(&"320003".to_string()));
        assert!(g.contains(&"355433".to_string()));
        let limited = shapes("G", &t, &SearchOptions::default());
        assert!(limited.len() <= 50);
    }
}
