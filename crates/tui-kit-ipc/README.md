# tui-kit-ipc

For a Rust terminal app that wants its screen drawn by an
[Ink](https://github.com/vadimdemedes/ink) (React) app instead of in Rust. Your Rust
program keeps all state and decisions; it starts the Ink renderer as a child process,
sends it what to show, and gets back what the user pressed. When the renderer cannot run,
your program is told so and can draw in Rust (with Ratatui) instead, on the same state.

It is half of a pair: the Ink app uses [`tui-kit-ink`](../../ink), the other half. Both
live in [tui-kit](https://github.com/temet-ai/tui-kit) (MIT).

## What you need

- Rust 1.90 or later, on macOS or Linux (it uses a Unix socket and `stty`).
- Node 22 or later on the machine that runs your app.
- An Ink renderer built with `tui-kit-ink`, bundled to one `.js` file (its README shows
  how). Below it is `ink/dist/renderer.js`, a path relative to where your app runs.

```toml
tui-kit-ipc = { git = "https://github.com/temet-ai/tui-kit" }
```

## Using it

```rust,no_run
use tui_kit_ipc::{App, Ended, Flow, Options, Size};

struct Counter(u32);

impl App for Counter {
    type View = String;          // what the renderer draws: any Serialize type
    type Intent = String;        // what it sends back: any Deserialize type
    fn view(&self, size: Size) -> String { format!("{} at {}x{}", self.0, size.columns, size.rows) }
    fn intent(&mut self, i: String) -> Flow {
        if i == "quit" { return Flow::Quit }
        self.0 += 1;
        Flow::Continue
    }
}

// node, the variable the renderer reads the socket path from, your view version.
let options = Options::new("node", "MYAPP_IPC_SOCKET", 1).arg("ink/dist/renderer.js");
match tui_kit_ipc::run(&mut Counter(0), &options) {
    Ended::Quit => {}
    Ended::FallBack(why) => {
        eprintln!("{why}; drawing in this process instead");
        // run your own Ratatui loop on the same Counter
    }
}
```

`run` starts the renderer with the socket's path in the variable you named
(`MYAPP_IPC_SOCKET` here), and blocks until the app quits (`Flow::Quit` from `intent` or
`tick`, or the renderer says `bye`) or the renderer cannot go on
(`Ended::FallBack(String)`, the string a sentence saying why). Either way the renderer has
exited and the terminal is restored when `run` returns. `tick`, `log` and `notices` on
`App` have default bodies; `view` and `intent` are the two you write.
Everything happens on the calling thread: `view` is called after every message from the
renderer and every tick (`App::tick`, every 200 ms by default), and the result is sent
only when it differs from the last view sent.

The view version (the `1`) is yours: raise it when the view type changes in a way an older
renderer cannot draw. The renderer says its own number when it connects; a different
number is refused and not restarted.

## What it does for you

- **Gives the renderer the terminal.** Its stdin and stdout are the terminal's; the socket
  carries only data, and your program writes nothing to the terminal while it runs.
- **Sends whole views, one at a time.** Each view carries a cursor (a number that rises
  with every view); the renderer acknowledges it once drawn, and only then does the next
  go, holding the newest state at that moment. A fast-changing app cannot flood a slow
  renderer. Each intent carries the cursor of the view the user saw.
- **Restarts, then falls back.** A crash, a renderer that never connects, or one that stops
  reading is restarted. The fourth failure within a minute ends it with `Ended::FallBack`.
- **Puts the terminal back** after every exit, crashes included: mouse and paste reporting
  off, alternate screen left, cursor shown, line settings restored.

`Options` fields and their defaults:

| Field | Default | Meaning |
|---|---|---|
| `tick` | 200 ms | how often `App::tick` runs when nothing else happens |
| `connect_within` | 10 s | how long a started renderer has to connect |
| `stalled_after` | 2 s | how long the outgoing queue may stay full before the renderer counts as stopped |
| `max_queued` | 256 | lines that may wait to be written |
| `leave_within` | 2 s | how long a renderer told to quit has to exit before it is killed |
| `policy` | 3 restarts per 60 s | `RestartPolicy::new(max, window)` |
| `socket_dir` | the system temp folder | where the socket is made |

## The protocol

One JSON object per line over a Unix socket, at most 16 MiB a line, tagged by `type`:

| From | Messages |
|---|---|
| renderer | `hello {protocol, view, size}`, `resize`, `intent {cursor, intent}`, `ack {cursor}`, `log`, `bye` |
| host | `welcome`, `refuse {wanted, got, message}`, `snapshot {cursor, view}`, `notice {text}`, `shutdown` |

The TypeScript types for these messages are generated into
[`ink/src/generated/`](../../ink/src/generated) by this crate's tests, which fail when that
copy is out of date. From the repository root, `TUI_KIT_TS_UPDATE=1 cargo test -p
tui-kit-ipc` rewrites it. Export your own view and input types the same way with
[ts-rs](https://crates.io/crates/ts-rs).

## Status

Early: the protocol is version 1 and may change while tui-kit has no releases.
