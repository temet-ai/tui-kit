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
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;

use tui_kit::ratatui::backend::{CrosstermBackend, TestBackend};
use tui_kit::ratatui::buffer::{Buffer, Cell};
use tui_kit::ratatui::crossterm::event::{self, Event};
use tui_kit::ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui_kit::ratatui::crossterm::{execute, ExecutableCommand};
use tui_kit::ratatui::widgets::StatefulWidget;
use tui_kit::ratatui::Terminal;
use tui_kit::ratatui_image::picker::Picker;
use tui_kit::ratatui_image::protocol::StatefulProtocol;
use tui_kit::ratatui_image::StatefulImage;
use tui_kit::{
    card_to_svg, handle_key, rasterize, Card, CardGrid, ImageCache, NavAction, NavState, Theme,
};

use notify_debouncer_full::notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};

// Deprecated constructor kept for deterministic 8x16 pixel output in
// headless paths: `Picker::halfblocks()` pins 10x20 and would change
// rendered output.
#[allow(deprecated)]
fn headless_picker() -> Picker {
    Picker::from_fontsize((8, 16).into())
}

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
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--print") {
        return print_mode();
    }
    if args.iter().any(|a| a == "--grid") {
        return grid_mode();
    }
    if args.iter().any(|a| a == "--selftest") {
        return selftest_mode();
    }
    if let Some(dir) = arg_value(&args, "--dump-svg") {
        return dump_svg_mode(Path::new(&dir));
    }
    if let Some(dir) = arg_value(&args, "--watch") {
        return watch_mode(Path::new(&dir));
    }
    if args.iter().any(|a| a == "--watch-selftest") {
        return watch_selftest_mode();
    }
    interactive_mode()
}

/// Value of the flag `name` (e.g. `--watch <dir>`): the argument immediately
/// following it, if present.
fn arg_value(args: &[String], name: &str) -> Option<String> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1).cloned()
}

