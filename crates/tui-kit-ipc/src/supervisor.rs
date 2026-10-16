//! Runs the renderer as a child process and keeps it running: starts it on a
//! Unix socket, drives the [`Bridge`] from its messages and the app's ticks,
//! restores the terminal whenever it exits, restarts it under the
//! [`RestartPolicy`], and says when the app must carry on without it.
//!
//! The child owns the terminal from when it starts until it exits; this
//! process writes nothing to the terminal in that time.

use crate::bridge::{Bridge, Received};
use crate::tty::TtyModes;
use crate::{Lines, Size, ToHost};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::ffi::OsString;
use std::io::{ErrorKind, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::Child;
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender, TrySendError};
use std::time::{Duration, Instant};

/// The app as the supervisor sees it: a view to send, intents to take.
pub trait App {
    type View: Serialize + PartialEq + Clone;
    type Intent: DeserializeOwned;
    fn view(&self, size: Size) -> Self::View;
    fn intent(&mut self, intent: Self::Intent) -> Flow;
    /// Every [`Options::tick`] while nothing else happens.
    fn tick(&mut self) -> Flow {
        Flow::Continue
    }
    /// A line the renderer logged (it cannot write to the terminal it draws on).
    fn log(&mut self, _level: &str, _text: &str) {}
    /// Status lines for the renderer since the last call; each is sent once.
    fn notices(&mut self) -> Vec<String> {
        vec![]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Quit,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Ended {
    Quit,
    /// The renderer cannot be used: carry on with another one on the same
    /// state, and say this line once where the user sees it.
    FallBack(String),
}

/// At most `max` restarts in any `window`; one more crash ends the renderer.
#[derive(Clone, Debug)]
pub struct RestartPolicy {
    pub max: usize,
    pub window: Duration,
    crashes: Vec<Instant>,
}

impl RestartPolicy {
    pub fn new(max: usize, window: Duration) -> Self {
        RestartPolicy {
            max,
            window,
            crashes: Vec::new(),
        }
    }

    /// A crash at `now`: whether the renderer may start again.
    pub fn allows(&mut self, now: Instant) -> bool {
        let window = self.window;
        self.crashes.retain(|t| now.duration_since(*t) < window);
        self.crashes.push(now);
        self.crashes.len() <= self.max
    }
}

impl Default for RestartPolicy {
    fn default() -> Self {
        RestartPolicy::new(3, Duration::from_secs(60))
    }
}

pub struct Options {
    pub program: OsString,
    pub args: Vec<OsString>,
    /// The variable the child reads the socket path from.
    pub socket_env: String,
    /// The app's view version.
    pub view: u32,
    pub socket_dir: PathBuf,
    pub tick: Duration,
    pub connect_within: Duration,
    /// How long a renderer told to leave has before it is ended.
    pub leave_within: Duration,
    /// How long the write queue may stay full before the renderer counts as
    /// having stopped reading. A full queue alone is not enough: the writer
    /// thread may not have had its turn.
    pub stalled_after: Duration,
    pub max_queued: usize,
    pub policy: RestartPolicy,
}

impl Options {
    pub fn new(program: impl Into<OsString>, socket_env: impl Into<String>, view: u32) -> Self {
        Options {
            program: program.into(),
            args: Vec::new(),
            socket_env: socket_env.into(),
            view,
            socket_dir: std::env::temp_dir(),
            tick: Duration::from_millis(200),
            connect_within: Duration::from_secs(10),
            leave_within: Duration::from_secs(2),
            stalled_after: Duration::from_secs(2),
            max_queued: 256,
            policy: RestartPolicy::default(),
        }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }
}

enum Run {
    Quit,
    Crashed(String),
    Refused(String),
}

enum Ev {
    Line(String),
    Closed(String),
}

/// Runs the renderer until the app quits or the renderer cannot go on.
pub fn run<A: App>(app: &mut A, options: &Options) -> Ended {
    let mut bridge = Bridge::new(options.view);
    let mut policy = options.policy.clone();
    let mut started = 0;
    loop {
        started += 1;
        let modes = TtyModes::save();
        let outcome = run_once(app, options, &mut bridge, started);
        modes.restore();
        match outcome {
            Run::Quit => return Ended::Quit,
            Run::Refused(why) => {
                return Ended::FallBack(format!("the renderer was refused: {why}"));
            }
            Run::Crashed(why) => {
                if !policy.allows(Instant::now()) {
                    return Ended::FallBack(format!(
                        "the renderer stopped {} times in {} s ({why})",
                        policy.max + 1,
                        policy.window.as_secs()
                    ));
                }
                bridge.reconnect();
            }
        }
    }
}

fn socket_path(options: &Options, n: usize) -> PathBuf {
    let name = format!("tui-kit-{}-{n}.sock", std::process::id());
    let path = options.socket_dir.join(&name);
    // A socket path holds about 100 bytes.
    if path.as_os_str().len() < 100 {
        path
    } else {
        PathBuf::from("/tmp").join(name)
    }
}

fn run_once<A: App>(app: &mut A, options: &Options, bridge: &mut Bridge<A::View>, n: usize) -> Run {
    let path = socket_path(options, n);
    let _ = std::fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => return Run::Crashed(format!("cannot listen on {}: {e}", path.display())),
    };
    let _ = listener.set_nonblocking(true);
    let child = std::process::Command::new(&options.program)
        .args(&options.args)
        .env(&options.socket_env, &path)
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::remove_file(&path);
            return Run::Crashed(format!(
                "cannot start {}: {e}",
                options.program.to_string_lossy()
            ));
        }
    };
    let stream = accept(&listener, &mut child, options.connect_within);
    let _ = std::fs::remove_file(&path);
    let stream = match stream {
        Ok(s) => s,
        Err(why) => {
            end(&mut child, Duration::ZERO);
            return Run::Crashed(why);
        }
    };
    let outcome = converse(app, options, bridge, &stream, &mut child);
    let leave = match outcome {
        Run::Crashed(_) => Duration::ZERO,
        _ => options.leave_within,
    };
    // The child first: closing the socket before it has read Shutdown or
    // Refuse would make it leave as if the host had died.
    end(&mut child, leave);
    let _ = stream.shutdown(Shutdown::Both);
    outcome
}

