//! The one universal layout (TUI-L1, `docs/tui/LAYOUT.md`).
//!
//! ```text
//! row 0        app bar   — the navigation path, edge to edge
//! rows 1..n-2  nav (content-sized) │ middle │ detail   (middle/detail split the rest equally)
//! row n-1      help bar  — navigation keys only
//! ```
//!
//! Every screen calls [`frame`] and renders into the five rectangles it hands
//! back. There is no per-screen layout choice; what varies is the nav width
//! (which is content-sized, so the caller supplies it — see [`nav_width_for`])
//! and the panels' contents.
//!
//! ## Narrow real estate
//!
//! LAYOUT.md reflows on *real estate*, never on a magic width constant, so the
//! decision here is a fits-check in cells: three columns survive only while
//! the nav's own width plus both panels' minimums fit across, and the content
//! region is tall enough for a panel. When they do not, detail stacks below
//! middle; when even the stack does not fit, [`FrameMode::Modal`] tells the
//! caller to render the detail panel as a modal over the screen instead.
//! [`fits`] answers that question on its own for a caller that needs to decide
//! before laying anything out.

pub mod app_bar;

use ratatui::layout::Rect;

pub use app_bar::AppBar;

/// Rows the app bar occupies. Always one, at every size.
pub const APP_BAR_HEIGHT: u16 = 1;
/// Rows the help bar occupies. Always one, at every size.
pub const HELP_BAR_HEIGHT: u16 = 1;

/// How the content region resolved for a given area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameMode {
    /// Three panels left to right: nav │ middle │ detail.
    Columns,
    /// Detail stacked below middle, both to the right of nav.
    Stacked,
    /// No room to stack: nav │ middle only, and the detail rectangle is a
    /// modal the caller opens over the screen when the user navigates to it.
    Modal,
}

/// The measurements a screen contributes to the universal layout.
///
/// Only `nav_width` is genuinely per-screen — the minimums exist so the
/// reflow decision is made in cells rather than against a hardcoded width, and
/// a screen with unusually wide rows can raise them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSpec {
    /// The nav panel's content-sized width, borders included. "nav is never
    /// wide" (LAYOUT.md) — size it to its longest item, not to the screen.
    pub nav_width: u16,
    /// Fewest columns the middle panel can usefully occupy, borders included.
    pub min_middle_width: u16,
    /// Fewest columns the detail panel can usefully occupy, borders included.
    pub min_detail_width: u16,
    /// Fewest rows a panel can usefully occupy, borders included.
    pub min_panel_height: u16,
}

impl FrameSpec {
    /// Two borders plus enough content for a readable row.
    pub const DEFAULT_MIN_PANEL_WIDTH: u16 = 20;
    /// Two borders plus three content rows.
    pub const DEFAULT_MIN_PANEL_HEIGHT: u16 = 5;
    /// The nav width used when a caller has not measured its own items.
    pub const DEFAULT_NAV_WIDTH: u16 = 14;

    /// A spec with an explicit nav width and the default minimums.
    #[must_use]
    pub const fn new(nav_width: u16) -> Self {
        Self {
            nav_width,
            min_middle_width: Self::DEFAULT_MIN_PANEL_WIDTH,
            min_detail_width: Self::DEFAULT_MIN_PANEL_WIDTH,
            min_panel_height: Self::DEFAULT_MIN_PANEL_HEIGHT,
        }
    }

    /// Override the minimum panel widths.
    #[must_use]
    pub const fn with_min_widths(mut self, middle: u16, detail: u16) -> Self {
        self.min_middle_width = middle;
        self.min_detail_width = detail;
        self
    }

    /// Override the minimum panel height.
    #[must_use]
    pub const fn with_min_panel_height(mut self, height: u16) -> Self {
        self.min_panel_height = height;
        self
    }
}

impl Default for FrameSpec {
    fn default() -> Self {
        Self::new(Self::DEFAULT_NAV_WIDTH)
    }
}

/// The five rectangles every screen renders into.
///
/// Positions are fixed for a given area and mode: panels never move, swap, or
/// collapse in response to data (TUI-P1). An empty panel keeps its rectangle
/// and states its empty state inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    /// Row 0, full width: the navigation path.
    pub app_bar: Rect,
    /// The content-sized navigation panel.
    pub nav: Rect,
    /// The middle panel.
    pub middle: Rect,
    /// The detail/preview panel. In [`FrameMode::Modal`] this is the modal
    /// rectangle, not a column, and is only drawn when the user opens it.
    pub detail: Rect,
    /// The last row, full width: navigation keys only.
    pub help_bar: Rect,
    /// How the content region resolved.
    pub mode: FrameMode,
}

