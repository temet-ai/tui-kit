//! Native-cell `Tabs` chrome: a horizontal row of `"{n}:{TITLE}"` labels with
//! a 1px bottom border. Drawn as truecolor ratatui cells (box-drawing +
//! `Style::fg`/`bg`) rather than rasterized SVG, so it composes cheaply with
//! the image-based [`crate::widget::CardGrid`] in the same frame.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::theme::Theme;

/// A row of tabs, each `"{n}:{TITLE}"` (1-based index, uppercase title). The
/// active tab fills with `theme.accent`/`theme.app`, bold; the rest use
/// `theme.muted`. Carries no state of its own — `active` is passed in fresh
/// each render, same as `CardGrid` takes its selection via `NavState`.
pub struct Tabs<'a> {
    titles: &'a [&'a str],
    active: usize,
    theme: &'a Theme,
}

impl<'a> Tabs<'a> {
    /// Rows this widget wants: the label row plus its bottom border
    /// hairline. Render into a 1-row area to suppress the border (e.g. when
    /// a caller draws a shared, wider border line itself).
    pub const HEIGHT: u16 = 2;

    pub fn new(titles: &'a [&'a str], active: usize, theme: &'a Theme) -> Self {
        Self {
            titles,
            active,
            theme,
        }
    }
}

impl<'a> Widget for Tabs<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let right_edge = area.x + area.width;
        let mut x = area.x;
        for (i, title) in self.titles.iter().enumerate() {
            if x >= right_edge {
                break;
            }
            let label = format!(" {}:{} ", i + 1, title.to_uppercase());
            let style = if i == self.active {
                Style::new()
                    .fg(self.theme.app)
                    .bg(self.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(self.theme.muted)
            };
            let max_width = (right_edge - x) as usize;
            let (next_x, _) = buf.set_stringn(x, area.y, &label, max_width, style);
            x = next_x;
        }

        if area.height >= 2 {
            let line = "\u{2500}".repeat(area.width as usize);
            buf.set_stringn(
                area.x,
                area.y + 1,
                &line,
                area.width as usize,
                Style::new().fg(self.theme.border),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn cell_at(buf: &Buffer, x: u16, y: u16) -> &ratatui::buffer::Cell {
        buf.cell((x, y)).expect("cell in bounds")
    }

    #[test]
    fn selected_tab_uses_accent_fill_bg0_text_bold() {
        let theme = Theme::temet();
        let titles = ["proc", "pods", "api"];
        let area = Rect::new(0, 0, 40, 2);
        let mut buf = Buffer::empty(area);
        Tabs::new(&titles, 0, &theme).render(area, &mut buf);

        // " 1:PROC " starts at x=0; sample a cell inside that span.
        let cell = cell_at(&buf, 1, 0);
        assert_eq!(cell.bg, theme.accent);
        assert_eq!(cell.fg, theme.app);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn unselected_tab_uses_muted_no_fill() {
        let theme = Theme::temet();
        let titles = ["proc", "pods", "api"];
        let area = Rect::new(0, 0, 40, 2);
        let mut buf = Buffer::empty(area);
        Tabs::new(&titles, 0, &theme).render(area, &mut buf);

        // " 1:PROC " is 8 cells wide, so " 2:PODS " starts at x=8.
        let cell = cell_at(&buf, 9, 0);
        assert_eq!(cell.fg, theme.muted);
        assert_ne!(cell.bg, theme.accent);
    }

    #[test]
    fn titles_render_uppercase() {
        let theme = Theme::temet();
        let titles = ["proc"];
        let area = Rect::new(0, 0, 20, 2);
        let mut buf = Buffer::empty(area);
        Tabs::new(&titles, 0, &theme).render(area, &mut buf);

        let row: String = (0..area.width)
            .map(|x| cell_at(&buf, x, 0).symbol().to_string())
            .collect();
        assert!(row.contains("PROC"));
        assert!(!row.contains("proc"));
    }

    #[test]
    fn bottom_border_row_uses_theme_border() {
        let theme = Theme::temet();
        let titles = ["proc"];
        let area = Rect::new(0, 0, 10, 2);
        let mut buf = Buffer::empty(area);
        Tabs::new(&titles, 0, &theme).render(area, &mut buf);

        let cell = cell_at(&buf, 3, 1);
        assert_eq!(cell.symbol(), "\u{2500}");
        assert_eq!(cell.fg, theme.border);
    }

    #[test]
    fn height_one_suppresses_border_row() {
        let theme = Theme::temet();
        let titles = ["proc"];
        // A 1-row area: only the label row's buffer cells exist, nothing to
        // sample at y=1 without going out of bounds — this just proves
        // `render` doesn't panic when given height 1 (the header composition
        // relies on this to draw its own shared border instead).
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        Tabs::new(&titles, 0, &theme).render(area, &mut buf);
        let row: String = (0..area.width)
            .map(|x| cell_at(&buf, x, 0).symbol().to_string())
            .collect();
        assert!(row.contains("PROC"));
    }

    #[test]
    fn never_uses_teal_or_amber() {
        let theme = Theme::temet();
        let titles = ["proc", "pods", "api"];
        let area = Rect::new(0, 0, 40, 2);
        let mut buf = Buffer::empty(area);
        Tabs::new(&titles, 0, &theme).render(area, &mut buf);

        for cell in buf.content() {
            assert_ne!(cell.fg, Color::Rgb(0x00, 0xd9, 0xc0), "teal leaked into fg");
            assert_ne!(cell.bg, Color::Rgb(0x00, 0xd9, 0xc0), "teal leaked into bg");
            assert_ne!(
                cell.fg,
                Color::Rgb(0xff, 0xbf, 0x00),
                "amber leaked into fg"
            );
            assert_ne!(
                cell.bg,
                Color::Rgb(0xff, 0xbf, 0x00),
                "amber leaked into bg"
            );
        }
    }
}