fn accept(
    listener: &UnixListener,
    child: &mut Child,
    within: Duration,
) -> Result<UnixStream, String> {
    let started = Instant::now();
    loop {
        match listener.accept() {
            Ok((s, _)) => {
                let _ = s.set_nonblocking(false);
                return Ok(s);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {}
            Err(e) => return Err(format!("accept: {e}")),
        }
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("it exited before connecting ({status})"));
        }
        if started.elapsed() > within {
            return Err(format!("it did not connect within {} s", within.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn converse<A: App>(
    app: &mut A,
    options: &Options,
    bridge: &mut Bridge<A::View>,
    stream: &UnixStream,
    child: &mut Child,
) -> Run {
    let (events_tx, events) = mpsc::channel();
    let Ok(mut reader) = stream.try_clone() else {
        return Run::Crashed("cannot read the socket".into());
    };
    std::thread::spawn(move || {
        let mut lines = Lines::default();
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let ev = match reader.read(&mut buf) {
                Ok(0) => Ev::Closed("closed".into()),
                Ok(k) => match lines.push(&buf[..k]) {
                    Ok(ls) => {
                        for l in ls {
                            if events_tx.send(Ev::Line(l)).is_err() {
                                return;
                            }
                        }
                        continue;
                    }
                    Err(_) => Ev::Closed("sent a line over MAX_LINE".into()),
                },
                Err(e) => Ev::Closed(format!("failed: {e}")),
            };
            let _ = events_tx.send(ev);
            return;
        }
    });
    let (queue, outgoing) = mpsc::sync_channel::<String>(options.max_queued);
    let Ok(mut writer) = stream.try_clone() else {
        return Run::Crashed("cannot write the socket".into());
    };
    std::thread::spawn(move || {
        for line in outgoing {
            if writer.write_all(line.as_bytes()).is_err() {
                return;
            }
        }
    });

    let mut size = Size::default();
    let mut last_tick = Instant::now();
    loop {
        let mut flow = Flow::Continue;
        match events.recv_timeout(options.tick.saturating_sub(last_tick.elapsed())) {
            Ok(Ev::Line(line)) => {
                let message: ToHost<A::Intent> = match serde_json::from_str(&line) {
                    Ok(m) => m,
                    Err(e) => return Run::Crashed(format!("it sent an unreadable message: {e}")),
                };
                match bridge.receive(message) {
                    Received::Ready(s) | Received::Resize(s) => size = s,
                    Received::Refused(why) => {
                        let _ = send(&queue, bridge.take(), options.stalled_after);
                        return Run::Refused(why);
                    }
                    Received::Intent(i) => flow = app.intent(i),
                    Received::Log { level, text } => app.log(&level, &text),
                    Received::Bye => return Run::Quit,
                    Received::Nothing => {}
                }
            }
            Ok(Ev::Closed(why)) => {
                let status = child.wait().map(|s| s.to_string()).unwrap_or_default();
                return Run::Crashed(format!("the connection {why} ({status})"));
            }
            Err(RecvTimeoutError::Timeout) => {
                last_tick = Instant::now();
                flow = app.tick();
                if let Ok(Some(status)) = child.try_wait() {
                    return Run::Crashed(format!("it exited ({status})"));
                }
            }
            Err(RecvTimeoutError::Disconnected) => return Run::Crashed("its reader ended".into()),
        }
        if flow == Flow::Quit {
            bridge.shutdown();
            let _ = send(&queue, bridge.take(), options.stalled_after);
            return Run::Quit;
        }
        for text in app.notices() {
            bridge.notice(text);
        }
        bridge.offer(&app.view(size));
        if let Err(why) = send(&queue, bridge.take(), options.stalled_after) {
            return Run::Crashed(why);
        }
    }
}

fn send(
    queue: &SyncSender<String>,
    lines: Vec<String>,
    stalled_after: Duration,
) -> Result<(), String> {
    for mut line in lines {
        let started = Instant::now();
        loop {
            match queue.try_send(line) {
                Ok(()) => break,
                Err(TrySendError::Disconnected(_)) => return Err("its connection closed".into()),
                Err(TrySendError::Full(back)) => {
                    if started.elapsed() > stalled_after {
                        return Err(format!(
                            "it stopped reading for {} s",
                            stalled_after.as_secs()
                        ));
                    }
                    line = back;
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }
    Ok(())
}

/// Waits up to `within` for the child to exit, then ends it.
fn end(child: &mut Child, within: Duration) {
    let started = Instant::now();
    while started.elapsed() < within {
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    let _ = child.wait();
}