impl Frame {
    /// Whether `detail` must be rendered as a modal over the other panels
    /// rather than as a panel beside or below them.
    #[must_use]
    pub const fn detail_is_modal(&self) -> bool {
        matches!(self.mode, FrameMode::Modal)
    }

    /// The whole content region — everything between the two bars.
    #[must_use]
    pub const fn content(&self) -> Rect {
        let rest = match self.mode {
            // Three columns tile the row, so the detail column counts too.
            FrameMode::Columns => self.middle.width + self.detail.width,
            // Stacked and modal both leave one column to the right of nav.
            FrameMode::Stacked | FrameMode::Modal => self.middle.width,
        };
        Rect {
            x: self.nav.x,
            y: self.nav.y,
            width: self.nav.width + rest,
            height: self.nav.height,
        }
    }
}

/// Which layout the area can carry, without computing the rectangles.
///
/// A caller that must decide whether a panel is a column or a modal *before*
/// laying out (for example to choose a key handler) asks this.
#[must_use]
pub fn fits(area: Rect, spec: FrameSpec) -> FrameMode {
    let content = content_region(area);
    if content.width == 0 || content.height == 0 {
        return FrameMode::Modal;
    }
    let nav_width = nav_width_in(content.width, spec);
    let rest = content.width.saturating_sub(nav_width);

    let columns_fit = nav_width == spec.nav_width
        && rest >= spec.min_middle_width.saturating_add(spec.min_detail_width)
        && content.height >= spec.min_panel_height;
    if columns_fit {
        return FrameMode::Columns;
    }

    let stack_fits =
        rest >= spec.min_middle_width && content.height >= spec.min_panel_height.saturating_mul(2);
    if stack_fits {
        return FrameMode::Stacked;
    }

    FrameMode::Modal
}

/// Split `area` into the universal layout.
///
/// Never panics and never returns a rectangle outside `area`. Degenerate
/// areas degrade rather than fail: at one row only the app bar exists (you
/// need to know where you are before you need to know how to leave), at two
/// rows both bars exist and the content region is empty, and below the width
/// where nav plus a usable middle fit, nav yields its columns to middle.
#[must_use]
pub fn frame(area: Rect, spec: FrameSpec) -> Frame {
    if area.width == 0 || area.height == 0 {
        let empty = Rect::new(area.x, area.y, 0, 0);
        return Frame {
            app_bar: empty,
            nav: empty,
            middle: empty,
            detail: empty,
            help_bar: empty,
            mode: FrameMode::Modal,
        };
    }

    let app_bar = Rect {
        height: APP_BAR_HEIGHT.min(area.height),
        ..area
    };
    let help_bar = if area.height >= 2 {
        Rect {
            y: area.y + area.height - HELP_BAR_HEIGHT,
            height: HELP_BAR_HEIGHT,
            ..area
        }
    } else {
        Rect::new(area.x, area.y + area.height, area.width, 0)
    };

    let content = content_region(area);
    let mode = fits(area, spec);

    if content.width == 0 || content.height == 0 {
        let empty = Rect::new(content.x, content.y, 0, 0);
        return Frame {
            app_bar,
            nav: empty,
            middle: empty,
            detail: empty,
            help_bar,
            mode,
        };
    }

    let nav_width = nav_width_in(content.width, spec);
    let nav = Rect {
        width: nav_width,
        ..content
    };
    let rest = Rect {
        x: content.x + nav_width,
        width: content.width - nav_width,
        ..content
    };

    let (middle, detail) = match mode {
        FrameMode::Columns => {
            // The extra column when `rest` is odd goes to middle, matching
            // ratatui's own `Fill(1)` / `Fill(1)` bias.
            let middle_width = rest.width - rest.width / 2;
            (
                Rect {
                    width: middle_width,
                    ..rest
                },
                Rect {
                    x: rest.x + middle_width,
                    width: rest.width - middle_width,
                    ..rest
                },
            )
        }
        FrameMode::Stacked => {
            let middle_height = rest.height - rest.height / 2;
            (
                Rect {
                    height: middle_height,
                    ..rest
                },
                Rect {
                    y: rest.y + middle_height,
                    height: rest.height - middle_height,
                    ..rest
                },
            )
        }
        FrameMode::Modal => (rest, modal_rect(rest)),
    };

    Frame {
        app_bar,
        nav,
        middle,
        detail,
        help_bar,
        mode,
    }
}

