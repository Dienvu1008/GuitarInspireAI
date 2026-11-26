//! Pagination (ADR-041 section 11, slice 6c-5a): how the systems of a layout are
//! stacked on printed pages. A pure layout decision; no PDF bytes are written here.
//!
//! - One layout unit is one PDF point (1/72 in). Systems are laid out at the page's
//!   content width with the ordinary line breaking (`outline_score_with`); pagination
//!   only stacks them and re-lays nothing out.
//! - Pages are filled greedily from the top, with the layout's system gap between
//!   items. A page takes whole systems.
//! - A system taller than the body of the page it would start on is split between
//!   tracks, each track's notation and TAB kept together. Only a track taller than a
//!   page body on its own is split between its notation and TAB. Each stave carries its
//!   own clef and key, so a continued part repeats them. A single stave taller than
//!   every page body is an error.
//! - Page 1 reserves the title; every page reserves the footer ("page n of N"). The
//!   text itself is supplied when the page is drawn (6c-5b).

use crate::layout::outline_with_staves;
use crate::{LayoutMetrics, NotationError, Rect, ScoreOutline, ShownTrack};
use mc_score::Score;
use std::ops::Range;

/// Points per millimetre: 72 points per inch, 25.4 mm per inch.
const PT_PER_MM: f32 = 72.0 / 25.4;

/// Page margin on every side: 0.5 in (ADR-041 section 11, decided 2026-11-26).
pub const PAGE_MARGIN_PT: f32 = 36.0;
/// Space for the title at the top of page 1: one 20 pt title line (twice the 10 pt
/// text of the TAB digits) and as much again below it. Chosen in 6c-5a.
pub const TITLE_HEIGHT_PT: f32 = 40.0;
/// Space for "page n of N" at the foot of every page: one 10 pt line and as much again
/// above it. Chosen in 6c-5a.
pub const FOOTER_HEIGHT_PT: f32 = 20.0;
/// Print staff size as a factor of the default layout metrics (ADR-041 section 11.2,
/// user decision 2026-11-26): a TAB string gap of 7.2 pt and a staff space of 4.8 pt,
/// close to common printed guitar editions.
pub const PRINT_STAFF_SCALE: f32 = 0.6;
/// The "Spacious" gap between systems in staff spaces (ADR-041 section 12, choice 1):
/// room between a TAB rhythm and the next system's notation.
pub const SPACIOUS_SYSTEM_GAP_SP: f32 = 4.0;

/// The gap between systems on a printed page, chosen by the person exporting (ADR-041
/// section 12, choice 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SystemSpacing {
    /// [`SPACIOUS_SYSTEM_GAP_SP`] staff spaces (19.2 pt at the print size).
    #[default]
    Spacious,
    /// The default layout gap times the staff scale (9.6 pt at the print size).
    Compact,
}

/// A paper size in points, portrait.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageSize {
    pub width: f32,
    pub height: f32,
}

impl PageSize {
    /// ISO 216 A4: 210 x 297 mm (595.28 x 841.89 pt). The default.
    pub const A4: PageSize = PageSize {
        width: 210.0 * PT_PER_MM,
        height: 297.0 * PT_PER_MM,
    };
    /// US Letter (ANSI A): 8.5 x 11 in (612 x 792 pt).
    pub const LETTER: PageSize = PageSize {
        width: 8.5 * 72.0,
        height: 11.0 * 72.0,
    };
}

/// Paper size and the space reserved on it, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageSetup {
    pub size: PageSize,
    /// The same on all four sides.
    pub margin: f32,
    /// Reserved on page 1 below the top margin.
    pub title_height: f32,
    /// Reserved on every page above the bottom margin.
    pub footer_height: f32,
    /// Factor applied to every length of the default layout metrics (see
    /// [`PageSetup::metrics`]).
    pub staff_scale: f32,
    pub system_spacing: SystemSpacing,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self::new(PageSize::A4)
    }
}

impl PageSetup {
    /// `size` with the default margin, title, footer and print staff size.
    pub fn new(size: PageSize) -> Self {
        PageSetup {
            size,
            margin: PAGE_MARGIN_PT,
            title_height: TITLE_HEIGHT_PT,
            footer_height: FOOTER_HEIGHT_PT,
            staff_scale: PRINT_STAFF_SCALE,
            system_spacing: SystemSpacing::default(),
        }
    }

    /// Width between the left and right margins: the layout width.
    pub fn content_width(&self) -> f32 {
        self.size.width - 2.0 * self.margin
    }

    /// The title area of page 1.
    pub fn title(&self) -> Rect {
        Rect {
            x: self.margin,
            y: self.margin,
            w: self.content_width(),
            h: self.title_height,
        }
    }

