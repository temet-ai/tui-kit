use super::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[test]
fn lines_come_whole_from_reads_of_any_size() {
    let mut l = Lines::default();
    assert_eq!(l.push(b"{\"a\":1}\n{\"b\"").unwrap(), vec!["{\"a\":1}"]);
    assert_eq!(
        l.push(b":2}\n\n{\"c\":3}\n").unwrap(),
        vec!["{\"b\":2}", "{\"c\":3}"]
    );
    assert!(l.push(b"").unwrap().is_empty());
}

#[test]
fn a_line_over_the_limit_is_refused() {
    let mut l = Lines::default();
    assert_eq!(l.push(&vec![b'x'; MAX_LINE + 1]), Err(TooLong));
}

#[test]
fn messages_are_tagged_by_type_in_snake_case() {
    let hello: ToHost<String> = ToHost::Hello {
        protocol: 1,
        view: 2,
        size: Size {
            columns: 80,
            rows: 24,
        },
    };
    assert_eq!(
        encode(&hello),
        "{\"type\":\"hello\",\"protocol\":1,\"view\":2,\"size\":{\"columns\":80,\"rows\":24}}\n"
    );
    let intent: ToHost<String> =
        serde_json::from_str(r#"{"type":"intent","cursor":3,"intent":"up"}"#).unwrap();
    assert_eq!(
        intent,
        ToHost::Intent {
            cursor: 3,
            intent: "up".into()
        }
    );
}

fn ready(b: &mut Bridge<String>) {
    let r = b.receive::<()>(ToHost::Hello {
        protocol: PROTOCOL,
        view: 7,
        size: Size {
            columns: 80,
            rows: 24,
        },
    });
    assert_eq!(
        r,
        Received::Ready(Size {
            columns: 80,
            rows: 24
        })
    );
    assert!(b.take()[0].starts_with("{\"type\":\"welcome\""));
}

#[test]
fn another_view_version_is_refused() {
    let mut b: Bridge<String> = Bridge::new(7);
    let r = b.receive::<()>(ToHost::Hello {
        protocol: PROTOCOL,
        view: 6,
        size: Size::default(),
    });
    assert!(matches!(r, Received::Refused(m) if m.contains("view 6") && m.contains("view 7")));
    assert!(b.take()[0].starts_with("{\"type\":\"refuse\",\"wanted\":7,\"got\":6"));
    b.offer(&"a".into());
    assert!(b.take().is_empty(), "nothing is sent to a refused renderer");
}

#[test]
fn one_snapshot_is_in_flight_and_the_next_is_the_newest() {
    let mut b: Bridge<String> = Bridge::new(7);
    b.offer(&"early".into());
    assert!(b.take().is_empty(), "nothing before the handshake");
    ready(&mut b);
    b.offer(&"a".into());
    assert_eq!(
        b.take(),
        vec!["{\"type\":\"snapshot\",\"cursor\":1,\"view\":\"a\"}\n"]
    );
    b.offer(&"b".into());
    b.offer(&"c".into());
    assert!(b.take().is_empty(), "held until the ack");
    b.receive::<()>(ToHost::Ack { cursor: 1 });
    b.offer(&"c".into());
    assert_eq!(
        b.take(),
        vec!["{\"type\":\"snapshot\",\"cursor\":2,\"view\":\"c\"}\n"]
    );
    b.receive::<()>(ToHost::Ack { cursor: 2 });
    b.offer(&"c".into());
    assert!(b.take().is_empty(), "an unchanged view is not sent again");
}

#[test]
fn a_new_renderer_is_told_everything_and_the_cursor_keeps_rising() {
    let mut b: Bridge<String> = Bridge::new(7);
    ready(&mut b);
    b.offer(&"a".into());
    b.take();
    b.reconnect();
    ready(&mut b);
    b.offer(&"a".into());
    assert_eq!(
        b.take(),
        vec!["{\"type\":\"snapshot\",\"cursor\":2,\"view\":\"a\"}\n"]
    );
}

#[test]
fn the_policy_allows_max_restarts_in_its_window() {
    let mut p = RestartPolicy::new(2, Duration::from_secs(60));
    let t = Instant::now();
    assert!(p.allows(t));
    assert!(p.allows(t + Duration::from_secs(1)));
    assert!(!p.allows(t + Duration::from_secs(2)));
    let mut p = RestartPolicy::new(2, Duration::from_secs(60));
    assert!(p.allows(t));
    assert!(p.allows(t + Duration::from_secs(1)));
    assert!(
        p.allows(t + Duration::from_secs(61)),
        "the first crash has left the window"
    );
}

// --- the TypeScript side ---------------------------------------------------------

fn generated_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ink/src/generated")
}

fn export(dir: &Path) {
    let cfg = ts_rs::Config::new()
        .with_out_dir(dir)
        .with_large_int("number")
        .with_import_extension(Some("js"));
    ToHost::<()>::export_all(&cfg).unwrap();
    ToRenderer::<()>::export_all(&cfg).unwrap();
}

fn files(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "ts") {
            out.insert(
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            );
        }
    }
    out
}

/// The committed TypeScript in ink/src/generated is what these types export.
/// `TUI_KIT_TS_UPDATE=1` writes it.
#[test]
fn the_committed_typescript_matches_the_rust_types() {
    let fresh = std::env::temp_dir().join(format!("tui-kit-ipc-ts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fresh);
    export(&fresh);
    let want = files(&fresh);
    let _ = std::fs::remove_dir_all(&fresh);
    let dir = generated_dir();
    if std::env::var_os("TUI_KIT_TS_UPDATE").is_some() {
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, text) in &want {
            std::fs::write(dir.join(name), text).unwrap();
        }
    }
    assert_eq!(
        files(&dir),
        want,
        "ink/src/generated differs from the Rust types: run with TUI_KIT_TS_UPDATE=1"
    );
}
