//! Native-cell `StatusBar` chrome: a mode badge, a message, and key hints,
//! above a 1px top-border hairline. Drawn as truecolor ratatui cells, same
//! rationale as [`crate::tabs::Tabs`] — box-drawing + `Style`, never
//! rasterized.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::theme::Theme;

/// Column width the mode badge is padded/centered to, matching the ~10ch
/// pill in the React reference's `.mode-badge`.
const BADGE_WIDTH: usize = 10;

/// One status row: `[ MODE ]  message…            key label · key label  [⌃ /] shortcuts`
/// above a `─` top-border hairline in `theme.border`.
pub struct StatusBar<'a> {
    mode: &'a str,
    message: &'a str,
    keys: &'a [(&'a str, &'a str)],
    theme: &'a Theme,
}

impl<'a> StatusBar<'a> {
    /// Rows this widget wants: its top border hairline plus the content row.
    pub const HEIGHT: u16 = 2;

    pub fn new(mode: &'a str, message: &'a str, keys: &'a [(&'a str, &'a str)], theme: &'a Theme) -> Self {
        Self { mode, message, keys, theme }
    }
}

impl<'a> Widget for StatusBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let line = "\u{2500}".repeat(area.width as usize);
        buf.set_stringn(area.x, area.y, &line, area.width as usize, Style::new().fg(self.theme.border));

        if area.height < 2 {
            return;
        }
        let y = area.y + 1;
        let right_edge = area.x + area.width;

        let badge = pad_center(&self.mode.to_uppercase(), BADGE_WIDTH);
        let badge_style = Style::new()
            .fg(self.theme.app)
            .bg(self.theme.accent)
            .add_modifier(Modifier::BOLD);
        let (badge_end, _) = buf.set_stringn(area.x, y, &badge, (right_edge - area.x) as usize, badge_style);
        let message_start = (badge_end + 1).min(right_edge);

        // Key hints + the `[⌃ /] shortcuts` hint, right-aligned: built as a
        // styled `Line` (each span keeps its own color) and measured before
        // placement so the message column can be sized around it.
        let hints = build_hints_line(self.keys, self.theme);
        let hints_width = (hints.width() as u16).min(right_edge.saturating_sub(message_start));
        let hints_x = right_edge.saturating_sub(hints_width);

        if hints_x > message_start {
            let budget = (hints_x - message_start).saturating_sub(1) as usize;
            let message = truncate(self.message, budget);
            buf.set_stringn(message_start, y, &message, budget, Style::new().fg(self.theme.muted));
        }

        buf.set_line(hints_x, y, &hints, hints_width);
    }
}

/// Pad/center `s` to `width` columns (char-count — the mode strings this
/// crate renders are plain ASCII, same assumption the rest of the crate's
/// text layout makes). Longer input is hard-clipped rather than overflowing
/// the fixed-width pill.
fn pad_center(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.chars().take(width).collect();
    }
    let pad = width - len;
    let left = pad / 2;
    let right = pad - left;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
}

