//! Runnable demo of `CardGrid`: a grid of sample temet-styled cards
//! navigable with the arrow keys / hjkl, `q`/Esc to quit.
//!
//! `cargo run --example gallery -- --print` skips the alt-screen entirely
//! and instead rasterizes one focused and one unfocused sample card to
//! `gallery_focused.png` / `gallery_unfocused.png` next to the crate root —
//! headless visual evidence for environments with no real terminal graphics
//! protocol.
//!
//! `cargo run --example gallery -- --grid` composes the WHOLE nav — every
//! sample card at its grid position, the selected one focused — into one
//! `gallery_grid.png`, so there's a single artifact that reads as the actual
//! screen rather than one isolated card.
//!
//! `cargo run --example gallery -- --selftest` stands up the real
//! `CardGrid`/`NavState`/`handle_key` path headlessly (a `TestBackend`, no
//! TTY) and drives a scripted key sequence through it, printing
//! `SELFTEST PASS` and exiting 0 on success.

use std::io;
use std::time::Duration;

use tui_kit::ratatui::Terminal;
use tui_kit::ratatui::backend::{CrosstermBackend, TestBackend};
use tui_kit::ratatui::buffer::Cell;
use tui_kit::ratatui::crossterm::event::{self, Event};
use tui_kit::ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use tui_kit::ratatui::crossterm::{ExecutableCommand, execute};
use tui_kit::ratatui_image::picker::Picker;
use tui_kit::{Card, CardGrid, ImageCache, NavAction, NavState, Theme, card_to_svg, handle_key, rasterize};

fn sample_cards() -> Vec<Card> {
    vec![
        Card {
            tag: "proc".to_string(),
            title: "Process monitor".to_string(),
            subtitle: Some("128 units · systemd".to_string()),
            bullets: vec![
                "Live CPU/RSS per unit".to_string(),
                "Auto-restart on failure".to_string(),
            ],
            action: Some("Inspect".to_string()),
            hotkey: Some("p".to_string()),
        },
        Card {
            tag: "pods".to_string(),
            title: "Pod fleet".to_string(),
            subtitle: Some("12 running · 0 pending".to_string()),
            bullets: vec![
                "Rolling restarts tracked".to_string(),
                "Node affinity respected".to_string(),
            ],
            action: Some("View".to_string()),
            hotkey: Some("d".to_string()),
        },
        Card {
            tag: "api".to_string(),
            title: "API gateway".to_string(),
            subtitle: Some("p99 42ms · 0.02% err".to_string()),
            bullets: vec![
                "Rate limits per route".to_string(),
                "Circuit breaker armed".to_string(),
            ],
            action: Some("Open".to_string()),
            hotkey: Some("f".to_string()),
        },
        Card {
            tag: "mem".to_string(),
            title: "Memory pressure".to_string(),
            subtitle: Some("61% used · 4.2GB free".to_string()),
            bullets: vec![
                "Per-process RSS trend".to_string(),
                "OOM-killer watch armed".to_string(),
            ],
            action: Some("Trace".to_string()),
            hotkey: Some("m".to_string()),
        },
        Card {
            tag: "net".to_string(),
            title: "Network I/O".to_string(),
            subtitle: Some("840 Mbps in · 210 out".to_string()),
            bullets: vec![
                "Per-interface throughput".to_string(),
                "Retransmit rate tracked".to_string(),
            ],
            action: Some("Graph".to_string()),
            hotkey: Some("e".to_string()),
        },
    ]
}

const COLUMNS: usize = 3;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--print") {
        return print_mode();
    }
    if std::env::args().any(|a| a == "--grid") {
        return grid_mode();
    }
    if std::env::args().any(|a| a == "--selftest") {
        return selftest_mode();
    }
    interactive_mode()
}

/// Headless visual evidence: rasterize one focused and one unfocused sample
/// card straight to PNG, with no terminal/alt-screen involved at all.
fn print_mode() -> Result<(), Box<dyn std::error::Error>> {
    let theme = Theme::temet();
    let cards = sample_cards();
    let sample = &cards[0];

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (focused, name) in [(true, "gallery_focused.png"), (false, "gallery_unfocused.png")] {
        let svg = card_to_svg(sample, &theme, focused);
        let img = rasterize(&svg, 840, Some(theme.bg_rgba()))?;
        let path = out_dir.join(name);
        img.save(&path)?;
        let bytes = std::fs::metadata(&path)?.len();
        println!(
            "{} ({}x{}, {} bytes)",
            path.display(),
            img.width(),
            img.height(),
            bytes
        );
    }
    Ok(())
}

