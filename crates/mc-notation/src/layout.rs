//! TAB layout of one track.
//!
//! 1. Measure every bar: one column per distinct beat onset across all voices; a
//!    column's natural width grows with the time to the next onset (logarithmic
//!    duration spacing) and is at least as wide as its widest fret number.
//! 2. Break into systems greedily by natural width (at least one bar per system).
//! 3. Justify: column widths are scaled so each system fills the content width. The
//!    last system is left at natural width when it would be stretched by more than a
//!    third.
//! 4. Emit lines, glyphs and hit regions per system.
//!
//! Layout is deterministic for identical inputs (bounded-contexts section 7). Floats
//! are used only for geometry; musical time stays rational (ADR-011).

use crate::display::*;
use crate::{LayoutMetrics, NotationError};
use mc_score::{Beat, MasterBar, Score, Track, TrackId};
use mc_theory::Rational;
use std::collections::BTreeSet;

/// Lays out every bar of `track`. The same as the single stave of
/// [`layout_score`] with only this track (ADR-031 M1).
pub fn layout_tab(
    score: &Score,
    track: TrackId,
    metrics: &LayoutMetrics,
) -> Result<TabLayout, NotationError> {
    let layout = layout_score(score, &[track], metrics)?;
    let strings = score
        .track(track)
        .expect("checked")
        .fretted()
        .tuning
        .string_count();
    Ok(TabLayout {
        track,
        strings,
        width: layout.width,
        height: layout.height,
        systems: layout
            .systems
            .into_iter()
            .map(|s| s.staves.into_iter().next().expect("one stave").layout)
            .collect(),
    })
}

/// Lays out every bar of the shown `tracks` together (ADR-031): each system has one
/// stave per track, in score order (duplicates ignored). Bar lines and beats that
/// start together line up across staves; bars are as wide as the busiest track needs.
pub fn layout_score(
    score: &Score,
    tracks: &[TrackId],
    metrics: &LayoutMetrics,
) -> Result<ScoreLayout, NotationError> {
    metrics.validate()?;
    for &t in tracks {
        score.track(t).ok_or(NotationError::UnknownTrack(t))?;
    }
    let shown: Vec<&Track> = score
        .tracks()
        .filter(|t| tracks.contains(&t.id()))
        .collect();
    if shown.is_empty() {
        return Err(NotationError::NoTracks);
    }
    let count = score.master_bar_count();
    let mut diagnostics = Vec::with_capacity(shown.len());
    for t in &shown {
        let bars: BTreeSet<usize> = score
            .track_diagnostics(t.id(), 0..count)
            .map_err(|_| NotationError::Overflow)?
            .into_iter()
            .map(|d| d.bar)
            .collect();
        diagnostics.push(bars);
    }
    let plans = (0..count)
        .map(|b| plan_bar(score, &shown, b, metrics))
        .collect::<Result<Vec<_>, _>>()?;
    let ctxs: Vec<Ctx> = shown
        .iter()
        .enumerate()
        .map(|(stave, track)| Ctx {
            m: metrics,
            track,
            stave,
            strings: track.fretted().tuning.string_count(),
            master: score.master_bars(),
        })
        .collect();

    let available = metrics.content_width();
    let mut systems = Vec::new();
    let mut y = 0.0f32;
    let mut start = 0;
    while start < plans.len() {
        let mut end = start + 1;
        let mut used = plans[start].natural();
        while end < plans.len() && used + plans[end].natural() <= available {
            used += plans[end].natural();
            end += 1;
        }
        let group = &plans[start..end];
        let scale = system_scale(group, end == plans.len(), metrics);
        let index = systems.len();
        let mut staves = Vec::with_capacity(ctxs.len());
        let mut top = 0.0f32;
        for (ctx, diag) in ctxs.iter().zip(&diagnostics) {
            let layout = ctx.emit_system(index, y + top, group, scale, diag);
            let height = layout.height;
            staves.push(Stave {
                track: ctx.track.id(),
                strings: ctx.strings,
                top,
                layout,
            });
            top += height;
        }
        systems.push(ScoreSystem {
            index,
            y,
            height: top,
            staves,
        });
        y += top + metrics.system_gap;
        start = end;
    }
    let height = if systems.is_empty() {
        0.0
    } else {
        y - metrics.system_gap
    };
    Ok(ScoreLayout {
        tracks: shown.iter().map(|t| t.id()).collect(),
        width: metrics.width,
        height,
        systems,
    })
}