/// Headless visual evidence: rasterize one focused and one unfocused sample
/// card straight to PNG, with no terminal/alt-screen involved at all.
fn print_mode() -> Result<(), Box<dyn std::error::Error>> {
    let theme = Theme::temet();
    let cards = sample_cards();
    let sample = &cards[0];

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (focused, name) in [
        (true, "gallery_focused.png"),
        (false, "gallery_unfocused.png"),
    ] {
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
    let card_svgs: Vec<String> = cards
        .iter()
        .enumerate()
        .map(|(idx, card)| card_to_svg(card, theme, idx == nav.selected))
        .collect();
    compose_grid_svg_from(&card_svgs, columns, theme)
}

/// Same tiling as [`compose_grid_svg`], but taking already-rendered per-card
/// SVG markup directly instead of generating it from `Card`/`Theme` — this
/// is what lets `--watch` mode splice in SVGs loaded from disk (or reloaded
/// after an edit) without touching `card_to_svg` at all.
fn compose_grid_svg_from(card_svgs: &[String], columns: usize, theme: &Theme) -> (String, f32) {
    let columns = columns.max(1);
    let rows = card_svgs.len().div_ceil(columns);
    let page_w = GRID_GAP + columns as f32 * (CARD_W + GRID_GAP);
    let page_h = GRID_GAP + rows as f32 * (CARD_H + GRID_GAP);
    let bg = hex_rgba(theme.bg_rgba());

    let mut body = String::new();
    for (idx, card_svg) in card_svgs.iter().enumerate() {
        let col = idx % columns;
        let row = idx / columns;
        let x = GRID_GAP + col as f32 * (CARD_W + GRID_GAP);
        let y = GRID_GAP + row as f32 * (CARD_H + GRID_GAP);
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
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| headless_picker());
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
    let result: Result<usize, std::convert::Infallible> =
        panic::catch_unwind(AssertUnwindSafe(|| {
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
    let picker = headless_picker();
    let mut nav = NavState::new(cards.len(), COLUMNS);

    println!(
        "selftest: {} sample cards, {} columns, Picker protocol = {:?}",
        cards.len(),
        COLUMNS,
        picker.protocol_type()
    );

    let mut terminal = Terminal::new(TestBackend::new(120, 40))?;

    let non_default =
        render_frame_headless(&mut terminal, &theme, &cards, &picker, &mut cache, &mut nav)?;
    if non_default == 0 {
        return Err("selftest FAIL: rendered TestBackend buffer has no non-default cells".into());
    }
    println!(
        "selftest: frame 1 rendered via the real CardGrid StatefulWidget ({non_default} non-default cells)"
    );

    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::NONE);
    let before = nav.selected;
    let action = handle_key(&mut nav, key(KeyCode::Down));
    println!(
        "selftest: Down   -> {action:?} (selected {before} -> {})",
        nav.selected
    );
    assert_eq!(
        nav.selected, 3,
        "Down from 0 in a 3-col/5-item grid should land on index 3"
    );

    let action = handle_key(&mut nav, key(KeyCode::Right));
    println!(
        "selftest: Right  -> {action:?} (selected -> {})",
        nav.selected
    );
    assert_eq!(
        nav.selected, 4,
        "Right from 3 (a 2-wide final row) should land on index 4"
    );

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

    let non_default =
        render_frame_headless(&mut terminal, &theme, &cards, &picker, &mut cache, &mut nav)?;
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

// ---------------------------------------------------------------------
// Hot-reload design loop: --dump-svg / --watch / --watch-selftest
// ---------------------------------------------------------------------

/// Path a card's editable SVG lives at inside a watch/dump directory —
/// keyed by `card.tag` so the file name is stable and human-legible.
fn card_svg_path(dir: &Path, card: &Card) -> PathBuf {
    dir.join(format!("{}.svg", card.tag))
}

/// `--dump-svg <dir>`: write each sample card's freshly generated
/// (unfocused) SVG to `<dir>/<tag>.svg` and exit. This is the other half of
/// the design loop: open one of these files in an editor, then run
/// `--watch <dir>` to see edits reflected live.
fn dump_svg_mode(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let theme = Theme::temet();
    let cards = sample_cards();
    for card in &cards {
        let svg = card_to_svg(card, &theme, false);
        let path = card_svg_path(dir, card);
        std::fs::write(&path, &svg)?;
        println!("{}", path.display());
    }
    Ok(())
}

/// Load every sample card's SVG from `dir` (as written by `--dump-svg`),
/// falling back to a freshly generated one for any card whose file is
/// missing or unreadable.
fn load_card_svgs(dir: &Path, cards: &[Card], theme: &Theme) -> Vec<String> {
    cards
        .iter()
        .map(|card| {
            let path = card_svg_path(dir, card);
            std::fs::read_to_string(&path).unwrap_or_else(|_| card_to_svg(card, theme, false))
        })
        .collect()
}

/// Rebuild the composed grid image (all cards tiled per `COLUMNS`) from the
/// current `svgs`, and encode it into a `ratatui-image` protocol via
/// `picker`.
fn build_grid_protocol(
    svgs: &[String],
    theme: &Theme,
    picker: &Picker,
    target_width_px: u32,
) -> Result<StatefulProtocol, Box<dyn std::error::Error>> {
    let (svg, _page_w) = compose_grid_svg_from(svgs, COLUMNS, theme);
    let img = rasterize(&svg, target_width_px, Some(theme.bg_rgba()))?;
    Ok(picker.new_resize_protocol(img))
}

/// Match a changed-file path from a debounced notify event back to the
/// sample-card index it belongs to, by comparing file stems against
/// `card.tag` (the same key `card_svg_path` writes by).
fn card_index_for_path(cards: &[Card], path: &Path) -> Option<usize> {
    let stem = path.file_stem()?.to_str()?;
    cards.iter().position(|card| card.tag == stem)
}

/// `--watch <dir>`: load card SVGs from `dir` (falling back to generated
/// ones), watch `dir` with a debounced `notify` watcher, and on any `.svg`
/// change reload that card's markup and re-render the composed grid into
/// the real terminal — the design loop is: edit an SVG in an editor next to
/// this running process, and the terminal updates.
fn watch_mode(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let theme = Theme::temet();
    let cards = sample_cards();
    let mut svgs = load_card_svgs(dir, &cards, &theme);

    let (tx, rx) = channel::<DebounceEventResult>();
    let mut debouncer = new_debouncer(Duration::from_millis(300), None, tx)?;
    debouncer.watch(dir, RecursiveMode::NonRecursive)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let picker = Picker::from_query_stdio().unwrap_or_else(|_| headless_picker());
    let (_svg, page_w) = compose_grid_svg_from(&svgs, COLUMNS, &theme);
    let target_width_px = (page_w * 2.1) as u32;
    let mut protocol = build_grid_protocol(&svgs, &theme, &picker, target_width_px)?;

    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        loop {
            terminal.draw(|frame| {
                StatefulImage::default().render(frame.area(), frame.buffer_mut(), &mut protocol);
            })?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    use tui_kit::ratatui::crossterm::event::KeyCode;
                    if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                        return Ok(());
                    }
                }
            }

            let mut dirty = false;
            while let Ok(result) = rx.try_recv() {
                if let Ok(events) = result {
                    for event in events {
                        if let Some(idx) = event
                            .paths
                            .iter()
                            .find(|p| p.extension().is_some_and(|e| e == "svg"))
                            .and_then(|p| card_index_for_path(&cards, p))
                        {
                            svgs[idx] =
                                load_card_svgs(dir, std::slice::from_ref(&cards[idx]), &theme)
                                    .remove(0);
                            dirty = true;
                        }
                    }
                }
            }
            if dirty {
                protocol = build_grid_protocol(&svgs, &theme, &picker, target_width_px)?;
            }
        }
    })();

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    result
}

