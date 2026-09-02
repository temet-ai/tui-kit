//! `ConsoleShell`: the full-screen temet console — native-cell chrome
//! ([`crate::tabs::Tabs`], [`crate::statusbar::StatusBar`], a header brand
//! line) composed around the existing image-based
//! [`crate::widget::CardGrid`] in one frame. The chrome is truecolor
//! ratatui cells; only the card content stays rasterized.

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{StatefulWidget, Widget};
use ratatui_image::picker::Picker;

use crate::cache::ImageCache;
use crate::card::Card;
use crate::nav::{self, NavAction, NavState};
use crate::statusbar::StatusBar;
use crate::tabs::Tabs;
use crate::theme::Theme;
use crate::widget::CardGrid;

/// The console's fixed tab set, in display order. `Tabs`/`ConsoleShell`
/// number them 1-based from this order; a caller's `tab_cards` and
/// `ConsoleState`'s `navs` must be given in the same order/length.
pub const TAB_TITLES: [&str; 3] = ["proc", "pods", "api"];

/// Static right-of-header node info, matching the console reference's
/// `node: <id> · up <uptime>` — decorative chrome, not live telemetry.
pub const NODE_INFO: &str = "node: temet-3 \u{b7} up 14d";

/// Rows the header block occupies: the brand/tabs/node-info content row plus
/// its bottom border hairline.
const HEADER_HEIGHT: u16 = 2;

/// Console-wide UI state: which tab is active, plus each tab's own
/// [`NavState`] — kept per-tab (rather than one state reset on switch) so
/// leaving a tab and coming back preserves where focus was.
pub struct ConsoleState {
    pub active_tab: usize,
    pub navs: Vec<NavState>,
}

impl ConsoleState {
    /// One `NavState` per tab, `columns` wide, sized from
    /// `tab_card_counts` (each tab's card count, in `TAB_TITLES` order).
    pub fn new(tab_card_counts: &[usize], columns: usize) -> Self {
        Self {
            active_tab: 0,
            navs: tab_card_counts.iter().map(|&len| NavState::new(len, columns)).collect(),
        }
    }

    /// The active tab's `NavState`, clamped so a mismatched `active_tab`
    /// (e.g. a `navs` shorter than `TAB_TITLES`) can't index out of bounds.
    fn active_nav_mut(&mut self) -> &mut NavState {
        let idx = self.active_tab.min(self.navs.len().saturating_sub(1));
        &mut self.navs[idx]
    }
}

/// The result of a console key press: [`crate::nav`]'s per-tab actions plus
/// the console-level tab switch and quit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleAction {
    /// The active tab changed to this (0-based) index.
    Tab(usize),
    /// A card-nav key was handled by the active tab's `NavState`.
    Nav(NavAction),
    /// `q` or Esc was pressed.
    Quit,
}

/// Handle one key against the whole console: `1..=3` switch the active tab,
/// `q`/Esc quit, everything else delegates to the active tab's
/// `nav::handle_key`. Pure/data-in-action-out like `nav::handle_key` itself
/// — no callbacks, no I/O.
pub fn handle_console_key(state: &mut ConsoleState, key: KeyEvent) -> Option<ConsoleAction> {
    // TODO: shortcuts overlay — StatusBar renders the "[⌃ /] shortcuts" hint
    // (see statusbar.rs), but Ctrl+/ isn't bound to anything here yet.
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(ConsoleAction::Quit),
        KeyCode::Char(c @ '1'..='3') => {
            let idx = (c as u8 - b'1') as usize;
            if idx < state.navs.len() {
                state.active_tab = idx;
                Some(ConsoleAction::Tab(idx))
            } else {
                None
            }
        }
        _ => nav::handle_key(state.active_nav_mut(), key).map(ConsoleAction::Nav),
    }
}

/// One full-screen console frame: a header (brand + [`Tabs`] + node info) on
/// top, the active tab's [`CardGrid`] in the middle, [`StatusBar`] pinned to
/// the bottom.
pub struct ConsoleShell<'a> {
    tab_cards: &'a [Vec<Card>],
    theme: &'a Theme,
    picker: &'a Picker,
    cache: &'a mut ImageCache,
    mode: &'a str,
    message: &'a str,
    keys: &'a [(&'a str, &'a str)],
}

impl<'a> ConsoleShell<'a> {
    /// `tab_cards` must have the same length/order as [`TAB_TITLES`] (proc,
    /// pods, api).
    pub fn new(
        tab_cards: &'a [Vec<Card>],
        theme: &'a Theme,
        picker: &'a Picker,
        cache: &'a mut ImageCache,
        mode: &'a str,
        message: &'a str,
        keys: &'a [(&'a str, &'a str)],
    ) -> Self {
        Self {
            tab_cards,
            theme,
            picker,
            cache,
            mode,
            message,
            keys,
        }
    }
}