    /// The footer area of every page.
    pub fn footer(&self) -> Rect {
        Rect {
            x: self.margin,
            y: self.size.height - self.margin - self.footer_height,
            w: self.content_width(),
            h: self.footer_height,
        }
    }

    /// Where systems go on page `number` (1-based): between the margins, below the
    /// title on page 1 and above the footer.
    pub fn body(&self, number: usize) -> Rect {
        let top = self.margin + if number == 1 { self.title_height } else { 0.0 };
        Rect {
            x: self.margin,
            y: top,
            w: self.content_width(),
            h: self.footer().y - top,
        }
    }

    /// The layout metrics of the printed page, in points: every length of
    /// `LayoutMetrics::default()` times `staff_scale` (the duration spacing is a ratio
    /// and is kept), at this page's content width. The page margins replace the layout
    /// margins: the left one is 0, and the right one is half the thick bar line, whose
    /// centre the layout puts at the end of the system, so the final bar line ends at
    /// the content edge (found by PR3). The screen metrics and zoom take no part.
    pub fn metrics(&self) -> LayoutMetrics {
        let m = LayoutMetrics::default();
        let k = self.staff_scale;
        let string_spacing = m.string_spacing * k;
        let staff_space = string_spacing / crate::TAB_STRING_GAP_SP;
        LayoutMetrics {
            width: self.content_width(),
            margin_left: 0.0,
            margin_right: crate::smufl::DEFAULTS.thick_barline_thickness / 2.0 * staff_space,
            string_spacing,
            digit_width: m.digit_width * k,
            glyph_padding: m.glyph_padding * k,
            min_column_width: m.min_column_width * k,
            duration_spacing: m.duration_spacing,
            bar_padding: m.bar_padding * k,
            empty_bar_width: m.empty_bar_width * k,
            header_width: m.header_width * k,
            time_signature_width: m.time_signature_width * k,
            repeat_width: m.repeat_width * k,
            above_staff: m.above_staff * k,
            below_staff: m.below_staff * k,
            system_gap: match self.system_spacing {
                SystemSpacing::Spacious => SPACIOUS_SYSTEM_GAP_SP * staff_space,
                SystemSpacing::Compact => m.system_gap * k,
            },
            stem_length: m.stem_length * k,
        }
    }

    fn validate(&self) -> Result<(), NotationError> {
        let values = [
            ("width", self.size.width),
            ("height", self.size.height),
            ("margin", self.margin),
            ("title_height", self.title_height),
            ("footer_height", self.footer_height),
            ("staff_scale", self.staff_scale),
        ];
        for (name, v) in values {
            if !v.is_finite() || v < 0.0 {
                return Err(NotationError::InvalidPage(format!(
                    "{name} must be finite and >= 0"
                )));
            }
        }
        if self.staff_scale <= 0.0 {
            return Err(NotationError::InvalidPage("staff_scale must be > 0".into()));
        }
        if self.content_width() <= 0.0 || self.body(1).h <= 0.0 {
            return Err(NotationError::InvalidPage(
                "the margins, title and footer leave no page body".into(),
            ));
        }
        Ok(())
    }
}

/// Some or all staves of one system, placed on a page.
#[derive(Clone, Debug, PartialEq)]
pub struct PageItem {
    /// Index of the system in the outline.
    pub system: usize,
    /// The staves placed, as indices into the system's staves (emission order).
    pub staves: Range<usize>,
    /// Top of the first stave in page coordinates (points from the page top).
    pub y: f32,
    /// Height of the staves placed: the distance from the top of the first to the
    /// bottom of the last, as laid out.
    pub height: f32,
    /// Every stave of the system is in this item.
    pub whole: bool,
}

