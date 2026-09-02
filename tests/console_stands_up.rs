//! Integration test: stand up the REAL `ConsoleShell` widget, `ConsoleState`,
//! and `handle_console_key` path headlessly (a `ratatui::backend::TestBackend`,
//! no real TTY) and prove it renders real chrome text and navigates without
//! panicking — the same shape as `tests/stands_up.rs` for `CardGrid`, as
//! `#[test]` functions so `cargo test` covers "the console stands up" on
//! every run.
//!
//! Self-contained fixtures: this file can't import `examples/console.rs`
//! (examples are separate binaries, not library code), so it builds its own
//! small card set — same pattern `tests/stands_up.rs` already uses relative
//! to `examples/gallery.rs`.

use tui_kit::ratatui::Terminal;
use tui_kit::ratatui::backend::TestBackend;
use tui_kit::ratatui::buffer::{Buffer, Cell};
use tui_kit::ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_kit::ratatui_image::picker::Picker;
use tui_kit::{Card, ConsoleAction, ConsoleShell, ConsoleState, ImageCache, NavAction, Theme, handle_console_key};

const COLUMNS: usize = 2;
const KEY_HINTS: [(&str, &str); 2] = [("1-3", "tab"), ("j/k", "move")];

/// 3 tabs (proc, pods, api — `tui_kit::console::TAB_TITLES` order), 3 cards
/// each so every tab's `CardGrid` wraps to a second row under `COLUMNS = 2`
/// and both Right and Down card-nav keys have somewhere to move focus.
fn tab_cards() -> Vec<Vec<Card>> {
    (0..3)
        .map(|tab| {
            (0..3)
                .map(|i| Card {
                    tag: format!("t{tab}"),
                    title: format!("Tab {tab} card {i}"),
                    subtitle: Some("console_stands_up fixture".to_string()),
                    bullets: vec!["one".to_string(), "two".to_string()],
                    action: Some("Open".to_string()),
                    hotkey: None,
                })
                .collect()
        })
        .collect()
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

// Deprecated constructor kept for deterministic 8x16 pixel output in
// headless paths: `Picker::halfblocks()` pins 10x20 and would change
// rendered output.
#[allow(deprecated)]
fn headless_picker() -> Picker {
    Picker::from_fontsize((8, 16).into())
}

/// Every row of `buf`, spaces stripped and newline-joined, so a search for
/// letter-spaced chrome text (the brand `"T E M E T"`) still finds it
/// contiguously (`"TEMET"`) while an ordinary-text search is unaffected.
fn buffer_text_stripped(buf: &Buffer) -> String {
    let area = buf.area;
    let mut out = String::new();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let symbol = buf.cell((x, y)).map(Cell::symbol).unwrap_or(" ");
            if symbol != " " {
                out.push_str(symbol);
            }
        }
        out.push('\n');
    }
    out
}

fn non_default_cells(buf: &Buffer) -> usize {
    buf.content().iter().filter(|cell| **cell != Cell::default()).count()
}

/// Render one frame of the real `ConsoleShell` `StatefulWidget` into
/// `terminal` and return a clone of the buffer for text/cell assertions.
fn render_frame(
    terminal: &mut Terminal<TestBackend>,
    tab_cards: &[Vec<Card>],
    theme: &Theme,
    picker: &Picker,
    cache: &mut ImageCache,
    state: &mut ConsoleState,
) -> Buffer {
    let completed = terminal
        .draw(|frame| {
            let shell = ConsoleShell::new(tab_cards, theme, picker, cache, "normal", "console ready", &KEY_HINTS);
            frame.render_stateful_widget(shell, frame.area(), state);
        })
        .expect("ConsoleShell must render into a TestBackend without an IO error");
    completed.buffer.clone()
}

