//! Putting the terminal back. The renderer owns the terminal while it runs
//! and restores it on every exit path it controls; a crash skips that, so the
//! host saves the line settings before each child and restores them, with
//! [`RESTORE`], after every exit.

use std::fs::File;
use std::io::{IsTerminal, Write};
use std::process::{Command, Stdio};

/// Mouse reporting off (every mode), focus and paste reports off, the
/// alternate screen left, the cursor shown, attributes reset.
pub const RESTORE: &str =
    "\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1004l\x1b[?2004l\x1b[0m\x1b[?1049l\x1b[?25h";

/// The terminal's line settings (`stty -g`), when there is a terminal.
pub struct TtyModes(Option<String>);

impl TtyModes {
    pub fn save() -> Self {
        let Ok(tty) = File::open("/dev/tty") else {
            return TtyModes(None);
        };
        let out = Command::new("stty")
            .arg("-g")
            .stdin(tty)
            .stderr(Stdio::null())
            .output();
        TtyModes(
            out.ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty()),
        )
    }

    /// Raw mode off and every mode a renderer may have turned on, off. Safe to repeat.
    pub fn restore(&self) {
        let mut out = std::io::stdout();
        if out.is_terminal() {
            let _ = out.write_all(RESTORE.as_bytes());
            let _ = out.flush();
        }
        if let (Some(saved), Ok(tty)) = (&self.0, File::open("/dev/tty")) {
            let _ = Command::new("stty")
                .arg(saved)
                .stdin(tty)
                .stderr(Stdio::null())
                .status();
        }
    }
}