/// Justification factor of a system's flexible width, shared by all its staves. The
/// last system stays at natural width when it would stretch by more than a third.
fn system_scale(plans: &[BarPlan], is_last: bool, m: &LayoutMetrics) -> f32 {
    let fixed: f32 = plans.iter().map(|p| p.fixed_left + p.fixed_right).sum();
    let flexible: f32 = plans.iter().map(|p| p.flexible).sum();
    let target = m.content_width() - fixed;
    let scale = if flexible > 0.0 {
        target / flexible
    } else {
        1.0
    };
    if is_last && scale > 4.0 / 3.0 {
        1.0
    } else {
        scale
    }
}

fn to_f32(r: Rational) -> f32 {
    (r.numerator() as f64 / r.denominator() as f64) as f32
}

/// One track's beats in a measured bar.
struct StavePlan {
    /// Onset of every beat, per voice.
    voice_onsets: Vec<Vec<Rational>>,
    /// Column index of every beat, per voice.
    voice_cols: Vec<Vec<usize>>,
}

/// Measured bar before justification, shared by every stave of a system.
struct BarPlan {
    bar: usize,
    /// Per shown track, in stave order.
    staves: Vec<StavePlan>,
    /// Natural width per column (scaled during justification). Columns are the
    /// distinct onsets of all voices of all shown tracks, ascending (ADR-031 5.1), so
    /// beats that start together line up across staves.
    columns: Vec<f32>,
    /// Width of the time signature, repeat start sign and left padding.
    fixed_left: f32,
    fixed_right: f32,
    show_time_signature: bool,
    /// Natural flexible width: the column sum, or the empty-bar width.
    flexible: f32,
}

impl BarPlan {
    fn natural(&self) -> f32 {
        self.fixed_left + self.flexible + self.fixed_right
    }
}

fn plan_bar(
    score: &Score,
    tracks: &[&Track],
    b: usize,
    m: &LayoutMetrics,
) -> Result<BarPlan, NotationError> {
    let master = &score.master_bars()[b];
    let nominal = score.nominal_bar_duration(b).expect("bar exists");
    // Onsets per voice of every track, computed once (each rational addition reduces
    // by gcd). With one track this is exactly the single-track plan.
    let mut end = nominal;
    let mut track_onsets: Vec<Vec<Vec<Rational>>> = Vec::with_capacity(tracks.len());
    for track in tracks {
        let content = track.bar(b).expect("H-02: bars aligned");
        let mut voice_onsets = Vec::with_capacity(content.voices.len());
        for voice in &content.voices {
            let mut t = Rational::ZERO;
            let mut list = Vec::with_capacity(voice.beats.len());
            for beat in &voice.beats {
                list.push(t);
                t = t
                    .checked_add(beat.value.length())
                    .ok_or(NotationError::Overflow)?;
            }
            end = end.max(t);
            voice_onsets.push(list);
        }
        track_onsets.push(voice_onsets);
    }
    let mut onsets: Vec<Rational> = track_onsets.iter().flatten().flatten().copied().collect();
    onsets.sort_unstable();
    onsets.dedup();
    let mut widest = vec![0.0f32; onsets.len()];
    let mut staves = Vec::with_capacity(tracks.len());
    for (track, voice_onsets) in tracks.iter().zip(track_onsets) {
        let content = track.bar(b).expect("H-02: bars aligned");
        let voice_cols: Vec<Vec<usize>> = voice_onsets
            .iter()
            .map(|list| {
                list.iter()
                    .map(|t| onsets.binary_search(t).expect("collected above"))
                    .collect()
            })
            .collect();
        for (voice, cols) in content.voices.iter().zip(&voice_cols) {
            for (beat, &c) in voice.beats.iter().zip(cols) {
                for n in &beat.notes {
                    let w = fret_glyph_width(n.position().fret.0, n.tie, n.effects.dead, m);
                    widest[c] = widest[c].max(w);
                }
            }
        }
        staves.push(StavePlan {
            voice_onsets,
            voice_cols,
        });
    }
    let mut columns = Vec::with_capacity(onsets.len());
    for (i, onset) in onsets.iter().enumerate() {
        let next = onsets.get(i + 1).copied().unwrap_or(end);
        let gap = to_f32(next.checked_sub(*onset).ok_or(NotationError::Overflow)?);
        let factor = (1.0 + m.duration_spacing * (gap * 16.0).log2()).max(0.6);
        columns.push((m.min_column_width * factor).max(widest[i] + 2.0 * m.glyph_padding));
    }
    let previous = b.checked_sub(1).map(|p| &score.master_bars()[p]);
    let show_time_signature = previous.is_none_or(|p| p.time_signature != master.time_signature);
    let fixed_left = m.bar_padding
        + if show_time_signature {
            m.time_signature_width
        } else {
            0.0
        }
        + if master.repeat_start {
            m.repeat_width
        } else {
            0.0
        };
    let fixed_right = m.bar_padding
        + if master.repeat_end.is_some() {
            m.repeat_width
        } else {
            0.0
        };
    let flexible = if columns.is_empty() {
        m.empty_bar_width
    } else {
        columns.iter().sum()
    };
    Ok(BarPlan {
        bar: b,
        staves,
        columns,
        fixed_left,
        fixed_right,
        show_time_signature,
        flexible,
    })
}

