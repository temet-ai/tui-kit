//! Runnable demo of `ConsoleShell`: native-cell chrome (brand header, `Tabs`,
//! `StatusBar`) composed around the image-based `CardGrid` into one
//! full-screen TUI. `1`/`2`/`3` switch tabs, arrows/hjkl move card focus,
//! `q`/Esc quits.
//!
//! `cargo run --example console -- --selftest` stands up the real
//! `ConsoleShell`/`ConsoleState`/`handle_console_key` path headlessly (a
//! `TestBackend`, no TTY) with a headless `Picker::from_fontsize` (same
//! pattern as `examples/gallery.rs`), asserts the chrome text is actually in
//! the rendered buffer, scripts a tab switch and a card-nav sequence, and
//! prints `CONSOLE SELFTEST PASS` on success.

use std::io;
use std::time::Duration;

use tui_kit::ratatui::backend::{CrosstermBackend, TestBackend};
use tui_kit::ratatui::buffer::{Buffer, Cell};
use tui_kit::ratatui::crossterm::event::{self, Event};
use tui_kit::ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui_kit::ratatui::crossterm::{execute, ExecutableCommand};
use tui_kit::ratatui::Terminal;
use tui_kit::ratatui_image::picker::Picker;
use tui_kit::{
    handle_console_key, Card, ConsoleAction, ConsoleShell, ConsoleState, ImageCache, NavAction,
    Theme,
};

// Deprecated constructor kept for deterministic 8x16 pixel output in
// headless paths: `Picker::halfblocks()` pins 10x20 and would change
// rendered output.
#[allow(deprecated)]
fn headless_picker() -> Picker {
    Picker::from_fontsize((8, 16).into())
}

/// Grid columns for each tab's `CardGrid` — 2, so every 3-card tab wraps to
/// a second row and both horizontal (Right) and vertical (Down) card-nav
/// keys have somewhere to land.
const COLUMNS: usize = 2;

/// `1-3 tab · ⇥ panes · j/k move`, the `StatusBar`'s right-hand key hints.
const KEY_HINTS: [(&str, &str); 3] = [("1-3", "tab"), ("\u{21e5}", "panes"), ("j/k", "move")];

/// Three tabs' worth of temet cards, in `tui_kit::console::TAB_TITLES`
/// order (proc, pods, api) — distinct content per tab, 3 cards each so
/// every tab lays out as a 2-column, 2-row grid under `COLUMNS`.
fn tab_cards() -> Vec<Vec<Card>> {
    vec![
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
                tag: "proc".to_string(),
                title: "Zombie reaper".to_string(),
                subtitle: Some("0 zombies · last swept 4m ago".to_string()),
                bullets: vec![
                    "SIGCHLD handler armed".to_string(),
                    "Reparents to init on exit".to_string(),
                ],
                action: Some("Sweep".to_string()),
                hotkey: Some("z".to_string()),
            },
            Card {
                tag: "proc".to_string(),
                title: "Cgroup limits".to_string(),
                subtitle: Some("cpu.max 4.0 · mem.max 8G".to_string()),
                bullets: vec![
                    "OOM score tracked per unit".to_string(),
                    "Throttling events logged".to_string(),
                ],
                action: Some("Tune".to_string()),
                hotkey: Some("c".to_string()),
            },
        ],
        vec![
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
                tag: "pods".to_string(),
                title: "Ingress routes".to_string(),
                subtitle: Some("6 hosts · TLS auto-renewed".to_string()),
                bullets: vec![
                    "Canary weight 10%".to_string(),
                    "Sticky sessions on".to_string(),
                ],
                action: Some("Route".to_string()),
                hotkey: Some("r".to_string()),
            },
            Card {
                tag: "pods".to_string(),
                title: "Node drain queue".to_string(),
                subtitle: Some("1 draining · 0 cordoned".to_string()),
                bullets: vec![
                    "PodDisruptionBudget respected".to_string(),
                    "Drain timeout 90s".to_string(),
                ],
                action: Some("Drain".to_string()),
                hotkey: Some("n".to_string()),
            },
        ],
        vec![
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
                tag: "api".to_string(),
                title: "Auth tokens".to_string(),
                subtitle: Some("JWT · 15m TTL".to_string()),
                bullets: vec![
                    "Refresh rotation enabled".to_string(),
                    "Revocation list synced".to_string(),
                ],
                action: Some("Rotate".to_string()),
                hotkey: Some("t".to_string()),
            },
            Card {
                tag: "api".to_string(),
                title: "Webhook delivery".to_string(),
                subtitle: Some("99.4% delivered · 3 retrying".to_string()),
                bullets: vec![
                    "Exponential backoff armed".to_string(),
                    "Dead-letter queue watched".to_string(),
                ],
                action: Some("Retry".to_string()),
                hotkey: Some("w".to_string()),
            },
        ],
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--selftest") {
        return selftest_mode();
    }
    interactive_mode()
}

