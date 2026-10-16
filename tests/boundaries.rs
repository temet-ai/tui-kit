//! The boundaries docs/ARCHITECTURE.md describes, checked by reading the
//! sources and manifests. A failure names the file and line that crosses one.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every file under `dir` with one of `exts`, with its text.
fn sources(dir: &Path, exts: &[&str]) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                if !p.ends_with("node_modules") {
                    todo.push(p);
                }
            } else if p.extension().is_some_and(|x| exts.iter().any(|e| x == *e)) {
                out.push((p.clone(), std::fs::read_to_string(&p).unwrap_or_default()));
            }
        }
    }
    out
}

/// Lines of `files` (comments skipped) that contain any of `words`.
fn hits(files: &[(PathBuf, String)], words: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text) in files {
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            for w in words {
                if code.contains(w) {
                    found.push(format!("{}:{}: {w}", path.display(), n + 1));
                }
            }
        }
    }
    found
}

/// The design layer draws cells: it uses none of the image path's modules or crates.
#[test]
fn the_design_layer_uses_nothing_from_the_image_path() {
    let design = sources(&root().join("src/design"), &["rs"]);
    let words = [
        "crate::card",
        "crate::cache",
        "crate::widget",
        "crate::render",
        "crate::svg",
        "crate::theme",
        "crate::nav",
        "crate::console",
        "crate::tabs",
        "crate::statusbar",
        "crate::shader",
        "ratatui_image",
        "resvg",
        "usvg",
        "tiny_skia",
        "fontdb",
        "image::",
        "tui_shader",
    ];
    let found = hits(&design, &words);
    assert!(
        found.is_empty(),
        "src/design reaches into the image path:\n{}",
        found.join("\n")
    );
}

/// The image path does not depend on the design layer (lib.rs only re-exports it).
#[test]
fn the_image_path_uses_nothing_from_the_design_layer() {
    let image: Vec<_> = sources(&root().join("src"), &["rs"])
        .into_iter()
        .filter(|(p, _)| !p.starts_with(root().join("src/design")) && !p.ends_with("lib.rs"))
        .collect();
    let found = hits(&image, &["crate::design", "design::"]);
    assert!(
        found.is_empty(),
        "the image path reaches into src/design:\n{}",
        found.join("\n")
    );
}

/// tui-kit-ipc knows no renderer, no terminal library and no part of tui-kit.
#[test]
fn the_ipc_crate_depends_on_serialisation_only() {
    let manifest = std::fs::read_to_string(root().join("crates/tui-kit-ipc/Cargo.toml")).unwrap();
    let deps: Vec<&str> = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap_or("")
        .lines()
        .take_while(|l| !l.starts_with('['))
        .filter_map(|l| l.split('=').next().map(str::trim))
        .filter(|n| !n.is_empty() && !n.starts_with('#'))
        .collect();
    for d in &deps {
        assert!(
            ["serde", "serde_json", "ts-rs"].contains(d),
            "crates/tui-kit-ipc depends on {d}: it may use serde, serde_json and ts-rs only"
        );
    }
    let src = sources(&root().join("crates/tui-kit-ipc/src"), &["rs"]);
    let found = hits(&src, &["ratatui", "crossterm", "tui_kit::"]);
    assert!(
        found.is_empty(),
        "tui-kit-ipc names a terminal library or tui-kit:\n{}",
        found.join("\n")
    );
}

/// tui-kit-ink imports neither React nor Ink, so an app keeps its own single copy.
#[test]
fn the_ink_package_imports_neither_react_nor_ink() {
    let src = sources(&root().join("ink/src"), &["ts", "tsx"]);
    let found = hits(
        &src,
        &[
            "from \"react\"",
            "from \"ink\"",
            "from 'react'",
            "from 'ink'",
        ],
    );
    assert!(
        found.is_empty(),
        "ink/src imports React or Ink:\n{}",
        found.join("\n")
    );
    let package = std::fs::read_to_string(root().join("ink/package.json")).unwrap();
    let deps = package
        .split("\"dependencies\"")
        .nth(1)
        .unwrap_or("")
        .split('}')
        .next()
        .unwrap_or("");
    assert!(
        !deps.contains("\"react\"") && !deps.contains("\"ink\""),
        "ink/package.json lists React or Ink as a dependency"
    );
}