/// Truncate to at most `max_chars`, appending an ellipsis when cut short.
fn truncate(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

/// `key label · key label  [⌃ /] shortcuts` — keys in `theme.text`, labels
/// and separators in `theme.faint`, and the trailing `[⌃ /]` hint in
/// `theme.info` (ice) bold.
fn build_hints_line(keys: &[(&str, &str)], theme: &Theme) -> Line<'static> {
    let key_style = Style::new().fg(theme.text);
    let label_style = Style::new().fg(theme.faint);

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (key, label)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" \u{b7} ".to_string(), label_style));
        }
        spans.push(Span::styled((*key).to_string(), key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled((*label).to_string(), label_style));
    }
    if !keys.is_empty() {
        spans.push(Span::raw("  "));
    }
    spans.push(Span::styled(
        "[\u{2303} /]".to_string(),
        Style::new().fg(theme.info).add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(" shortcuts".to_string(), label_style));

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn cell_at(buf: &Buffer, x: u16, y: u16) -> &ratatui::buffer::Cell {
        buf.cell((x, y)).expect("cell in bounds")
    }

    fn row_string(buf: &Buffer, y: u16, width: u16) -> String {
        (0..width).map(|x| cell_at(buf, x, y).symbol().to_string()).collect()
    }

    #[test]
    fn top_border_row_uses_theme_border() {
        let theme = Theme::temet();
        let area = Rect::new(0, 0, 40, 2);
        let mut buf = Buffer::empty(area);
        StatusBar::new("normal", "ready", &[], &theme).render(area, &mut buf);

        let cell = cell_at(&buf, 5, 0);
        assert_eq!(cell.symbol(), "\u{2500}");
        assert_eq!(cell.fg, theme.border);
    }

    #[test]
    fn mode_badge_is_uppercase_accent_fill_bold() {
        let theme = Theme::temet();
        let area = Rect::new(0, 0, 60, 2);
        let mut buf = Buffer::empty(area);
        StatusBar::new("normal", "ready", &[], &theme).render(area, &mut buf);

        let row = row_string(&buf, 1, area.width);
        assert!(row.contains("NORMAL"));

        let cell = cell_at(&buf, 1, 1);
        assert_eq!(cell.bg, theme.accent);
        assert_eq!(cell.fg, theme.app);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn message_is_muted_and_present() {
        let theme = Theme::temet();
        let area = Rect::new(0, 0, 60, 2);
        let mut buf = Buffer::empty(area);
        StatusBar::new("normal", "loading fleet", &[], &theme).render(area, &mut buf);

        let row = row_string(&buf, 1, area.width);
        assert!(row.contains("loading fleet"));
    }

    #[test]
    fn overlong_message_is_ellipsized() {
        let theme = Theme::temet();
        // Wide enough to leave a message column, too narrow for the full text.
        let area = Rect::new(0, 0, 50, 2);
        let mut buf = Buffer::empty(area);
        let long = "this message is far too long to fit in a narrow status bar column";
        StatusBar::new("normal", long, &[("q", "quit")], &theme).render(area, &mut buf);

        let row = row_string(&buf, 1, area.width);
        assert!(row.contains('\u{2026}'), "row was: {row:?}");
        assert!(!row.contains(long), "full message must not fit verbatim");
    }

    #[test]
    fn shortcuts_hint_present_with_info_bracket() {
        let theme = Theme::temet();
        let area = Rect::new(0, 0, 60, 2);
        let mut buf = Buffer::empty(area);
        StatusBar::new("normal", "ready", &[("1-3", "tab")], &theme).render(area, &mut buf);

        let row = row_string(&buf, 1, area.width);
        assert!(row.contains("shortcuts"));
        assert!(row.contains("1-3"));
        assert!(row.contains("tab"));

        // Find the `[` of `[⌃ /]` and confirm it carries theme.info + bold.
        let bracket_x = (0..area.width).find(|&x| cell_at(&buf, x, 1).symbol() == "[").expect("bracket cell");
        let cell = cell_at(&buf, bracket_x, 1);
        assert_eq!(cell.fg, theme.info);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn never_uses_teal_or_amber() {
        let theme = Theme::temet();
        let area = Rect::new(0, 0, 60, 2);
        let mut buf = Buffer::empty(area);
        StatusBar::new("normal", "ready", &[("1-3", "tab"), ("j/k", "move")], &theme).render(area, &mut buf);

        for cell in buf.content() {
            assert_ne!(cell.fg, Color::Rgb(0x00, 0xd9, 0xc0), "teal leaked into fg");
            assert_ne!(cell.bg, Color::Rgb(0x00, 0xd9, 0xc0), "teal leaked into bg");
            assert_ne!(cell.fg, Color::Rgb(0xff, 0xbf, 0x00), "amber leaked into fg");
            assert_ne!(cell.bg, Color::Rgb(0xff, 0xbf, 0x00), "amber leaked into bg");
        }
    }
}
