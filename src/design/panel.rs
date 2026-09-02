//! Bordered panels, titled by the item they are showing.
//!
//! LAYOUT.md fixes four things about a panel and this module implements
//! exactly those: it is a rectangle of whole cells with a box-drawn border one
//! cell thick; its name sits in the top border; the name is the name of the
//! **item** it is showing, not the panel's role; and one letter of that name
//! is highlighted as the panel's jump shortcut.
//!
//! Everything else here — lowercasing, the `[⌃s]` hint, which slot the name
//! itself takes — is a tui-kit-level choice, documented as such. Panel
//! *internals* (rows, padding, banding) are a reserved design phase and this
//! module deliberately says nothing about them.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Widget};

use crate::design::theme::{DesignTheme, Slot};

/// A bordered panel's chrome: the border and the title, no contents.
///
/// Render it, then draw into [`Panel::inner`]. Or take [`Panel::block`] and
/// hand it to any ratatui widget that accepts a `Block`.
#[derive(Debug, Clone)]
pub struct Panel<'a> {
    name: String,
    shortcut: usize,
    focused: bool,
    jump_hint: bool,
    theme: &'a DesignTheme,
}

impl<'a> Panel<'a> {
    /// A panel titled by the item it shows.
    ///
    /// The name is lowercased — one casing per screen keeps the frame quiet.
    /// The shortcut defaults to the first character; call [`Panel::shortcut`]
    /// or [`Panel::shortcut_index`] to designate another.
    #[must_use]
    pub fn new(name: &str, theme: &'a DesignTheme) -> Self {
        Self {
            name: name.to_lowercase(),
            shortcut: 0,
            focused: false,
            jump_hint: true,
            theme,
        }
    }

    /// Designate which letter of the name is the jump shortcut.
    ///
    /// The letter must belong to the name (LAYOUT.md). A letter that does not
    /// occur is ignored and the first character stays the shortcut — the title
    /// can never advertise a key that is not in the name it is drawn from.
    /// Matching is case-insensitive, since the name is lowercased.
    #[must_use]
    pub fn shortcut(mut self, letter: char) -> Self {
        let letter = letter.to_ascii_lowercase();
        if let Some(index) = self.name.chars().position(|c| c == letter) {
            self.shortcut = index;
        } else {
            debug_assert!(
                false,
                "shortcut '{letter}' is not a letter of panel name '{}'",
                self.name
            );
        }
        self
    }

    /// Designate the shortcut by character index. Out-of-range indices are
    /// clamped to the last character.
    #[must_use]
    pub fn shortcut_index(mut self, index: usize) -> Self {
        let len = self.name.chars().count();
        self.shortcut = index.min(len.saturating_sub(1));
        self
    }

    /// Mark the panel focused. The focused panel's border is `accent.primary`;
    /// every other panel's is `accent.muted` (the non-focused accent
    /// structure).
    #[must_use]
    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Show or hide the `[⌃s]` jump hint after the name. On by default; a
    /// nested panel inside an already-jumpable panel may want it off.
    #[must_use]
    pub const fn jump_hint(mut self, show: bool) -> Self {
        self.jump_hint = show;
        self
    }

    /// The lowercased item name this panel is titled by.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The letter that is highlighted in the title and jumps to this panel.
    #[must_use]
    pub fn shortcut_char(&self) -> Option<char> {
        self.name.chars().nth(self.shortcut)
    }

    /// The `Ctrl+<letter>` key this panel answers to.
    #[must_use]
    pub fn jump_key(&self) -> Option<char> {
        self.shortcut_char()
    }

    /// The title line: the name with its shortcut letter highlighted, then the
    /// jump hint.
    ///
    /// The highlighted letter takes `accent.primary` plus `UNDERLINED`, so the
    /// designation survives the monochrome tier where the accent color is
    /// gone (TUI-P4).
    #[must_use]
    pub fn title(&self) -> Line<'static> {
        let name_style = if self.focused {
            self.theme.style(Slot::TextEmphasis)
        } else {
            self.theme.style(Slot::TextPrimary)
        };
        let letter_style = self
            .theme
            .style(Slot::AccentPrimary)
            .add_modifier(Modifier::UNDERLINED);