impl PageItem {
    /// The item continues a system begun on an earlier page.
    pub fn continued(&self) -> bool {
        self.staves.start > 0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    /// 1-based page number, for "page n of N".
    pub number: usize,
    /// The title area (page 1 only).
    pub title: Option<Rect>,
    pub body: Rect,
    pub footer: Rect,
    /// Top to bottom.
    pub items: Vec<PageItem>,
}

/// The pages of a score, with the outline and metrics the systems were laid out with.
/// Lay out the content of a system with `layout_score_systems(score, &outline, ..,
/// &metrics)`; item y minus the first stave's `top` places it on the page.
#[derive(Clone, Debug, PartialEq)]
pub struct Pagination {
    pub setup: PageSetup,
    pub metrics: LayoutMetrics,
    pub outline: ScoreOutline,
    /// At least one page.
    pub pages: Vec<Page>,
}

impl Pagination {
    /// N in "page n of N".
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
}

/// Paginates the shown `tracks` of `score` (what the view shows, ADR-041 section 11
/// choice 4) at the print metrics of `setup` ([`PageSetup::metrics`], section 11.2).
///
/// Errors: invalid page setup, unknown tracks, and [`NotationError::PageTooSmall`]
/// when one stave is taller than every page body.
pub fn paginate(
    score: &Score,
    tracks: &[ShownTrack],
    setup: &PageSetup,
) -> Result<Pagination, NotationError> {
    setup.validate()?;
    let metrics = setup.metrics();
    let (outline, staves) = outline_with_staves(score, tracks, &metrics)?;
    let mut packer = Packer::new(setup, metrics.system_gap);
    for (system, heights) in staves.iter().enumerate() {
        packer.system(system, heights)?;
    }
    Ok(Pagination {
        setup: *setup,
        metrics,
        outline,
        pages: packer.pages,
    })
}

struct Packer<'a> {
    setup: &'a PageSetup,
    gap: f32,
    pages: Vec<Page>,
}

impl<'a> Packer<'a> {
    fn new(setup: &'a PageSetup, gap: f32) -> Self {
        let mut p = Packer {
            setup,
            gap,
            pages: Vec::new(),
        };
        p.new_page();
        p
    }

    fn new_page(&mut self) {
        let number = self.pages.len() + 1;
        self.pages.push(Page {
            number,
            title: (number == 1).then(|| self.setup.title()),
            body: self.setup.body(number),
            footer: self.setup.footer(),
            items: Vec::new(),
        });
    }

    fn page(&self) -> &Page {
        self.pages.last().expect("at least one page")
    }

    /// Top of a new item on the current page.
    fn next_top(&self) -> f32 {
        let page = self.page();
        match page.items.last() {
            Some(last) => last.y + last.height + self.gap,
            None => page.body.y,
        }
    }

    /// Places one system with stave `heights` (emission order: track position and
    /// height).
    fn system(&mut self, system: usize, heights: &[(usize, f32)]) -> Result<(), NotationError> {
        // Stave tops within the system, summed in emission order as the layout does.
        let mut tops = Vec::with_capacity(heights.len() + 1);
        let mut top = 0.0f32;
        tops.push(top);
        for (_, h) in heights {
            top += h;
            tops.push(top);
        }
        let n = heights.len();
        let total = tops[n];
        let whole_fits = |p: &Self| p.next_top() + total <= p.page().body.bottom();
        if !whole_fits(self) && !self.page().items.is_empty() {
            let next = self.setup.body(self.pages.len() + 1);
            if total <= next.h {
                self.new_page();
            }
        }
        if whole_fits(self) {
            let y = self.next_top();
            self.push(system, 0..n, y, &tops);
            return Ok(());
        }
        // Taller than the body of the page it would start on: split between tracks,
        // then, for a track taller than a body, between its staves.
        let mut start = 0;
        while start < n {
            let track = heights[start].0;
            let mut end = start + 1;
            while end < n && heights[end].0 == track {
                end += 1;
            }
            if !self.part(system, start..end, &tops, false) {
                for stave in start..end {
                    if !self.part(system, stave..stave + 1, &tops, true) {
                        return Err(NotationError::PageTooSmall { system, stave });
                    }
                }
            }
            start = end;
        }
        Ok(())
    }

    /// Places staves `range` of a split system: appended to the system's item on this
    /// page when it is the last one, else as a new item, on this page or a new one.
    /// False when the range does not fit even on an empty page. A single stave
    /// (`skip_short`) that does not fit the shorter body of an empty page 1 goes on
    /// page 2, leaving page 1 with the title only, rather than failing.
    fn part(&mut self, system: usize, range: Range<usize>, tops: &[f32], skip_short: bool) -> bool {
        let h = tops[range.end] - tops[range.start];
        for _ in 0..3 {
            let page = self.page();
            let bottom = page.body.bottom();
            let joins = page
                .items
                .last()
                .is_some_and(|l| l.system == system && l.staves.end == range.start);
            if joins {
                let last = page.items.last().expect("checked");
                if last.y + last.height + h <= bottom {
                    let last = self.last_item();
                    last.staves.end = range.end;
                    last.height = tops[range.end] - tops[last.staves.start];
                    last.whole = last.staves.start == 0 && last.staves.end + 1 == tops.len();
                    return true;
                }
            } else if self.next_top() + h <= bottom {
                let y = self.next_top();
                self.push(system, range, y, tops);
                return true;
            }
            if self.page().items.is_empty() {
                let next = self.setup.body(self.pages.len() + 1);
                if !(skip_short && self.page().body.h < next.h && h <= next.h) {
                    return false;
                }
            }
            self.new_page();
        }
        false
    }

