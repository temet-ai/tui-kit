//! Ratzilla browser demo: the `tui-kit` temet card gallery, rendered in a
//! browser terminal.
//!
//! The whole pipeline runs client-side in WebAssembly: each card is authored
//! as SVG by `tui_kit::svg::card_to_svg`, rasterized to RGBA by the vendored
//! `resvg`/`tiny-skia` renderer, and placed into the ratatui `Buffer` by
//! `tui_kit::CardGrid` as half-block cells (upper-half-block glyph, foreground
//! = top pixel, background = bottom pixel). Ratzilla's `DomBackend` then paints
//! that `Buffer` as DOM text, so no terminal graphics protocol — and no
//! terminal — is involved.
//!
//! Build (no trunk required for the compile gate):
//!
//! ```text
//! cargo build --target wasm32-unknown-unknown
//! ```
//!
//! Serve with `trunk serve` when trunk is available; `index.html` carries the
//! `data-trunk` link tag it needs.

use std::cell::RefCell;
use std::rc::Rc;

use ratzilla::DomBackend;
use ratzilla::WebRenderer;
use ratzilla::event::{KeyCode, KeyEvent};
use ratzilla::ratatui::Terminal;
use ratzilla::ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratzilla::ratatui::style::{Modifier, Style};
use ratzilla::ratatui::text::{Line, Span};
use ratzilla::ratatui::widgets::{Paragraph, StatefulWidget, Widget};

use tui_kit::cache::ImageCache;
use tui_kit::card::Card;
use tui_kit::nav::NavState;
use tui_kit::ratatui_image::picker::Picker;
use tui_kit::theme::Theme;
use tui_kit::widget::CardGrid;

/// Grid width in cards. `NavState` uses the same value for arrow-key motion.
const COLUMNS: usize = 3;

fn main() {
    // Panics inside a wasm module otherwise surface as an opaque
    // `RuntimeError: unreachable`; this prints the Rust message and a
    // backtrace to the browser console instead.
    console_error_panic_hook::set_once();

    let backend = DomBackend::new().expect("DomBackend");
    let mut terminal = Terminal::new(backend).expect("terminal");

    let cards = gallery_cards();
    let theme = Theme::temet();

    // The half-block protocol: each cell becomes an upper-half-block glyph
    // whose foreground is the top pixel and background the bottom pixel, so a
    // card lands in the ratatui `Buffer` as plain colored cells. That is the
    // only protocol that can work here — Kitty/iTerm2/Sixel all need a real
    // terminal to interpret their escape bytes, and the DOM backend paints
    // `Buffer` cells as spans.
    //
    // `Picker::halfblocks()` also sidesteps the autodetection problem:
    // `from_query_stdio` writes a capability query to the tty and reads the
    // reply back, which cannot happen in a browser. It instead states a fixed
    // 10x20 cell, which is what the rasterizer targets and what index.html's
    // CSS is sized to match.
    let picker = Picker::halfblocks();

    // Shared across the render closure and the key handler, both of which are
    // `'static` callbacks owned by the browser event loop. Single-threaded
    // wasm, so `Rc<RefCell<_>>` (no `Arc`/`Mutex`).
    let state = Rc::new(RefCell::new(NavState::new(cards.len(), COLUMNS)));
    let cache = Rc::new(RefCell::new(ImageCache::new()));
    let cards = Rc::new(cards);

    {
        let state = Rc::clone(&state);
        let card_count = cards.len();
        let handler = move |event: KeyEvent| {
            // `tui_kit::nav::handle_key` takes a crossterm `KeyEvent` and is
            // behind the `input` feature, which is off here (crossterm does
            // not build for wasm). Ratzilla's own key events are mapped
            // directly onto the same `NavState` fields instead.
            let mut state = state.borrow_mut();
            let selected = state.selected;
            let next = match event.code {
                KeyCode::Left | KeyCode::Char('h') => selected.saturating_sub(1),
                KeyCode::Right | KeyCode::Char('l') => (selected + 1).min(card_count - 1),
                KeyCode::Up | KeyCode::Char('k') => selected.saturating_sub(COLUMNS),
                KeyCode::Down | KeyCode::Char('j') => {
                    (selected + COLUMNS).min(card_count.saturating_sub(1))
                }
                KeyCode::Home => 0,
                KeyCode::End => card_count.saturating_sub(1),
                _ => selected,
            };
            state.selected = next;
        };
        // A `WebGl2Backend` would silently accept and ignore this; `DomBackend`
        // registers a real `keydown` listener, and only fails if attaching it
        // fails.
        terminal.on_key_event(handler).expect("key listener");
    }

    terminal.draw_web(move |frame| {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let buf = frame.buffer_mut();
        header(&theme).render(chunks[0], buf);
        footer(&theme, state.borrow().selected, &cards).render(chunks[2], buf);

        let grid_area = pad(chunks[1]);
        let mut cache = cache.borrow_mut();
        let mut state = state.borrow_mut();
        CardGrid::new(&cards, &theme, &picker, &mut cache).render(grid_area, buf, &mut state);
    });
}

