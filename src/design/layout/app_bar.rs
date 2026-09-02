//! The app bar: row 0, edge to edge, the navigation path and nothing else.
//!
//! "The app bar always shows where the user is in the navigation, readable at
//! a glance, as a path from the app root" (LAYOUT.md). It never scrolls and
//! never grows, and it carries no status, no counts and no clock — those
//! belong inside a panel.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::design::theme::{DesignTheme, Slot};

/// The separator between path segments. `·` is the documented inline
/// separator (TUI-G1); `/` reads as a path and is what LAYOUT.md's own
/// examples use, so the bar spells the path with slashes.
const SEPARATOR: &str = " / ";

/// One row showing the navigation path: `mike / sessions / a1b2c3d4`.
///
/// Segments are borrowed, so a caller builds the path per frame without
/// allocating a joined string.
#[derive(Debug, Clone, Copy)]
pub struct AppBar<'a> {
    path: &'a [&'a str],
    theme: &'a DesignTheme,
}

impl<'a> AppBar<'a> {
    /// Rows the app bar occupies. Always one.
    pub const HEIGHT: u16 = 1;

    /// Build an app bar for a path from the app root, root segment first.
    #[must_use]
    pub const fn new(path: &'a [&'a str], theme: &'a DesignTheme) -> Self {
        Self { path, theme }
    }

    /// The styled path line: leading segments and separators muted, the
    /// current (last) segment in `text.emphasis` so the eye lands on where the
    /// user actually is.
    ///
    /// This is a tui-kit-level styling choice — LAYOUT.md fixes the app bar's
    /// content and geometry, not its emphasis.
    #[must_use]
    pub fn line(&self) -> Line<'a> {
        let muted = self.theme.style(Slot::TextMuted);
        let current = self.theme.style(Slot::TextEmphasis);

        let mut spans: Vec<Span<'a>> = Vec::with_capacity(self.path.len() * 2);
        let last = self.path.len().saturating_sub(1);
        for (i, segment) in self.path.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(SEPARATOR, muted));
            }
            let style = if i == last { current } else { muted };
            spans.push(Span::styled(*segment, style));
        }
        Line::from(spans)
    }

    /// The path trimmed from the left until it fits `width` columns.
    ///
    /// Overflow drops whole leading segments rather than eliding characters —
    /// the deepest, most specific part of the path is what the user needs, and
    /// TUI-G4 rules out `…`. If even the last segment is too long it is hard
    /// clipped by the renderer.
    #[must_use]
    pub fn line_for_width(&self, width: u16) -> Line<'a> {
        let width = width as usize;
        let mut start = 0;
        while start < self.path.len() {
            let candidate = Self {
                path: &self.path[start..],
                theme: self.theme,
            }
            .line();
            if candidate.width() <= width || start + 1 == self.path.len() {
                return candidate;
            }
            start += 1;
        }
        Line::default()
    }
}

impl Widget for AppBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let row = Rect {
            height: Self::HEIGHT,
            ..area
        };

        // Edge to edge: the bar's surface is painted across the full width
        // even where the path does not reach.
        let surface = self.theme.bg_style(Slot::BgSurface);
        if surface != Style::new() {
            buf.set_style(row, surface);
        }

        let line = self.line_for_width(row.width);
        buf.set_line(row.x, row.y, &line, row.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::theme::Tier;

    fn render(path: &[&str], theme: &DesignTheme, width: u16) -> String {
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);
        AppBar::new(path, theme).render(area, &mut buf);
        (0..width)
            .map(|x| buf[(x, 0)].symbol())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn renders_the_path_and_nothing_else() {
        let theme = DesignTheme::with_tier(Tier::TrueColor);
        assert_eq!(
            render(&["mike", "sessions", "a1b2c3d4"], &theme, 40),
            "mike / sessions / a1b2c3d4"
        );
        assert_eq!(render(&["mike"], &theme, 40), "mike");
        assert_eq!(render(&[], &theme, 40), "");
    }

    #[test]
    fn overflow_drops_leading_segments_never_elides() {
        let theme = DesignTheme::with_tier(Tier::TrueColor);
        let path = ["mike", "sessions", "a1b2c3d4", "request 138469"];
        let full = render(&path, &theme, 60);
        assert_eq!(full, "mike / sessions / a1b2c3d4 / request 138469");

        let narrow = render(&path, &theme, 30);
        assert!(
            narrow.ends_with("request 138469"),
            "kept the tail: {narrow}"
        );
        assert!(!narrow.contains('\u{2026}'), "elided: {narrow}");
        assert!(narrow.chars().count() <= 30, "overflowed: {narrow}");
    }

    #[test]
    fn the_last_segment_is_emphasised_and_the_rest_muted() {
        let theme = DesignTheme::with_tier(Tier::TrueColor);
        let line = AppBar::new(&["mike", "config"], &theme).line();
        assert_eq!(line.spans[0].style, theme.style(Slot::TextMuted));
        assert_eq!(line.spans[1].style, theme.style(Slot::TextMuted));
        assert_eq!(line.spans[2].style, theme.style(Slot::TextEmphasis));
    }

    #[test]
    fn monochrome_app_bar_carries_no_color() {
        let theme = DesignTheme::with_tier(Tier::Mono);
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        AppBar::new(&["mike", "sessions"], &theme).render(area, &mut buf);
        for x in 0..area.width {
            let cell = &buf[(x, 0)];
            assert_eq!(cell.fg, ratatui::style::Color::Reset, "fg at {x}");
            assert_eq!(cell.bg, ratatui::style::Color::Reset, "bg at {x}");
        }
    }

    #[test]
    fn zero_sized_area_renders_nothing() {
        let theme = DesignTheme::with_tier(Tier::TrueColor);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        AppBar::new(&["mike"], &theme).render(Rect::new(0, 0, 0, 0), &mut buf);
        assert_eq!(buf[(0, 0)].symbol(), " ");
    }
}