/// SVG user-unit size of every card canvas. Mirrors `svg::CARD_W`/`CARD_H`
/// (crate-private, so not reachable from an example): every `card_to_svg`
/// output is a fixed 400x248 document at this size.
const CARD_W: f32 = 400.0;
const CARD_H: f32 = 248.0;
/// Gap between grid cells, and around the page edge, in the same SVG user
/// units as the cards.
const GRID_GAP: f32 = 16.0;

/// Compose every sample card's own `card_to_svg` output into one page SVG,
/// tiled on a `nav.columns`-wide grid with `nav.selected` focused and every
/// other card unfocused. Each card keeps its own 400x248 viewBox and is
/// placed by adding `x`/`y` to its own root `<svg>` tag, nesting it verbatim
/// at its grid cell — so this reuses the exact per-card markup `CardGrid`
/// rasterizes, just tiled instead of cropped to one cell. Returns the
/// composed document plus its page width (SVG user units), so the caller can
/// pick a raster scale.
fn compose_grid_svg(cards: &[Card], theme: &Theme, nav: &NavState) -> (String, f32) {
    let columns = nav.columns.max(1);
    let rows = cards.len().div_ceil(columns);
    let page_w = GRID_GAP + columns as f32 * (CARD_W + GRID_GAP);
    let page_h = GRID_GAP + rows as f32 * (CARD_H + GRID_GAP);
    let bg = hex_rgba(theme.bg_rgba());

    let mut body = String::new();
    for (idx, card) in cards.iter().enumerate() {
        let col = idx % columns;
        let row = idx / columns;
        let x = GRID_GAP + col as f32 * (CARD_W + GRID_GAP);
        let y = GRID_GAP + row as f32 * (CARD_H + GRID_GAP);
        let focused = idx == nav.selected;
        let card_svg = card_to_svg(card, theme, focused);
        let placed = card_svg.replacen("<svg ", &format!("<svg x=\"{x}\" y=\"{y}\" "), 1);
        body.push_str(&placed);
        body.push('\n');
    }

    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{page_w}" height="{page_h}" viewBox="0 0 {page_w} {page_h}">
  <rect x="0" y="0" width="{page_w}" height="{page_h}" fill="{bg}"/>
{body}</svg>"#
    );
    (svg, page_w)
}

fn hex_rgba(rgba: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2])
}

/// Headless visual evidence for the WHOLE nav: compose every sample card at
/// its grid position (selected one focused) into a single parent SVG and
/// rasterize it once, straight to PNG — the artifact that reads as an
/// actual screen, not one card in isolation.
fn grid_mode() -> Result<(), Box<dyn std::error::Error>> {
    let theme = Theme::temet();
    let cards = sample_cards();
    let nav = NavState::new(cards.len(), COLUMNS);

    let (svg, page_w) = compose_grid_svg(&cards, &theme, &nav);
    // Same ~2.1x scale-up as --print's single-card 400->840px raster, so the
    // grid reads crisply instead of at 1 SVG-unit-per-pixel.
    let target_width_px = (page_w * 2.1) as u32;
    let img = rasterize(&svg, target_width_px, Some(theme.bg_rgba()))?;

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = out_dir.join("gallery_grid.png");
    img.save(&path)?;
    let bytes = std::fs::metadata(&path)?.len();
    println!(
        "{} ({}x{}, {} bytes)",
        path.display(),
        img.width(),
        img.height(),
        bytes
    );
    Ok(())
}

fn interactive_mode() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::temet();
    let cards = sample_cards();
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::from_fontsize((8, 16)));
    let mut cache = ImageCache::new();
    let mut nav = NavState::new(cards.len(), COLUMNS);

    let result = run(&mut terminal, &theme, &cards, &picker, &mut cache, &mut nav);

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    theme: &Theme,
    cards: &[Card],
    picker: &Picker,
    cache: &mut ImageCache,
    nav: &mut NavState,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| {
            let grid = CardGrid::new(cards, theme, picker, cache);
            frame.render_stateful_widget(grid, frame.area(), nav);
        })?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                use tui_kit::ratatui::crossterm::event::KeyCode;
                if matches!(key.code, KeyCode::Char('q')) {
                    return Ok(());
                }
                match handle_key(nav, key) {
                    Some(NavAction::Back) => return Ok(()),
                    Some(NavAction::Activate(_)) | Some(NavAction::Focus(_)) | None => {}
                }
            }
        }
    }
}

