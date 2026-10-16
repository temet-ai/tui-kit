//! The supervisor over a real socket and a real child: this test binary run
//! again as a fake renderer (`fake_renderer`, which does nothing unless
//! FAKE_MODE is set). Each start appends a line to FAKE_STARTS, so a test can
//! count restarts.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;
use tui_kit_ipc::{App, Ended, Flow, Options, RestartPolicy, Size, run};

#[test]
fn fake_renderer() {
    let Ok(mode) = std::env::var("FAKE_MODE") else {
        return;
    };
    let starts = std::env::var("FAKE_STARTS").unwrap();
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(starts)
        .unwrap();
    writeln!(f, "{mode}").unwrap();
    let mut s = UnixStream::connect(std::env::var("FAKE_SOCKET").unwrap()).unwrap();
    if mode == "crash" {
        std::process::exit(1);
    }
    let mut lines = BufReader::new(s.try_clone().unwrap()).lines();
    let mut next =
        || -> serde_json::Value { serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap() };
    let mut say = |v: serde_json::Value| writeln!(s, "{v}").unwrap();
    let view = if mode == "old" { 6 } else { 7 };
    say(
        serde_json::json!({"type": "hello", "protocol": 1, "view": view, "size": {"columns": 80, "rows": 24}}),
    );
    if next()["type"] == "refuse" {
        std::process::exit(3);
    }
    let first = next();
    assert_eq!(first["view"], "0 at 80x24", "{first}");
    say(serde_json::json!({"type": "ack", "cursor": first["cursor"]}));
    if mode == "bye" {
        say(serde_json::json!({"type": "bye"}));
        std::process::exit(0);
    }
    say(serde_json::json!({"type": "intent", "cursor": first["cursor"], "intent": "inc"}));
    let second = next();
    if second["view"] != "1 at 80x24" {
        std::process::exit(4);
    }
    say(serde_json::json!({"type": "ack", "cursor": second["cursor"]}));
    say(serde_json::json!({"type": "intent", "cursor": second["cursor"], "intent": "quit"}));
    while next()["type"] != "shutdown" {}
    writeln!(f, "shutdown").unwrap();
    std::process::exit(0);
}

#[derive(Default)]
struct Counter(u32);

impl App for Counter {
    type View = String;
    type Intent = String;
    fn view(&self, size: Size) -> String {
        format!("{} at {}x{}", self.0, size.columns, size.rows)
    }
    fn intent(&mut self, intent: String) -> Flow {
        match intent.as_str() {
            "inc" => self.0 += 1,
            "quit" => return Flow::Quit,
            _ => {}
        }
        Flow::Continue
    }
}

fn with_fake(mode: &str) -> (Ended, Counter, Vec<String>) {
    let dir = std::env::temp_dir().join(format!("tui-kit-sv-{mode}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let starts = dir.join("starts");
    // SAFETY: each test sets the variables before starting its children, and
    // nothing in this binary reads them except the fake renderer.
    unsafe {
        std::env::set_var("FAKE_STARTS", &starts);
    }
    let mut options = Options::new(std::env::current_exe().unwrap(), "FAKE_SOCKET", 7)
        .arg("fake_renderer")
        .arg("--exact")
        .arg("--nocapture");
    options.socket_dir = dir.clone();
    options.tick = Duration::from_millis(50);
    options.leave_within = Duration::from_millis(500);
    options.policy = RestartPolicy::new(2, Duration::from_secs(60));
    // The child takes FAKE_MODE from its environment.
    let mut app = Counter::default();
    let ended = {
        let mut o = options;
        o.args.insert(0, "--test-threads=1".into());
        unsafe {
            std::env::set_var("FAKE_MODE", mode);
        }
        let r = run(&mut app, &o);
        unsafe {
            std::env::remove_var("FAKE_MODE");
        }
        r
    };
    let lines = std::fs::read_to_string(&starts)
        .unwrap_or_default()
        .lines()
        .map(String::from)
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    (ended, app, lines)
}

/// One test drives every case in turn: the cases share the process
/// environment the fake reads, so they must not run at once.
#[test]
fn the_supervisor_runs_restarts_refuses_and_quits() {
    let (ended, app, starts) = with_fake("quit");
    assert_eq!(ended, Ended::Quit);
    assert_eq!(
        app.0, 1,
        "the intent reached the app and the next snapshot showed it"
    );
    assert_eq!(
        starts,
        ["quit", "shutdown"],
        "the renderer read Shutdown before the socket closed"
    );

    let (ended, _, starts) = with_fake("bye");
    assert_eq!(ended, Ended::Quit);
    assert_eq!(starts, ["bye"]);

    let (ended, _, starts) = with_fake("crash");
    assert!(
        matches!(&ended, Ended::FallBack(m) if m.contains("stopped 3 times")),
        "{ended:?}"
    );
    assert_eq!(starts.len(), 3, "two restarts, then the fallback");

    let (ended, _, starts) = with_fake("old");
    assert!(
        matches!(&ended, Ended::FallBack(m) if m.contains("refused") && m.contains("view 6")),
        "{ended:?}"
    );
    assert_eq!(starts, ["old"], "a refused renderer is not restarted");
}