        let mut spans: Vec<Span<'static>> = Vec::with_capacity(6);
        spans.push(Span::styled(" ", name_style));

        let chars: Vec<char> = self.name.chars().collect();
        let before: String = chars[..self.shortcut.min(chars.len())].iter().collect();
        if !before.is_empty() {
            spans.push(Span::styled(before, name_style));
        }
        if let Some(letter) = chars.get(self.shortcut) {
            spans.push(Span::styled(letter.to_string(), letter_style));
        }
        let after: String = chars
            .get(self.shortcut.saturating_add(1)..)
            .unwrap_or_default()
            .iter()
            .collect();
        if !after.is_empty() {
            spans.push(Span::styled(after, name_style));
        }

        if self.jump_hint {
            if let Some(letter) = self.shortcut_char() {
                spans.push(Span::styled(
                    format!(" [\u{2303}{letter}]"),
                    self.theme.style(Slot::TextMuted),
                ));
            }
        }
        spans.push(Span::styled(" ", name_style));
        Line::from(spans)
    }

    /// The border style: `accent.primary` when focused, `accent.muted`
    /// otherwise. Focus is also carried by `BOLD`, so it survives monochrome.
    #[must_use]
    pub fn border_style(&self) -> Style {
        if self.focused {
            self.theme
                .style(Slot::AccentPrimary)
                .add_modifier(Modifier::BOLD)
        } else {
            self.theme.style(Slot::AccentMuted)
        }
    }

    /// The ratatui `Block` this panel is: square box-drawing borders (TUI-G5
    /// bans rounded chrome), the title in the top border, `bg.surface` inside.
    #[must_use]
    pub fn block(&self) -> Block<'static> {
        Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(self.border_style())
            .style(self.theme.bg_style(Slot::BgSurface))
            .title(self.title())
    }

    /// The area inside the border, where the panel's content goes.
    #[must_use]
    pub fn inner(&self, area: Rect) -> Rect {
        self.block().inner(area)
    }
}