#[test]
fn console_shell_renders_chrome_and_a_nonempty_frame_headlessly() {
    let theme = Theme::temet();
    let cards = tab_cards();
    let mut cache = ImageCache::new();
    let picker = headless_picker();
    let mut state = ConsoleState::new(&[3, 3, 3], COLUMNS);
    let mut terminal = Terminal::new(TestBackend::new(140, 44)).expect("TestBackend terminal");

    let buf = render_frame(&mut terminal, &cards, &theme, &picker, &mut cache, &mut state);

    assert!(
        non_default_cells(&buf) > 0,
        "ConsoleShell render into a TestBackend must paint non-default cells"
    );

    let text = buffer_text_stripped(&buf);
    for needle in ["TEMET", "PROC", "PODS", "API", "NORMAL"] {
        assert!(text.contains(needle), "rendered buffer is missing {needle:?}\n---\n{text}");
    }
}

#[test]
fn number_key_switches_the_active_tab() {
    let mut state = ConsoleState::new(&[3, 3, 3], COLUMNS);
    assert_eq!(state.active_tab, 0);

    let action = handle_console_key(&mut state, key(KeyCode::Char('3')));
    assert_eq!(action, Some(ConsoleAction::Tab(2)));
    assert_eq!(state.active_tab, 2);
}

#[test]
fn arrow_keys_drive_the_active_tabs_card_nav() {
    let mut state = ConsoleState::new(&[3, 3, 3], COLUMNS);

    // 3 cards, 2 columns: [0 1] / [2]. Right from 0 -> 1; Down from 1 -> 2.
    let action = handle_console_key(&mut state, key(KeyCode::Right));
    assert_eq!(action, Some(ConsoleAction::Nav(NavAction::Focus(1))));
    assert_eq!(state.navs[0].selected, 1);

    let action = handle_console_key(&mut state, key(KeyCode::Down));
    assert_eq!(action, Some(ConsoleAction::Nav(NavAction::Focus(2))));
    assert_eq!(state.navs[0].selected, 2);
}

#[test]
fn switching_tabs_preserves_each_tabs_own_selection() {
    let mut state = ConsoleState::new(&[3, 3, 3], COLUMNS);

    handle_console_key(&mut state, key(KeyCode::Right));
    assert_eq!(state.navs[0].selected, 1);

    handle_console_key(&mut state, key(KeyCode::Char('2')));
    assert_eq!(state.active_tab, 1);
    assert_eq!(state.navs[1].selected, 0, "a freshly-switched-to tab starts at its own selection, not tab 0's");

    handle_console_key(&mut state, key(KeyCode::Char('1')));
    assert_eq!(state.active_tab, 0);
    assert_eq!(state.navs[0].selected, 1, "returning to tab 0 must keep its earlier selection");
}

#[test]
fn q_and_esc_quit() {
    let mut state = ConsoleState::new(&[3, 3, 3], COLUMNS);
    assert_eq!(handle_console_key(&mut state, key(KeyCode::Char('q'))), Some(ConsoleAction::Quit));
    assert_eq!(handle_console_key(&mut state, key(KeyCode::Esc)), Some(ConsoleAction::Quit));
}

#[test]
fn real_key_sequence_navigates_switches_tabs_and_renders_after() {
    let theme = Theme::temet();
    let cards = tab_cards();
    let mut cache = ImageCache::new();
    let picker = headless_picker();
    let mut state = ConsoleState::new(&[3, 3, 3], COLUMNS);
    let mut terminal = Terminal::new(TestBackend::new(140, 44)).expect("TestBackend terminal");

    render_frame(&mut terminal, &cards, &theme, &picker, &mut cache, &mut state);

    assert_eq!(handle_console_key(&mut state, key(KeyCode::Char('2'))), Some(ConsoleAction::Tab(1)));
    assert_eq!(
        handle_console_key(&mut state, key(KeyCode::Right)),
        Some(ConsoleAction::Nav(NavAction::Focus(1)))
    );
    assert_eq!(handle_console_key(&mut state, key(KeyCode::Char('q'))), Some(ConsoleAction::Quit));

    let buf = render_frame(&mut terminal, &cards, &theme, &picker, &mut cache, &mut state);
    assert!(
        non_default_cells(&buf) > 0,
        "a second ConsoleShell render after tab-switch + nav must still paint non-default cells"
    );
    let text = buffer_text_stripped(&buf);
    assert!(text.contains("PODS"), "post-switch frame should still show the PODS tab label");
}
