//! A terminal UI's renderer in another process.
//!
//! The host owns all state and decisions; a renderer (the Ink package in
//! `ink/`, or anything that speaks this protocol) owns the terminal while it
//! runs and draws the views the host sends. Taken from radar, where the same
//! split runs an Ink/React renderer beside the Ratatui one.
//!
//! - Transport: a Unix socket the host listens on; its path is in an
//!   environment variable the child reads. The child's stdin, stdout and
//!   stderr are the terminal's: the socket carries only data.
//! - Framing: one JSON object per `\n`-terminated UTF-8 line, at most
//!   [`MAX_LINE`] bytes, tagged by `"type"` in snake case.
//! - Versions: [`PROTOCOL`] for this envelope, and the app's own view version.
//!   A renderer whose versions differ is refused and not restarted.
//! - Flow: whole-state snapshots, one in flight until the renderer acks it;
//!   the next is the newest state, so ticks coalesce to the renderer's pace
//!   ([`Bridge`]). Intents carry the cursor of the snapshot they were seen at.
//! - Supervision: connect timeout, a bounded write queue with a stall timer,
//!   restarts under a [`RestartPolicy`], then [`Ended::FallBack`] so the app can
//!   carry on with an in-process renderer on the same state, and the terminal
//!   put back after every child exit ([`supervisor`], [`tty`]).

pub mod bridge;
pub mod supervisor;
pub mod tty;

pub use bridge::{Bridge, Received};
pub use supervisor::{App, Ended, Flow, Options, RestartPolicy, run};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The envelope's version. The app's view types carry their own.
pub const PROTOCOL: u32 = 1;

/// The longest line either side accepts, in bytes.
pub const MAX_LINE: usize = 16 * 1024 * 1024;

/// The terminal's size in cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Size {
    pub columns: u16,
    pub rows: u16,
}

/// What a renderer sends the host. `I` is the app's input type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToHost<I> {
    /// First message: the versions the renderer speaks and the terminal's size.
    Hello {
        protocol: u32,
        view: u32,
        size: Size,
    },
    Resize {
        size: Size,
    },
    /// The user did something. `cursor` is the newest snapshot the renderer had.
    Intent {
        cursor: u64,
        intent: I,
    },
    /// The snapshot with this cursor is drawn; send the next.
    Ack {
        cursor: u64,
    },
    Log {
        level: String,
        text: String,
    },
    /// The renderer is leaving on its own (the user quit in it).
    Bye,
}

/// What the host sends a renderer. `V` is the app's view type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToRenderer<V> {
    Welcome {
        protocol: u32,
        view: u32,
        cursor: u64,
    },
    /// The renderer's versions are not the host's; it exits and is not restarted.
    Refuse {
        wanted: u32,
        got: u32,
        message: String,
    },
    Snapshot {
        cursor: u64,
        view: V,
    },
    /// A line for the status bar; never dropped or coalesced.
    Notice {
        text: String,
    },
    Shutdown,
}

/// One message as its line.
pub fn encode<T: Serialize>(message: &T) -> String {
    let mut line = serde_json::to_string(message).expect("messages serialise");
    line.push('\n');
    line
}

#[derive(Debug, PartialEq, Eq)]
pub struct TooLong;

/// Reassembles lines from reads of any size.
#[derive(Default)]
pub struct Lines {
    held: Vec<u8>,
}

impl Lines {
    /// The whole lines `bytes` completes, without their `\n`.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, TooLong> {
        let mut out = Vec::new();
        for &b in bytes {
            if b == b'\n' {
                let line = std::mem::take(&mut self.held);
                if !line.is_empty() {
                    out.push(String::from_utf8_lossy(&line).into_owned());
                }
            } else {
                if self.held.len() >= MAX_LINE {
                    return Err(TooLong);
                }
                self.held.push(b);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