impl Widget for Panel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.block().render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::theme::Tier;
    use ratatui::style::Color;

    fn theme() -> DesignTheme {
        DesignTheme::with_tier(Tier::TrueColor)
    }

    fn title_text(panel: &Panel<'_>) -> String {
        panel
            .title()
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect()
    }

    #[test]
    fn the_title_is_the_lowercased_item_name_plus_the_jump_hint() {
        let theme = theme();
        let panel = Panel::new("Sessions", &theme);
        assert_eq!(panel.name(), "sessions");
        assert_eq!(title_text(&panel), " sessions [\u{2303}s] ");
    }

    #[test]
    fn exactly_one_letter_of_the_name_is_highlighted() {
        let theme = theme();
        let panel = Panel::new("config", &theme).shortcut('n');
        assert_eq!(panel.shortcut_char(), Some('n'));

        let accent = theme
            .style(Slot::AccentPrimary)
            .add_modifier(Modifier::UNDERLINED);
        let title = panel.title();
        let highlighted: Vec<&str> = title
            .spans
            .iter()
            .filter(|s| s.style == accent)
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(highlighted, vec!["n"], "one letter, and only one");
        assert_eq!(title_text(&panel), " config [\u{2303}n] ");
    }

    #[test]
    fn the_shortcut_letter_must_belong_to_the_name() {
        let theme = theme();
        // `shortcut` debug-asserts on a letter outside the name, so exercise
        // the release-mode fallback through the index API instead.
        let panel = Panel::new("stack", &theme).shortcut_index(99);
        assert_eq!(panel.shortcut_char(), Some('k'), "clamped into the name");
        assert!(panel.name().contains(panel.shortcut_char().unwrap()));
    }

    #[test]
    fn the_first_letter_is_the_default_shortcut() {
        let theme = theme();
        let panel = Panel::new("log", &theme);
        assert_eq!(panel.shortcut_char(), Some('l'));
        assert_eq!(panel.jump_key(), Some('l'));
    }

    #[test]
    fn a_repeated_letter_takes_its_first_position() {
        let theme = theme();
        let panel = Panel::new("sessions", &theme).shortcut('s');
        assert_eq!(panel.shortcut_char(), Some('s'));
        let accent = theme
            .style(Slot::AccentPrimary)
            .add_modifier(Modifier::UNDERLINED);
        // The highlighted span is the leading `s`, so nothing precedes it.
        let idx = panel
            .title()
            .spans
            .iter()
            .position(|s| s.style == accent)
            .expect("a highlighted span");
        assert_eq!(idx, 1, "span 0 is the leading pad");
    }

    #[test]
    fn a_shortcut_in_the_middle_splits_the_name_into_three_spans() {
        let theme = theme();
        let panel = Panel::new("request 138469", &theme).shortcut('q');
        assert_eq!(title_text(&panel), " request 138469 [\u{2303}q] ");
        assert_eq!(panel.shortcut_char(), Some('q'));
    }

    #[test]
    fn the_jump_hint_can_be_suppressed() {
        let theme = theme();
        let panel = Panel::new("log", &theme).jump_hint(false);
        assert_eq!(title_text(&panel), " log ");
    }

    #[test]
    fn focus_changes_the_border_slot_not_the_geometry() {
        let theme = theme();
        let unfocused = Panel::new("stack", &theme);
        let focused = Panel::new("stack", &theme).focused(true);

        assert_eq!(
            unfocused.border_style().fg,
            theme.color(Slot::AccentMuted),
            "unfocused borders take the non-focused accent slot"
        );
        assert_eq!(
            focused.border_style().fg,
            theme.color(Slot::AccentPrimary),
            "focused borders take accent.primary"
        );

        let area = Rect::new(0, 0, 30, 10);
        assert_eq!(unfocused.inner(area), focused.inner(area));
    }

    #[test]
    fn renders_square_box_drawing_borders() {
        let theme = theme();
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        Panel::new("stack", &theme).render(area, &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "\u{250c}");
        assert_eq!(buf[(19, 0)].symbol(), "\u{2510}");
        assert_eq!(buf[(0, 4)].symbol(), "\u{2514}");
        assert_eq!(buf[(19, 4)].symbol(), "\u{2518}");

        let title: String = (1..14).map(|x| buf[(x, 0)].symbol()).collect();
        assert!(title.starts_with(" stack"), "title in the border: {title}");
    }

    #[test]
    fn monochrome_panel_carries_no_color() {
        let theme = DesignTheme::with_tier(Tier::Mono);
        let area = Rect::new(0, 0, 24, 6);
        let mut buf = Buffer::empty(area);
        Panel::new("stack", &theme).focused(true).render(area, &mut buf);
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &buf[(x, y)];
                assert_eq!(cell.fg, Color::Reset, "fg at {x},{y}");
                assert_eq!(cell.bg, Color::Reset, "bg at {x},{y}");
            }
        }
        // Focus and the shortcut letter survive as modifiers.
        let focused = Panel::new("stack", &theme).focused(true);
        assert!(focused.border_style().add_modifier.contains(Modifier::BOLD));
        let underlined = focused
            .title()
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::UNDERLINED));
        assert!(underlined, "the shortcut letter stays marked in monochrome");
    }

    #[test]
    fn tiny_areas_do_not_panic() {
        let theme = theme();
        for (w, h) in [(0, 0), (1, 1), (2, 2), (3, 1), (1, 3), (4, 3)] {
            let area = Rect::new(0, 0, w, h);
            let mut buf = Buffer::empty(Rect::new(0, 0, w.max(1), h.max(1)));
            Panel::new("x", &theme).render(area, &mut buf);
            let _ = Panel::new("x", &theme).inner(area);
        }
    }

    #[test]
    fn an_empty_name_still_produces_a_usable_title() {
        let theme = theme();
        let panel = Panel::new("", &theme);
        assert_eq!(panel.shortcut_char(), None);
        assert_eq!(title_text(&panel), "  ");
    }
}
