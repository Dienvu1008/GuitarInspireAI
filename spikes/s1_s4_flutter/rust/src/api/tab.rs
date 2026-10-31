//! S4: TAB layout computed in Rust as a display list in abstract units.
//! Dart supplies metrics (width, string spacing, digit width) and only paints.

use flutter_rust_bridge::frb;
use std::time::Instant;

pub struct LayoutMetrics {
    pub width: f64,
    pub string_spacing: f64,
    pub digit_width: f64,
    pub bars_per_system: u32,
    pub margin: f64,
}

/// Display list for one system (line of bars).
pub struct SystemLayout {
    pub index: u32,
    pub height: f64,
    /// Line segments as x1, y1, x2, y2 quadruples.
    pub lines: Vec<f32>,
    /// Glyph anchors as x, y pairs (centre of the fret number).
    pub glyph_pos: Vec<f32>,
    /// Fret number per glyph.
    pub glyph_fret: Vec<u8>,
    /// Time spent computing this layout in Rust.
    pub layout_us: u32,
}

#[frb(opaque)]
pub struct TabScore {
    /// bar -> beats -> notes as (string 1..=6, fret)
    bars: Vec<Vec<Vec<(u8, u8)>>>,
}

impl TabScore {
    #[frb(sync)]
    pub fn generate(bars: u32, seed: u64) -> TabScore {
        let mut x = seed.wrapping_mul(0x9E3779B97F4A7C15) | 1;
        let mut rnd = move |m: u64| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x % m
        };
        let bars = (0..bars)
            .map(|_| {
                let beats = if rnd(4) == 0 { 16 } else { 8 };
                (0..beats)
                    .map(|_| {
                        let count = 1 + rnd(3) as u8;
                        let first = 1 + rnd(6) as u8;
                        (0..count)
                            .map(|i| (((first - 1 + i) % 6) + 1, rnd(16) as u8))
                            .collect()
                    })
                    .collect()
            })
            .collect();
        TabScore { bars }
    }

    #[frb(sync)]
    pub fn system_count(&self, bars_per_system: u32) -> u32 {
        (self.bars.len() as u32).div_ceil(bars_per_system)
    }

    #[frb(sync)]
    pub fn notes_total(&self) -> u32 {
        self.bars.iter().flatten().map(|b| b.len() as u32).sum()
    }

    #[frb(sync)]
    pub fn layout_system(&self, index: u32, metrics: LayoutMetrics) -> SystemLayout {
        let t0 = Instant::now();
        let m = &metrics;
        let per = m.bars_per_system as usize;
        let first = index as usize * per;
        let bars = &self.bars[first.min(self.bars.len())..(first + per).min(self.bars.len())];
        let top = m.margin;
        let staff_h = 5.0 * m.string_spacing;
        let stem_len = m.string_spacing * 1.5;
        let height = top * 2.0 + staff_h + stem_len;
        let usable = m.width - 2.0 * m.margin;
        let bar_w = usable / per as f64;
        let mut lines = Vec::with_capacity(64);
        let mut pos = Vec::with_capacity(256);
        let mut frets = Vec::with_capacity(128);
        let mut seg = |v: &mut Vec<f32>, x1: f64, y1: f64, x2: f64, y2: f64| v.extend([x1 as f32, y1 as f32, x2 as f32, y2 as f32]);
        for s in 0..6 {
            let y = top + s as f64 * m.string_spacing;
            seg(&mut lines, m.margin, y, m.margin + bars.len() as f64 * bar_w, y);
        }
        for (b, bar) in bars.iter().enumerate() {
            let bx = m.margin + b as f64 * bar_w;
            seg(&mut lines, bx, top, bx, top + staff_h);
            let pad = m.digit_width * 1.5;
            let step = (bar_w - 2.0 * pad) / bar.len() as f64;
            for (k, beat) in bar.iter().enumerate() {
                let x = bx + pad + (k as f64 + 0.5) * step;
                seg(&mut lines, x, top + staff_h + m.string_spacing * 0.3, x, top + staff_h + stem_len);
                for &(string, fret) in beat {
                    pos.push(x as f32);
                    pos.push((top + (string - 1) as f64 * m.string_spacing) as f32);
                    frets.push(fret);
                }
            }
        }
        let end = m.margin + bars.len() as f64 * bar_w;
        seg(&mut lines, end, top, end, top + staff_h);
        SystemLayout { index, height, lines, glyph_pos: pos, glyph_fret: frets, layout_us: t0.elapsed().as_micros() as u32 }
    }
}