/// A content-sized nav width for a list of item names: the longest name, plus
/// one column of padding either side, plus the two borders.
///
/// "nav is never wide. It is sized to its content and borders" (LAYOUT.md).
/// Width is counted in `char`s, which is exact for the ASCII item names this
/// layout is designed around; a caller with wide characters should measure
/// with `unicode-width` and pass the result to [`FrameSpec::new`] directly.
#[must_use]
pub fn nav_width_for<'a>(items: impl IntoIterator<Item = &'a str>) -> u16 {
    let longest = items
        .into_iter()
        .map(|item| u16::try_from(item.chars().count()).unwrap_or(u16::MAX))
        .max()
        .unwrap_or(0);
    longest
        .saturating_add(4)
        .max(FrameSpec::DEFAULT_NAV_WIDTH / 2)
}

/// The rows between the two bars.
fn content_region(area: Rect) -> Rect {
    let chrome = APP_BAR_HEIGHT.saturating_add(HELP_BAR_HEIGHT);
    let height = area.height.saturating_sub(chrome);
    Rect {
        y: area.y + APP_BAR_HEIGHT.min(area.height),
        height,
        ..area
    }
}

/// Nav's width inside a content region of `width` columns: its requested
/// width, but never so wide that middle drops below its minimum.
fn nav_width_in(width: u16, spec: FrameSpec) -> u16 {
    spec.nav_width
        .min(width.saturating_sub(spec.min_middle_width))
}

