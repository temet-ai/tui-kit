//! Runnable demo of `tui_kit::shader`'s temet plasma/scanline backdrop: a
//! full-screen `ShaderCanvas` running GPU-side, with a title line drawn on
//! top. `q`/Esc quits.
//!
//! Requires the `shader` cargo feature (which pulls in `wgpu` via
//! `tui-shader`, kept out of the default build):
//! `cargo run --example shader --features shader`.

use std::io;
use std::time::Duration;

use tui_kit::ratatui::Terminal;
use tui_kit::ratatui::backend::CrosstermBackend;
use tui_kit::ratatui::crossterm::event::{self, Event, KeyCode};
use tui_kit::ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use tui_kit::ratatui::crossterm::{ExecutableCommand, execute};
use tui_kit::ratatui::layout::Alignment;
use tui_kit::ratatui::text::Line;
use tui_kit::ratatui::widgets::Paragraph;
use tui_kit::shader::{temet_canvas, temet_state};
use tui_kit::theme::Theme;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A panicking draw must not leave the terminal stuck in raw mode / the
    // alt screen with no way back to the shell, so restore both *before*
    // the default panic hook prints — not just on the normal-return path
    // below. Same pattern as `examples/console.rs`/`examples/gallery.rs`.
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

    let result = run(&mut terminal);

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
    let theme = Theme::temet();
    let mut state = temet_state();

    loop {
        terminal.draw(|frame| {
            // The shader canvas fills the whole frame as a backdrop; the
            // title line renders as an ordinary native-cell widget on top
            // of it, sharing the same `Theme::temet()` values so nothing
            // here is a color literal either.
            frame.render_stateful_widget(temet_canvas(), frame.area(), &mut state);

            let title = Paragraph::new(Line::from("T E M E T — shader backdrop · q / Esc quits"))
                .style(tui_kit::ratatui::style::Style::new().fg(theme.text))
                .alignment(Alignment::Center);
            frame.render_widget(title, frame.area());
        })?;

        if event::poll(Duration::from_millis(33))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}