    fn push(&mut self, system: usize, staves: Range<usize>, y: f32, tops: &[f32]) {
        let item = PageItem {
            system,
            whole: staves.start == 0 && staves.end + 1 == tops.len(),
            height: tops[staves.end] - tops[staves.start],
            staves,
            y,
        };
        self.pages
            .last_mut()
            .expect("at least one page")
            .items
            .push(item);
    }

    fn last_item(&mut self) -> &mut PageItem {
        self.pages
            .last_mut()
            .expect("at least one page")
            .items
            .last_mut()
            .expect("checked")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PG5: the page geometry as numbers.
    #[test]
    fn pg5_page_sizes_margins_and_content_width() {
        let near = |a: f32, b: f32| (a - b).abs() < 0.005;
        assert!(near(PageSize::A4.width, 595.28) && near(PageSize::A4.height, 841.89));
        assert_eq!(
            PageSize::LETTER,
            PageSize {
                width: 612.0,
                height: 792.0
            }
        );
        let a4 = PageSetup::default();
        assert_eq!(a4.size, PageSize::A4);
        assert_eq!(a4.margin, 36.0);
        assert!(near(a4.content_width(), 523.28));
        assert_eq!(PageSetup::new(PageSize::LETTER).content_width(), 540.0);
        // A4: body of page 1 from 36 + 40 to 841.89 - 36 - 20; later pages from 36.
        assert!(near(a4.body(1).y, 76.0) && near(a4.body(1).h, 709.89));
        assert!(near(a4.body(2).y, 36.0) && near(a4.body(2).h, 749.89));
        assert!(near(a4.footer().y, 785.89) && near(a4.footer().h, 20.0));
        assert_eq!(
            a4.title(),
            Rect {
                x: 36.0,
                y: 36.0,
                w: a4.content_width(),
                h: 40.0
            }
        );
        // PR1: the print metrics. Page content width, no layout margins, the defaults
        // at 0.6: TAB string gap 7.2 pt, staff space 4.8 pt.
        assert_eq!(PRINT_STAFF_SCALE, 0.6);
        assert_eq!(a4.staff_scale, PRINT_STAFF_SCALE);
        let m = a4.metrics();
        assert_eq!((m.width, m.margin_left), (a4.content_width(), 0.0));
        // Half of Bravura's 0.5 sp thick bar line: 0.25 x 4.8 pt.
        assert!(near(m.margin_right, 1.2));
        assert!(near(m.string_spacing, 7.2) && near(m.staff_space(), 4.8));
        // System gap: Spacious by default (4 sp = 19.2 pt), Compact 0.6 x 16 = 9.6 pt.
        assert_eq!(a4.system_spacing, SystemSpacing::Spacious);
        assert!(near(m.digit_width, 4.2) && near(m.system_gap, 19.2));
        let compact = PageSetup {
            system_spacing: SystemSpacing::Compact,
            ..a4
        };
        assert!(near(compact.metrics().system_gap, 9.6));
        assert!(near(m.header_width, 16.8) && near(m.stem_length, 12.0));
        assert_eq!(
            m.duration_spacing,
            LayoutMetrics::default().duration_spacing
        );
        assert_eq!(m.validate(), Ok(()));
        // At scale 1 the lengths are the defaults.
        let one = PageSetup {
            staff_scale: 1.0,
            system_spacing: SystemSpacing::Compact,
            ..a4
        }
        .metrics();
        assert_eq!(
            one,
            LayoutMetrics {
                width: a4.content_width(),
                margin_left: 0.0,
                margin_right: 0.25 * 8.0,
                ..LayoutMetrics::default()
            }
        );
    }

    #[test]
    fn invalid_page_setups_are_rejected() {
        for s in [
            PageSetup {
                margin: 400.0,
                ..PageSetup::default()
            },
            PageSetup {
                title_height: 800.0,
                ..PageSetup::default()
            },
            PageSetup {
                footer_height: f32::NAN,
                ..PageSetup::default()
            },
            PageSetup {
                margin: -1.0,
                ..PageSetup::default()
            },
            PageSetup {
                staff_scale: 0.0,
                ..PageSetup::default()
            },
            PageSetup {
                staff_scale: f32::INFINITY,
                ..PageSetup::default()
            },
        ] {
            assert!(
                matches!(s.validate(), Err(NotationError::InvalidPage(_))),
                "{s:?}"
            );
        }
        assert_eq!(PageSetup::default().validate(), Ok(()));
    }
}
