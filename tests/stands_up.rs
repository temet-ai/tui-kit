//! Integration test: stand up the REAL `CardGrid` widget + `NavState` +
//! `handle_key` path headlessly (a `ratatui::backend::TestBackend`, no real
//! TTY) and prove it renders and navigates without panicking — the same
//! assertions `examples/gallery.rs --selftest` prints, as `#[test]`
//! functions so `cargo test` covers "it stands up" on every run.
//!
//! `Picker::from_fontsize` (unlike `from_query_stdio`) never touches stdio,
//! so it's the headless-safe constructor; with no tmux/iTerm2 env hints it
//! resolves to the `Halfblocks` protocol, which paints plain colored
//! `Buffer` cells rather than emitting a real terminal graphics escape
//! sequence — exactly what a `TestBackend` can render, so this exercises
//! `CardGrid`'s real image path, not a stub.

use tui_kit::ratatui::Terminal;
use tui_kit::ratatui::backend::TestBackend;
use tui_kit::ratatui::buffer::Cell;
use tui_kit::ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_kit::ratatui_image::picker::Picker;
use tui_kit::{Card, CardGrid, ImageCache, NavAction, NavState, Theme, handle_key};

const COLUMNS: usize = 3;

fn sample_cards() -> Vec<Card> {
    (0..5)
        .map(|i| Card {
            tag: format!("t{i}"),
            title: format!("Card {i}"),
            subtitle: Some("stands_up fixture".to_string()),
            bullets: vec!["one".to_string(), "two".to_string()],
            action: Some("Open".to_string()),
            hotkey: None,
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

/// Render one frame of the real `CardGrid` `StatefulWidget` into `terminal`
/// and return how many buffer cells are non-default, so tests can assert
/// the image path actually painted something rather than staying blank.
fn render_frame(
    terminal: &mut Terminal<TestBackend>,
    theme: &Theme,
    cards: &[Card],
    picker: &Picker,
    cache: &mut ImageCache,
    nav: &mut NavState,
) -> usize {
    let completed = terminal
        .draw(|frame| {
            let grid = CardGrid::new(cards, theme, picker, cache);
            frame.render_stateful_widget(grid, frame.area(), nav);
        })
        .expect("CardGrid must render into a TestBackend without an IO error");
    completed
        .buffer
        .content()
        .iter()
        .filter(|cell| **cell != Cell::default())
        .count()
}

#[test]
fn card_grid_renders_a_nonempty_frame_headlessly() {
    let theme = Theme::temet();
    let cards = sample_cards();
    let mut cache = ImageCache::new();
    let picker = headless_picker();
    let mut nav = NavState::new(cards.len(), COLUMNS);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("TestBackend terminal");

    let non_default = render_frame(&mut terminal, &theme, &cards, &picker, &mut cache, &mut nav);
    assert!(
        non_default > 0,
        "CardGrid render into a TestBackend must paint non-default cells"
    );
}

#[test]
fn real_handle_key_sequence_navigates_and_renders_after() {
    let theme = Theme::temet();
    let cards = sample_cards();
    let mut cache = ImageCache::new();
    let picker = headless_picker();
    let mut nav = NavState::new(cards.len(), COLUMNS);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("TestBackend terminal");

    render_frame(&mut terminal, &theme, &cards, &picker, &mut cache, &mut nav);

    // 5 cards, 3 columns: [0 1 2] / [3 4]. From 0: Down lands on the row
    // below at the same column (3); Right from 3 clamps at the end of its
    // shorter 2-wide row (4).
    assert_eq!(
        handle_key(&mut nav, key(KeyCode::Down)),
        Some(NavAction::Focus(3))
    );
    assert_eq!(nav.selected, 3);

    assert_eq!(
        handle_key(&mut nav, key(KeyCode::Right)),
        Some(NavAction::Focus(4))
    );
    assert_eq!(nav.selected, 4);

    assert_eq!(
        handle_key(&mut nav, key(KeyCode::Enter)),
        Some(NavAction::Activate(4)),
        "Enter must Activate the focused index"
    );
    assert_eq!(nav.selected, 4, "Activate must not move focus");

    assert_eq!(
        handle_key(&mut nav, key(KeyCode::Esc)),
        Some(NavAction::Back),
        "Esc must yield Back"
    );

    let non_default = render_frame(&mut terminal, &theme, &cards, &picker, &mut cache, &mut nav);
    assert!(
        non_default > 0,
        "a second CardGrid render after navigation must still paint non-default cells"
    );
}