fn interactive_mode() -> Result<(), Box<dyn std::error::Error>> {
    // A panicking draw (e.g. inside CardGrid/ratatui-image) must not leave
    // the terminal stuck in raw mode / the alt screen with no way back to
    // the shell, so restore both *before* the default panic hook prints —
    // not just on the normal-return path below.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        default_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::temet();
    let cards = tab_cards();
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| headless_picker());
    let mut cache = ImageCache::new();
    let counts: Vec<usize> = cards.iter().map(Vec::len).collect();
    let mut state = ConsoleState::new(&counts, COLUMNS);

    let result = run(
        &mut terminal,
        &cards,
        &theme,
        &picker,
        &mut cache,
        &mut state,
    );

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    tab_cards: &[Vec<Card>],
    theme: &Theme,
    picker: &Picker,
    cache: &mut ImageCache,
    state: &mut ConsoleState,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| {
            let shell = ConsoleShell::new(
                tab_cards,
                theme,
                picker,
                cache,
                "normal",
                "console ready",
                &KEY_HINTS,
            );
            frame.render_stateful_widget(shell, frame.area(), state);
        })?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if let Some(ConsoleAction::Quit) = handle_console_key(state, key) {
                    return Ok(());
                }
            }
        }
    }
}

/// Render one frame of the real `ConsoleShell` `StatefulWidget` into
/// `terminal` and return a clone of the resulting buffer (so callers can
/// both count non-default cells and search rendered text after the borrow
/// on `terminal` ends). Wrapped in `catch_unwind` so a panic inside
/// `ConsoleShell`/`CardGrid`/`ratatui-image` is reported as a clear failure
/// instead of an opaque abort.
#[allow(clippy::too_many_arguments)]
fn render_shell_headless(
    terminal: &mut Terminal<TestBackend>,
    tab_cards: &[Vec<Card>],
    theme: &Theme,
    picker: &Picker,
    cache: &mut ImageCache,
    mode: &str,
    message: &str,
    state: &mut ConsoleState,
) -> Result<Buffer, Box<dyn std::error::Error>> {
    use std::panic::{self, AssertUnwindSafe};

    let result: Result<Buffer, std::convert::Infallible> =
        panic::catch_unwind(AssertUnwindSafe(|| {
            let drawn = terminal.draw(|frame| {
                let shell =
                    ConsoleShell::new(tab_cards, theme, picker, cache, mode, message, &KEY_HINTS);
                frame.render_stateful_widget(shell, frame.area(), state);
            })?;
            Ok(drawn.buffer.clone())
        }))
        .map_err(|e| -> Box<dyn std::error::Error> {
            let context = e
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| e.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "<non-string panic payload>".to_string());
            format!("ConsoleShell render panicked inside TestBackend: {context}").into()
        })?;

    Ok(result?)
}

/// Every row of `buf`, with spaces stripped, concatenated with `\n`
/// separators — so a search for letter-spaced chrome text (e.g. the brand
/// `"T E M E T"`) can still find it contiguously (`"TEMET"`), while a search
/// for ordinary text still matches unchanged (stripping spaces from "PROC"
/// or "NORMAL" doesn't alter them).
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
    buf.content()
        .iter()
        .filter(|cell| **cell != Cell::default())
        .count()
}