/// `--watch-selftest`: fully headless scripted proof of the hot-reload
/// loop. Dumps SVGs to a temp dir, starts the debounced watcher, renders
/// once via `TestBackend`, programmatically edits one SVG on disk, waits
/// for the debounced event, re-renders, and asserts the rendered buffer
/// changed. Prints `WATCH SELFTEST PASS` and returns `Ok(())` (exit 0) on
/// success; otherwise returns `Err` (exit non-zero) describing the failure.
fn watch_selftest_mode() -> Result<(), Box<dyn std::error::Error>> {
    let theme = Theme::temet();
    let cards = sample_cards();

    let dir = std::env::temp_dir().join(format!(
        "tui-kit-gallery-watch-selftest-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    dump_svg_mode(&dir)?;
    println!(
        "watch-selftest: dumped {} card SVGs to {}",
        cards.len(),
        dir.display()
    );

    let mut svgs = load_card_svgs(&dir, &cards, &theme);

    let (tx, rx) = channel::<DebounceEventResult>();
    let mut debouncer = new_debouncer(Duration::from_millis(200), None, tx)?;
    debouncer.watch(&dir, RecursiveMode::NonRecursive)?;

    let picker = headless_picker();
    let (_svg, page_w) = compose_grid_svg_from(&svgs, COLUMNS, &theme);
    let target_width_px = (page_w * 2.1) as u32;

    let mut terminal = Terminal::new(TestBackend::new(160, 60))?;
    let mut protocol = build_grid_protocol(&svgs, &theme, &picker, target_width_px)?;
    let frame1: Buffer = terminal
        .draw(|frame| {
            StatefulImage::default().render(frame.area(), frame.buffer_mut(), &mut protocol);
        })?
        .buffer
        .clone();
    println!("watch-selftest: frame 1 rendered");

    // Programmatically edit one card's SVG on disk: swap its title text
    // for a distinguishable edited string, exactly the "edit in an editor"
    // step a human would perform.
    let edited_card = &cards[0];
    let path = card_svg_path(&dir, edited_card);
    let original = std::fs::read_to_string(&path)?;
    let edited = original.replacen(&edited_card.title, "EDITED TITLE", 1);
    if edited == original {
        return Err(format!(
            "watch-selftest FAIL: card title {:?} not found verbatim in its own dumped SVG",
            edited_card.title
        )
        .into());
    }
    std::fs::write(&path, &edited)?;
    println!("watch-selftest: edited {} on disk", path.display());

    // Wait for the debounced event referencing the edited card, with a
    // generous overall timeout so a slow CI filesystem-watcher backend
    // doesn't flake this out.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut reloaded = false;
    while std::time::Instant::now() < deadline && !reloaded {
        if let Ok(Ok(events)) = rx.recv_timeout(Duration::from_millis(250)) {
            for event in events {
                if let Some(idx) = event
                    .paths
                    .iter()
                    .find(|p| p.extension().is_some_and(|e| e == "svg"))
                    .and_then(|p| card_index_for_path(&cards, p))
                {
                    svgs[idx] =
                        load_card_svgs(&dir, std::slice::from_ref(&cards[idx]), &theme).remove(0);
                    reloaded = true;
                }
            }
        }
    }
    if !reloaded {
        return Err(
            "watch-selftest FAIL: no debounced filesystem event observed within 10s".into(),
        );
    }
    println!("watch-selftest: debounced reload observed, re-rasterizing");

    let mut protocol2 = build_grid_protocol(&svgs, &theme, &picker, target_width_px)?;
    let frame2: Buffer = terminal
        .draw(|frame| {
            StatefulImage::default().render(frame.area(), frame.buffer_mut(), &mut protocol2);
        })?
        .buffer
        .clone();
    println!("watch-selftest: frame 2 rendered");

    let _ = std::fs::remove_dir_all(&dir);

    if frame1 == frame2 {
        return Err(
            "watch-selftest FAIL: rendered buffer identical before/after the SVG edit".into(),
        );
    }
    println!("WATCH SELFTEST PASS");
    Ok(())
}