fn fret_glyph_width(fret: u8, tied: bool, dead: bool, m: &LayoutMetrics) -> f32 {
    if dead {
        return m.digit_width;
    }
    let digits = if fret >= 100 {
        3.0
    } else if fret >= 10 {
        2.0
    } else {
        1.0
    };
    digits * m.digit_width + if tied { 1.2 * m.digit_width } else { 0.0 }
}

/// Emits one stave: `track` is the `stave`-th track of the bar plans.
struct Ctx<'a> {
    m: &'a LayoutMetrics,
    track: &'a Track,
    stave: usize,
    strings: u8,
    master: &'a [MasterBar],
}

struct Out {
    lines: Vec<Line>,
    glyphs: Vec<Glyph>,
    hits: Vec<HitRegion>,
}

impl Out {
    fn line(&mut self, kind: LineKind, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.lines.push(Line {
            kind,
            x1,
            y1,
            x2,
            y2,
        });
    }

    fn glyph(&mut self, kind: GlyphKind, x: f32, y: f32, width: f32) {
        self.glyphs.push(Glyph { kind, x, y, width });
    }
}

impl Ctx<'_> {
    fn staff_top(&self) -> f32 {
        self.m.above_staff
    }

    fn string_y(&self, string: u8) -> f32 {
        self.staff_top() + (string as f32 - 1.0) * self.m.string_spacing
    }

    fn staff_bottom(&self) -> f32 {
        self.string_y(self.strings)
    }

    /// One stave of a system, in stave coordinates (`y` is the system's document y).
    /// `scale` is shared by every stave of the system so bar lines line up.
    fn emit_system(
        &self,
        index: usize,
        y: f32,
        plans: &[BarPlan],
        scale: f32,
        diagnostic_bars: &BTreeSet<usize>,
    ) -> SystemLayout {
        let m = self.m;
        let mut out = Out {
            lines: Vec::new(),
            glyphs: Vec::new(),
            hits: Vec::new(),
        };
        let top = self.staff_top();
        let bottom = self.staff_bottom();
        let middle = (top + bottom) / 2.0;

        // Header: TAB clef on every system, open-string names on the first.
        let header_x = m.margin_left;
        out.glyph(
            GlyphKind::TabClef,
            header_x + m.header_width * 0.65,
            middle,
            m.header_width * 0.5,
        );
        if index == 0 {
            for (i, p) in self.track.fretted().tuning.strings().iter().enumerate() {
                let string = i as u8 + 1;
                let name = p.to_string();
                let name = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '-');
                out.glyph(
                    GlyphKind::StringName {
                        string,
                        name: name.to_string(),
                    },
                    header_x + m.digit_width * 0.6,
                    self.string_y(string),
                    m.digit_width * name.len() as f32,
                );
            }
        }

        let mut x = m.margin_left + m.header_width;
        let mut bars = Vec::with_capacity(plans.len());
        for (k, plan) in plans.iter().enumerate() {
            let width = plan.fixed_left + plan.flexible * scale + plan.fixed_right;
            let last_in_system = k + 1 == plans.len();
            self.emit_bar(&mut out, plan, x, width, scale, last_in_system);
            if k == 0 {
                out.glyph(
                    GlyphKind::BarNumber {
                        number: plan.bar + 1,
                    },
                    x + m.digit_width,
                    top - m.above_staff * 0.35,
                    m.digit_width * digits(plan.bar + 1) as f32,
                );
            }
            bars.push(BarLayout {
                bar: plan.bar,
                x,
                width,
                diagnostic: diagnostic_bars.contains(&plan.bar),
            });
            x += width;
        }
        let staff_start = m.margin_left;
        for s in 1..=self.strings {
            let sy = self.string_y(s);
            out.lines.insert(
                (s - 1) as usize,
                Line {
                    kind: LineKind::String,
                    x1: staff_start,
                    y1: sy,
                    x2: x,
                    y2: sy,
                },
            );
        }
        SystemLayout {
            index,
            y,
            height: bottom + m.below_staff,
            staff_top: top,
            bars,
            lines: out.lines,
            glyphs: out.glyphs,
            hits: out.hits,
        }
    }

    fn emit_bar(
        &self,
        out: &mut Out,
        plan: &BarPlan,
        x: f32,
        width: f32,
        scale: f32,
        last_in_system: bool,
    ) {
        let m = self.m;
        let master = &self.master[plan.bar];
        let top = self.staff_top();
        let bottom = self.staff_bottom();
        let middle = (top + bottom) / 2.0;
        let end = x + width;

        // Left bar line or repeat start (thick, thin, dots).
        let mut cursor = x;
        if master.repeat_start {
            out.line(LineKind::ThickBarLine, x + 1.0, top, x + 1.0, bottom);
            out.line(LineKind::BarLine, x + 4.0, top, x + 4.0, bottom);
            out.glyph(
                GlyphKind::RepeatDots,
                x + m.repeat_width * 0.75,
                middle,
                2.0,
            );
            cursor += m.repeat_width;
        } else {
            out.line(LineKind::BarLine, x, top, x, bottom);
        }
        if plan.show_time_signature {
            out.glyph(
                GlyphKind::TimeSignature {
                    numerator: master.time_signature.numerator(),
                    denominator: master.time_signature.denominator(),
                },
                cursor + m.bar_padding * 0.5 + m.time_signature_width * 0.5,
                middle,
                m.time_signature_width,
            );
            cursor += m.time_signature_width;
        }
        cursor += m.bar_padding;
        let content_start = cursor;

        // Right side: repeat end, final bar line or system end line.
        let is_final = plan.bar + 1 == self.master.len();
        if let Some(count) = master.repeat_end {
            out.glyph(
                GlyphKind::RepeatDots,
                end - m.repeat_width * 0.75,
                middle,
                2.0,
            );
            out.line(LineKind::BarLine, end - 4.0, top, end - 4.0, bottom);
            out.line(LineKind::ThickBarLine, end - 1.0, top, end - 1.0, bottom);
            out.glyph(
                GlyphKind::RepeatCount { count },
                end - m.repeat_width,
                top - m.string_spacing * 0.8,
                m.digit_width * 2.0,
            );
        } else if is_final {
            out.line(LineKind::BarLine, end - 4.0, top, end - 4.0, bottom);
            out.line(LineKind::ThickBarLine, end - 1.0, top, end - 1.0, bottom);
        } else if last_in_system {
            out.line(LineKind::BarLine, end, top, end, bottom);
        }

        // Volta bracket and marker above the staff.
        if !master.alternate_endings.is_empty() {
            let vy = top - m.above_staff * 0.6;
            out.line(LineKind::Volta, x + 2.0, vy, end - 2.0, vy);
            out.line(
                LineKind::Volta,
                x + 2.0,
                vy,
                x + 2.0,
                vy + m.string_spacing * 0.6,
            );
            out.glyph(
                GlyphKind::Volta {
                    endings: master.alternate_endings.clone(),
                },
                x + 2.0 + m.digit_width * 1.5,
                vy + m.string_spacing * 0.5,
                m.digit_width * 2.0 * master.alternate_endings.len() as f32,
            );
        }
        if let Some(text) = &master.marker {
            out.glyph(
                GlyphKind::Marker { text: text.clone() },
                x + m.bar_padding + m.digit_width * text.chars().count() as f32 * 0.5,
                top - m.above_staff * 0.85,
                m.digit_width * text.chars().count() as f32,
            );
        }

        // Columns.
        let empty = plan.columns.is_empty();
        let mut col_x = Vec::with_capacity(plan.columns.len());
        let mut cx = content_start;
        for w in &plan.columns {
            col_x.push((cx, w * scale));
            cx += w * scale;
        }
        let content_end = if empty {
            content_start + m.empty_bar_width * scale
        } else {
            cx
        };
        let inner_end = end
            - m.bar_padding.min(width)
            - if master.repeat_end.is_some() {
                m.repeat_width
            } else {
                0.0
            };

        let content = self.track.bar(plan.bar).expect("H-02");
        let stave = &plan.staves[self.stave];
        for (v, voice) in content.voices.iter().enumerate() {
            let mut placed = Vec::with_capacity(voice.beats.len());
            for (k, beat) in voice.beats.iter().enumerate() {
                let t = stave.voice_onsets[v][k];
                let (bx, bw) = col_x[stave.voice_cols[v][k]];
                let center = bx + bw / 2.0;
                for note in &beat.notes {
                    let p = note.position();
                    out.glyph(
                        GlyphKind::Fret {
                            fret: p.fret.0,
                            tied: note.tie,
                            dead: note.effects.dead,
                            let_ring: note.effects.let_ring,
                        },
                        center,
                        self.string_y(p.string.get()),
                        fret_glyph_width(p.fret.0, note.tie, note.effects.dead, m),
                    );
                }
                for s in 1..=self.strings {
                    out.hits.push(HitRegion {
                        rect: self.string_rect(bx, bw, s),
                        bar: plan.bar,
                        string: s,
                        target: HitTarget::Beat {
                            voice: v,
                            beat: k,
                            beat_id: beat.id,
                        },
                    });
                }
                placed.push((center, t, beat));
            }
            self.emit_rhythm(out, master, v, &placed);
            // Append slot after the last beat (the whole content area when empty).
            let (ax, aw) = if voice.beats.is_empty() {
                (
                    content_start,
                    (content_end.max(inner_end) - content_start).max(1.0),
                )
            } else {
                (content_end, (end - content_end).max(1.0))
            };
            for s in 1..=self.strings {
                out.hits.push(HitRegion {
                    rect: self.string_rect(ax, aw, s),
                    bar: plan.bar,
                    string: s,
                    target: HitTarget::Append { voice: v },
                });
            }
        }
    }

    fn string_rect(&self, x: f32, w: f32, string: u8) -> Rect {
        let h = self.m.string_spacing;
        Rect {
            x,
            y: self.string_y(string) - h / 2.0,
            w,
            h,
        }
    }

    /// Stems, beams, flags, dots, rests and tuplet numbers of one voice. Voice 0 has
    /// its rhythm below the staff, other voices above.
    fn emit_rhythm(
        &self,
        out: &mut Out,
        master: &MasterBar,
        voice: usize,
        placed: &[(f32, Rational, &Beat)],
    ) {
        let m = self.m;
        let down = voice == 0;
        let dir = if down { 1.0 } else { -1.0 };
        let base_y = if down {
            self.staff_bottom() + m.string_spacing * 0.4
        } else {
            self.staff_top() - m.string_spacing * 0.4
        };
        let stem_end = |exponent: u8| {
            let len = if exponent == 1 {
                m.stem_length * 0.5
            } else {
                m.stem_length
            };
            base_y + dir * len
        };
        let beam_gap = m.string_spacing * 0.35;
        let stub = m.digit_width * 1.2;
        let window = beam_window(master);

        // Stems, dots and rests.
        for (x, _, beat) in placed {
            let e = beat.value.base().exponent();
            let dots = beat.value.dots();
            if beat.notes.is_empty() {
                out.glyph(
                    GlyphKind::Rest { exponent: e, dots },
                    *x,
                    base_y + dir * m.stem_length * 0.6,
                    m.digit_width,
                );
                continue;
            }
            if e >= 1 {
                out.line(LineKind::Stem, *x, base_y, *x, stem_end(e));
            }
            for d in 0..dots {
                out.glyph(
                    GlyphKind::Dot,
                    x + m.digit_width * (0.7 + 0.5 * d as f32),
                    stem_end(e.max(1)) - dir * m.string_spacing * 0.3,
                    2.0,
                );
            }
        }

        // Beam groups: consecutive non-rest beats shorter than a quarter whose onsets
        // fall in the same beam window.
        let mut i = 0;
        while i < placed.len() {
            let levels = |k: usize| -> u8 {
                let b = placed[k].2;
                if b.notes.is_empty() {
                    0
                } else {
                    b.value.base().exponent().saturating_sub(2)
                }
            };
            if levels(i) == 0 {
                i += 1;
                continue;
            }
            let group_window = window_index(placed[i].1, window);
            let mut j = i + 1;
            while j < placed.len()
                && levels(j) > 0
                && window_index(placed[j].1, window) == group_window
            {
                j += 1;
            }
            let members: Vec<(f32, u8)> = (i..j).map(|k| (placed[k].0, levels(k))).collect();
            let y_at = |level: u8| stem_end(2) - dir * (level as f32 - 1.0) * beam_gap;
            let max_level = members.iter().map(|(_, l)| *l).max().unwrap_or(0);
            for level in 1..=max_level {
                let y = y_at(level);
                if members.len() == 1 {
                    let (x, _) = members[0];
                    out.line(LineKind::Beam, x, y, x + stub, y);
                    continue;
                }
                for k in 0..members.len() {
                    let (x, l) = members[k];
                    if l < level {
                        continue;
                    }
                    let next = members.get(k + 1).filter(|(_, nl)| *nl >= level);
                    let prev = k
                        .checked_sub(1)
                        .map(|p| members[p])
                        .filter(|(_, pl)| *pl >= level);
                    if let Some((nx, _)) = next {
                        out.line(LineKind::Beam, x, y, *nx, y);
                    } else if prev.is_none() {
                        let toward = if k + 1 < members.len() { stub } else { -stub };
                        out.line(LineKind::Beam, x, y, x + toward, y);
                    }
                }
            }
            i = j;
        }

        // Tuplet numbers: runs of beats with the same ratio, `actual` beats per group.
        let mut i = 0;
        while i < placed.len() {
            let Some(tuplet) = placed[i].2.value.tuplet() else {
                i += 1;
                continue;
            };
            let mut j = i + 1;
            while j < placed.len()
                && j - i < tuplet.actual() as usize
                && placed[j].2.value.tuplet() == Some(tuplet)
            {
                j += 1;
            }
            let cx = (placed[i].0 + placed[j - 1].0) / 2.0;
            out.glyph(
                GlyphKind::Tuplet {
                    actual: tuplet.actual(),
                },
                cx,
                stem_end(2) + dir * m.string_spacing * 0.8,
                m.digit_width * digits(tuplet.actual() as usize) as f32,
            );
            i = j;
        }
    }
}

/// Beam grouping window: a dotted quarter in compound meters (6/8, 9/8, 12/8 ...),
/// otherwise a quarter note.
fn beam_window(master: &MasterBar) -> Rational {
    let ts = master.time_signature;
    let (n, d) = (ts.numerator(), ts.denominator());
    if d >= 8 && n > 3 && n % 3 == 0 {
        Rational::new(3, d as i64).expect("non-zero")
    } else {
        Rational::new(1, 4).expect("non-zero")
    }
}

fn window_index(onset: Rational, window: Rational) -> i64 {
    let q = onset.checked_div(window).unwrap_or(Rational::ZERO);
    q.numerator().div_euclid(q.denominator())
}

fn digits(n: usize) -> usize {
    n.to_string().len()
}