/// One cell of horizontal breathing room on each side, so the outermost card
/// hairlines are not flush against the viewport edge.
fn pad(area: Rect) -> Rect {
    if area.width <= 2 {
        return area;
    }
    Rect {
        x: area.x + 1,
        width: area.width - 2,
        ..area
    }
}

fn header(theme: &Theme) -> Paragraph<'static> {
    Paragraph::new(Line::from(vec![
        Span::styled(
            " TUI-KIT ",
            Style::default()
                .fg(theme.app)
                .bg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" temet card gallery ", Style::default().fg(theme.text)),
        Span::styled("// wasm", Style::default().fg(theme.muted)),
    ]))
    .style(Style::default().bg(theme.app))
}

fn footer(theme: &Theme, selected: usize, cards: &[Card]) -> Paragraph<'static> {
    let title = cards
        .get(selected)
        .map(|c| c.title.clone())
        .unwrap_or_default();
    Paragraph::new(Line::from(vec![
        Span::styled(" \u{25b8} ", Style::default().fg(theme.accent)),
        Span::styled(title, Style::default().fg(theme.text)),
        Span::styled(
            "   arrows / hjkl to move",
            Style::default().fg(theme.faint),
        ),
    ]))
    .style(Style::default().bg(theme.app))
}

/// The demo's five cards. Plain `tui_kit::Card` values — the toolset knows
/// nothing about this demo, and this demo hardcodes no styling.
fn gallery_cards() -> Vec<Card> {
    vec![
        card(
            "PROC",
            "Process",
            Some("scheduler view"),
            &["run queue depth", "per-core affinity", "cgroup limits"],
            "Inspect",
            "p",
        ),
        card(
            "DATA",
            "Data",
            Some("column store"),
            &["schema drift", "row counts", "retention window"],
            "Query",
            "d",
        ),
        card(
            "FLOW",
            "Flow",
            Some("pipeline graph"),
            &["stage latency", "backpressure", "retry budget"],
            "Trace",
            "f",
        ),
        card(
            "MEM",
            "Memory",
            Some("arena map"),
            &["resident set", "fragmentation", "page faults"],
            "Profile",
            "m",
        ),
        card(
            "EXEC",
            "Execute",
            Some("task runner"),
            &["queued jobs", "worker pool", "exit codes"],
            "Launch",
            "e",
        ),
    ]
}

fn card(
    tag: &str,
    title: &str,
    subtitle: Option<&str>,
    bullets: &[&str],
    action: &str,
    hotkey: &str,
) -> Card {
    Card {
        tag: tag.to_string(),
        title: title.to_string(),
        subtitle: subtitle.map(str::to_string),
        bullets: bullets.iter().map(|b| b.to_string()).collect(),
        action: Some(action.to_string()),
        hotkey: Some(hotkey.to_string()),
    }
}