impl<'a> StatefulWidget for ConsoleShell<'a> {
    type State = ConsoleState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ConsoleState) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(HEADER_HEIGHT),
                Constraint::Min(1),
                Constraint::Length(StatusBar::HEIGHT),
            ])
            .split(area);
        let (header_area, main_area, status_area) = (chunks[0], chunks[1], chunks[2]);

        render_header(header_area, buf, self.theme, state.active_tab);

        let active = state
            .active_tab
            .min(self.tab_cards.len().saturating_sub(1))
            .min(state.navs.len().saturating_sub(1));
        let cards = self.tab_cards.get(active).map(Vec::as_slice).unwrap_or(&[]);
        CardGrid::new(cards, self.theme, self.picker, self.cache).render(main_area, buf, &mut state.navs[active]);

        StatusBar::new(self.mode, self.message, self.keys, self.theme).render(status_area, buf);
    }
}

/// The header block: brand + [`Tabs`] + node info on row 0, a full-width `─`
/// hairline on row 1. `Tabs` itself is given height 1 here (its own border
/// only fires at height >= 2), so the shared border spans the WHOLE row —
/// under the brand and node-info columns too, not just the tab labels.
fn render_header(area: Rect, buf: &mut Buffer, theme: &Theme, active_tab: usize) {
    if area.height == 0 {
        return;
    }
    let row0 = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };

    let brand = brand_text();
    let brand_width = (brand.chars().count() as u16).min(row0.width);
    let node_width = (NODE_INFO.chars().count() as u16).min(row0.width.saturating_sub(brand_width));

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(brand_width),
            Constraint::Min(0),
            Constraint::Length(node_width),
        ])
        .split(row0);

    buf.set_stringn(
        cols[0].x,
        cols[0].y,
        &brand,
        cols[0].width as usize,
        Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
    );

    Tabs::new(&TAB_TITLES, active_tab, theme).render(cols[1], buf);

    let node_x = cols[2].x + cols[2].width.saturating_sub(node_width);
    buf.set_stringn(node_x, cols[2].y, NODE_INFO, node_width as usize, Style::new().fg(theme.faint));

    if area.height >= 2 {
        let line = "\u{2500}".repeat(area.width as usize);
        buf.set_stringn(area.x, area.y + 1, &line, area.width as usize, Style::new().fg(theme.border));
    }
}

/// `TEMET`, letter-spaced (a literal space between each glyph — the only way
/// to fake CSS `letter-spacing` on a fixed terminal cell grid) plus a
/// trailing block cursor.
fn brand_text() -> String {
    let mut s = String::new();
    for (i, c) in "TEMET".chars().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push(c);
    }
    s.push_str("  \u{258a}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn state() -> ConsoleState {
        // 3 tabs, 3/2/2 cards, 2 columns — mirrors what `ConsoleShell`
        // expects (`TAB_TITLES` order: proc, pods, api).
        ConsoleState::new(&[3, 2, 2], 2)
    }

    #[test]
    fn number_key_switches_tab() {
        let mut s = state();
        let action = handle_console_key(&mut s, key(KeyCode::Char('2')));
        assert_eq!(action, Some(ConsoleAction::Tab(1)));
        assert_eq!(s.active_tab, 1);
    }

    #[test]
    fn out_of_range_number_key_is_ignored() {
        let mut s = state();
        let action = handle_console_key(&mut s, key(KeyCode::Char('9')));
        assert_eq!(action, None);
        assert_eq!(s.active_tab, 0, "an unhandled key must not move the active tab");
    }

    #[test]
    fn q_and_esc_quit() {
        let mut s = state();
        assert_eq!(handle_console_key(&mut s, key(KeyCode::Char('q'))), Some(ConsoleAction::Quit));
        assert_eq!(handle_console_key(&mut s, key(KeyCode::Esc)), Some(ConsoleAction::Quit));
    }

    #[test]
    fn arrow_key_drives_active_tabs_nav() {
        let mut s = state();
        // Tab 0 has 3 cards, 2 columns: [0 1] [2]. Right from 0 -> 1.
        let action = handle_console_key(&mut s, key(KeyCode::Right));
        assert_eq!(action, Some(ConsoleAction::Nav(NavAction::Focus(1))));
        assert_eq!(s.navs[0].selected, 1);
    }

    #[test]
    fn each_tab_keeps_its_own_nav_selection() {
        let mut s = state();
        handle_console_key(&mut s, key(KeyCode::Right)); // tab 0 selection -> 1
        assert_eq!(s.navs[0].selected, 1);

        handle_console_key(&mut s, key(KeyCode::Char('2'))); // switch to tab 1
        assert_eq!(s.active_tab, 1);
        assert_eq!(s.navs[1].selected, 0, "switching tabs must not touch the new tab's own selection");

        handle_console_key(&mut s, key(KeyCode::Char('1'))); // back to tab 0
        assert_eq!(s.navs[0].selected, 1, "leaving and returning to a tab must preserve its selection");
    }

    #[test]
    fn enter_activates_via_nav() {
        let mut s = state();
        let action = handle_console_key(&mut s, key(KeyCode::Enter));
        assert_eq!(action, Some(ConsoleAction::Nav(NavAction::Activate(0))));
    }
}