/// The modal rectangle: the content region inset by one row and two columns
/// where there is room for the inset, otherwise the content region itself.
fn modal_rect(content: Rect) -> Rect {
    let dx = if content.width >= 8 { 2 } else { 0 };
    let dy = if content.height >= 6 { 1 } else { 0 };
    Rect {
        x: content.x + dx,
        y: content.y + dy,
        width: content.width - dx * 2,
        height: content.height - dy * 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every rectangle sits inside `area` and the three regions tile its rows.
    fn assert_within(area: Rect, f: &Frame) {
        for (name, r) in [
            ("app_bar", f.app_bar),
            ("nav", f.nav),
            ("middle", f.middle),
            ("detail", f.detail),
            ("help_bar", f.help_bar),
        ] {
            assert!(
                r.x >= area.x && r.y >= area.y,
                "{name} starts before the area: {r:?} vs {area:?}"
            );
            assert!(
                r.x + r.width <= area.x + area.width,
                "{name} overflows right: {r:?} vs {area:?}"
            );
            assert!(
                r.y + r.height <= area.y + area.height,
                "{name} overflows bottom: {r:?} vs {area:?}"
            );
        }
    }

    #[test]
    fn floor_80x24_is_three_columns() {
        let area = Rect::new(0, 0, 80, 24);
        let f = frame(area, FrameSpec::default());
        assert_eq!(f.mode, FrameMode::Columns);

        assert_eq!(f.app_bar, Rect::new(0, 0, 80, 1));
        assert_eq!(f.help_bar, Rect::new(0, 23, 80, 1));
        assert_eq!(f.nav, Rect::new(0, 1, 14, 22));
        assert_eq!(f.middle, Rect::new(14, 1, 33, 22));
        assert_eq!(f.detail, Rect::new(47, 1, 33, 22));

        assert_eq!(f.nav.width + f.middle.width + f.detail.width, 80);
        assert_eq!(
            f.middle.width, f.detail.width,
            "middle/detail split equally"
        );
        assert!(!f.detail_is_modal());
        assert_within(area, &f);
    }

    #[test]
    fn wide_120x40_keeps_nav_width_and_grows_the_other_two() {
        let area = Rect::new(0, 0, 120, 40);
        let f = frame(area, FrameSpec::default());
        assert_eq!(f.mode, FrameMode::Columns);

        assert_eq!(f.app_bar, Rect::new(0, 0, 120, 1));
        assert_eq!(f.help_bar, Rect::new(0, 39, 120, 1));
        assert_eq!(
            f.nav,
            Rect::new(0, 1, 14, 38),
            "nav keeps its content width"
        );
        assert_eq!(f.middle, Rect::new(14, 1, 53, 38));
        assert_eq!(f.detail, Rect::new(67, 1, 53, 38));
        assert_eq!(f.middle.width, f.detail.width);
        assert_within(area, &f);
    }

    #[test]
    fn extra_rows_go_to_the_content_region_only() {
        let short = frame(Rect::new(0, 0, 100, 24), FrameSpec::default());
        let tall = frame(Rect::new(0, 0, 100, 60), FrameSpec::default());
        assert_eq!(short.app_bar.height, tall.app_bar.height);
        assert_eq!(short.help_bar.height, tall.help_bar.height);
        assert_eq!(tall.middle.height - short.middle.height, 36);
        assert_eq!(short.middle.width, tall.middle.width);
    }

    #[test]
    fn narrow_width_stacks_detail_below_middle() {
        // 14 nav + 20 middle fits, but 14 + 20 + 20 does not.
        let area = Rect::new(0, 0, 44, 24);
        assert_eq!(fits(area, FrameSpec::default()), FrameMode::Stacked);

        let f = frame(area, FrameSpec::default());
        assert_eq!(f.mode, FrameMode::Stacked);
        assert_eq!(f.nav, Rect::new(0, 1, 14, 22));
        assert_eq!(f.middle, Rect::new(14, 1, 30, 11));
        assert_eq!(f.detail, Rect::new(14, 12, 30, 11));
        assert_eq!(
            f.middle.width, f.detail.width,
            "each keeps its full usable width"
        );
        assert_eq!(
            f.detail.y,
            f.middle.y + f.middle.height,
            "no gap, no overlap"
        );
        assert_within(area, &f);
    }

    #[test]
    fn short_content_cannot_stack_so_detail_becomes_a_modal() {
        // Wide enough for one column, far too short for two stacked panels.
        let area = Rect::new(0, 0, 44, 8);
        assert_eq!(fits(area, FrameSpec::default()), FrameMode::Modal);

        let f = frame(area, FrameSpec::default());
        assert!(f.detail_is_modal());
        assert_eq!(f.nav, Rect::new(0, 1, 14, 6));
        assert_eq!(f.middle, Rect::new(14, 1, 30, 6));
        assert!(f.detail.width > 0 && f.detail.height > 0);
        assert_within(area, &f);
    }

    #[test]
    fn the_reflow_is_decided_by_cells_not_by_a_width_constant() {
        // Same 44 columns, but a screen whose panels need less room keeps
        // three columns — proving the decision reads the spec, not the width.
        let area = Rect::new(0, 0, 44, 24);
        let tight = FrameSpec::new(8).with_min_widths(16, 16);
        assert_eq!(fits(area, tight), FrameMode::Columns);
        assert_eq!(fits(area, FrameSpec::default()), FrameMode::Stacked);
    }

    #[test]
    fn nav_yields_columns_before_middle_drops_below_its_minimum() {
        let area = Rect::new(0, 0, 30, 24);
        let f = frame(area, FrameSpec::default());
        assert_eq!(f.nav.width, 10, "nav gave up 4 columns so middle keeps 20");
        assert_eq!(f.middle.width, 20);
        assert_within(area, &f);
    }

    #[test]
    fn pathological_sizes_do_not_panic_and_stay_inside_the_area() {
        for (w, h) in [
            (0, 0),
            (1, 1),
            (1, 24),
            (80, 1),
            (80, 2),
            (80, 3),
            (2, 2),
            (5, 5),
            (20, 4),
            (u16::MAX, 1),
        ] {
            let area = Rect::new(0, 0, w, h);
            let f = frame(area, FrameSpec::default());
            assert_within(area, &f);
            let _ = fits(area, FrameSpec::default());
        }
    }

    #[test]
    fn one_row_keeps_the_app_bar_and_two_rows_keep_both_bars() {
        let one = frame(Rect::new(0, 0, 80, 1), FrameSpec::default());
        assert_eq!(one.app_bar, Rect::new(0, 0, 80, 1));
        assert_eq!(one.help_bar.height, 0);
        assert_eq!(one.nav.height, 0);

        let two = frame(Rect::new(0, 0, 80, 2), FrameSpec::default());
        assert_eq!(two.app_bar, Rect::new(0, 0, 80, 1));
        assert_eq!(two.help_bar, Rect::new(0, 1, 80, 1));
        assert_eq!(two.middle.height, 0);
    }

    #[test]
    fn a_frame_is_offset_correctly_inside_a_non_zero_origin_area() {
        let area = Rect::new(7, 3, 80, 24);
        let f = frame(area, FrameSpec::default());
        assert_eq!(f.app_bar, Rect::new(7, 3, 80, 1));
        assert_eq!(f.nav, Rect::new(7, 4, 14, 22));
        assert_eq!(f.help_bar, Rect::new(7, 26, 80, 1));
        assert_within(area, &f);
    }

    #[test]
    fn nav_width_is_sized_to_its_longest_item() {
        assert_eq!(
            nav_width_for(["services", "config", "setup", "sessions"]),
            12
        );
        assert_eq!(nav_width_for(["a"]), 7);
        assert_eq!(nav_width_for(std::iter::empty::<&str>()), 7);
    }

    #[test]
    fn content_reports_the_whole_region_in_every_mode() {
        for (w, h) in [(80, 24), (44, 24), (44, 8)] {
            let area = Rect::new(0, 0, w, h);
            let f = frame(area, FrameSpec::default());
            let content = f.content();
            assert_eq!(content.width, w, "{w}x{h}");
            assert_eq!(content.height, h - 2, "{w}x{h}");
            assert_eq!(content.y, 1);
        }
    }
}