/// `--selftest`: stand up the REAL `ConsoleShell` + `ConsoleState` +
/// `handle_console_key` path headlessly and prove it renders real chrome
/// text and navigates without panicking.
fn selftest_mode() -> Result<(), Box<dyn std::error::Error>> {
    use tui_kit::ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let theme = Theme::temet();
    let cards = tab_cards();
    let mut cache = ImageCache::new();
    let picker = headless_picker();
    let counts: Vec<usize> = cards.iter().map(Vec::len).collect();
    let mut state = ConsoleState::new(&counts, COLUMNS);

    println!(
        "selftest: {} tabs ({:?} cards), Picker protocol = {:?}",
        counts.len(),
        counts,
        picker.protocol_type()
    );

    let mut terminal = Terminal::new(TestBackend::new(140, 44))?;

    let buf = render_shell_headless(
        &mut terminal,
        &cards,
        &theme,
        &picker,
        &mut cache,
        "normal",
        "console ready",
        &mut state,
    )?;
    let nd = non_default_cells(&buf);
    if nd == 0 {
        return Err("selftest FAIL: rendered TestBackend buffer has no non-default cells".into());
    }
    let text = buffer_text_stripped(&buf);
    for needle in ["TEMET", "PROC", "PODS", "API", "NORMAL"] {
        if !text.contains(needle) {
            return Err(format!(
                "selftest FAIL: rendered buffer is missing {needle:?}\n---\n{text}"
            )
            .into());
        }
    }
    println!("selftest: frame 1 rendered via the real ConsoleShell StatefulWidget ({nd} non-default cells)");
    println!("selftest: chrome text present: brand TEMET, tab labels PROC/PODS/API, StatusBar mode NORMAL");

    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::NONE);

    let action = handle_console_key(&mut state, key(KeyCode::Char('2')));
    println!(
        "selftest: '2'    -> {action:?} (active_tab now {})",
        state.active_tab
    );
    assert_eq!(
        action,
        Some(ConsoleAction::Tab(1)),
        "'2' must switch to tab index 1"
    );
    assert_eq!(state.active_tab, 1);

    let before = state.navs[1].selected;
    let action = handle_console_key(&mut state, key(KeyCode::Right));
    println!(
        "selftest: Right  -> {action:?} (selected {before} -> {})",
        state.navs[1].selected
    );
    assert!(
        matches!(action, Some(ConsoleAction::Nav(NavAction::Focus(_)))),
        "Right must move focus"
    );
    assert_ne!(
        state.navs[1].selected, before,
        "Right must change the active tab's card selection"
    );

    let before = state.navs[1].selected;
    let action = handle_console_key(&mut state, key(KeyCode::Down));
    println!(
        "selftest: Down   -> {action:?} (selected {before} -> {})",
        state.navs[1].selected
    );
    assert!(
        matches!(action, Some(ConsoleAction::Nav(NavAction::Focus(_)))),
        "Down must move focus"
    );
    assert_ne!(
        state.navs[1].selected, before,
        "Down must change the active tab's card selection"
    );

    let action = handle_console_key(&mut state, key(KeyCode::Char('q')));
    println!("selftest: 'q'    -> {action:?}");
    assert_eq!(action, Some(ConsoleAction::Quit), "'q' must quit");

    let buf2 = render_shell_headless(
        &mut terminal,
        &cards,
        &theme,
        &picker,
        &mut cache,
        "normal",
        "console ready",
        &mut state,
    )?;
    let nd2 = non_default_cells(&buf2);
    if nd2 == 0 {
        return Err(
            "selftest FAIL: post-switch TestBackend buffer has no non-default cells".into(),
        );
    }
    let text2 = buffer_text_stripped(&buf2);
    if !text2.contains("PODS") {
        return Err("selftest FAIL: post-switch frame is missing the PODS tab label".into());
    }
    println!(
        "selftest: frame 2 (post-switch, active tab now {}) rendered ({nd2} non-default cells)",
        state.active_tab
    );

    println!("CONSOLE SELFTEST PASS");
    Ok(())
}