/// Render one frame of the real `CardGrid` `StatefulWidget` into `terminal`
/// and return how many cells in the resulting buffer are non-default —
/// evidence the image path actually painted something, not just blank
/// cells. Wrapped in `catch_unwind` so a panic inside `CardGrid`/
/// `ratatui-image` is reported as a clear failure instead of an opaque abort.
fn render_frame_headless(
    terminal: &mut Terminal<TestBackend>,
    theme: &Theme,
    cards: &[Card],
    picker: &Picker,
    cache: &mut ImageCache,
    nav: &mut NavState,
) -> Result<usize, Box<dyn std::error::Error>> {
    use std::panic::{self, AssertUnwindSafe};

    // The cell count is computed *inside* the caught closure (rather than
    // returning the borrowed `CompletedFrame` out of it) because a closure
    // cannot return a reference derived from its own captured `terminal`.
    let result: io::Result<usize> = panic::catch_unwind(AssertUnwindSafe(|| {
        let drawn = terminal.draw(|frame| {
            let grid = CardGrid::new(cards, theme, picker, cache);
            frame.render_stateful_widget(grid, frame.area(), nav);
        })?;
        Ok(drawn
            .buffer
            .content()
            .iter()
            .filter(|cell| **cell != Cell::default())
            .count())
    }))
    .map_err(|_| "CardGrid render panicked inside TestBackend".to_string())?;

    Ok(result?)
}

/// `--selftest`: stand up the REAL `CardGrid` + `NavState` + `handle_key`
/// path headlessly (a `ratatui::backend::TestBackend`, no real TTY) and
/// prove it renders and navigates without panicking.
///
/// `Picker::from_fontsize` (unlike `from_query_stdio`) never touches stdio,
/// so it's the headless-safe constructor; with no tmux/iTerm2 env hints it
/// resolves to the `Halfblocks` protocol, which paints plain colored
/// `Buffer` cells rather than emitting a real terminal graphics escape
/// sequence — exactly what a `TestBackend` can render, so this exercises
/// `CardGrid`'s real image path, not a stub.
///
/// Prints `SELFTEST PASS` and returns `Ok(())` (exit 0) if every assertion
/// holds; otherwise returns `Err` (exit non-zero) describing what failed.
fn selftest_mode() -> Result<(), Box<dyn std::error::Error>> {
    use tui_kit::ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let theme = Theme::temet();
    let cards = sample_cards();
    let mut cache = ImageCache::new();
    let picker = Picker::from_fontsize((8, 16));
    let mut nav = NavState::new(cards.len(), COLUMNS);

    println!(
        "selftest: {} sample cards, {} columns, Picker protocol = {:?}",
        cards.len(),
        COLUMNS,
        picker.protocol_type()
    );

    let mut terminal = Terminal::new(TestBackend::new(120, 40))?;

    let non_default = render_frame_headless(&mut terminal, &theme, &cards, &picker, &mut cache, &mut nav)?;
    if non_default == 0 {
        return Err("selftest FAIL: rendered TestBackend buffer has no non-default cells".into());
    }
    println!(
        "selftest: frame 1 rendered via the real CardGrid StatefulWidget ({non_default} non-default cells)"
    );

    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::NONE);
    let before = nav.selected;
    let action = handle_key(&mut nav, key(KeyCode::Down));
    println!("selftest: Down   -> {action:?} (selected {before} -> {})", nav.selected);
    assert_eq!(nav.selected, 3, "Down from 0 in a 3-col/5-item grid should land on index 3");

    let action = handle_key(&mut nav, key(KeyCode::Right));
    println!("selftest: Right  -> {action:?} (selected -> {})", nav.selected);
    assert_eq!(nav.selected, 4, "Right from 3 (a 2-wide final row) should land on index 4");

    let action = handle_key(&mut nav, key(KeyCode::Enter));
    println!("selftest: Enter  -> {action:?}");
    assert_eq!(
        action,
        Some(NavAction::Activate(4)),
        "Enter must Activate the focused index"
    );

    let action = handle_key(&mut nav, key(KeyCode::Esc));
    println!("selftest: Esc    -> {action:?}");
    assert_eq!(action, Some(NavAction::Back), "Esc must yield Back");

    let non_default = render_frame_headless(&mut terminal, &theme, &cards, &picker, &mut cache, &mut nav)?;
    if non_default == 0 {
        return Err("selftest FAIL: post-nav TestBackend buffer has no non-default cells".into());
    }
    println!(
        "selftest: frame 2 (post-nav, selection now {}) rendered ({non_default} non-default cells)",
        nav.selected
    );

    println!("SELFTEST PASS");
    Ok(())
}
