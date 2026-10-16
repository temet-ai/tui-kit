# Architecture

tui-kit is a set of independent parts for Rust terminal apps. An app takes one part or
several; no part needs another to work. This document says what each part owns, where the
lines between them run, and how those lines are kept. `tests/boundaries.rs` checks the
lines; a change that crosses one fails there.

## The parts

```
                         your app (Rust)
          ┌──────────────────┼────────────────────────┐
          │                  │                        │
   tui_kit::design      tui_kit (image path)     tui-kit-ipc ──── socket ──── tui-kit-ink
   theme, layout,       Card, CardGrid,          host side:                   renderer side,
   panels, help bar     ConsoleShell, nav,       start, send views,           inside your
          │             SVG → image              restart, fall back           Ink app
          │                  │                        │                           │
       opaline          resvg, ratatui-image     serde, ts-rs                 string-width
          └────── ratatui ───┘                   (no terminal library)        (no React, no Ink)
                     ▲
          docs/tui/: the TUI Design rules every drawn part follows
```

| Part | Where | Owns | Depends on |
|---|---|---|---|
| TUI Design | `docs/tui/` | The rules: one layout, colour slots, glyphs, keys and help, verification. Numbered (`TUI-L3`) so code and reviews cite them | nothing |
| Design layer | `src/design/` | The rules in code: `DesignTheme` (slots over an Opaline palette, resolved per colour tier), the frame and app bar, `Panel`, `HelpBar`, row selection | ratatui, opaline |
| Image path | `src/` outside `design/` | Cards drawn as SVG, rasterised, shown through the terminal's graphics protocol; the console shell, tabs and status bar around them; card navigation | ratatui, ratatui-image, resvg and its crates |
| Renderer link, host side | `crates/tui-kit-ipc` | Running a renderer as a child process: the protocol, one view in flight, restarts, fallback, terminal restore | serde, serde_json, ts-rs |
| Renderer link, Ink side | `ink/` (`tui-kit-ink`) | The same link from inside an Ink app: connection, session, one exit path, cell widths | string-width |

## The lines between them

1. **The design layer and the image path do not use each other.** `src/design/` draws
   box-drawing cells and styles and names no card, cache, rasteriser or image crate; the
   image path names nothing in `design`. `src/lib.rs` re-exports both, and that is the only
   place they meet. An app that wants only the design layer still compiles the image
   crates today (one crate); splitting them into two crates is the next step if that cost
   matters, and these lines are what make the split mechanical.
2. **The renderer link knows no renderer and no screen.** `tui-kit-ipc` is generic over the
   app's view and input types (`App::View`, `App::Intent`) and names no terminal library
   and nothing else in tui-kit. It moves views and inputs and supervises a process; what a
   view contains is the app's business.
3. **The Ink side imports neither React nor Ink.** `tui-kit-ink` hands the app plain values
   and callbacks (`Session.state`, `subscribe`); the app's components use them. An app
   therefore has exactly one React and one Ink, its own.
4. **Rust is the source of the protocol.** The message types are written once in Rust and
   generated into TypeScript (`ink/src/generated/`) by a test that fails when the copy is
   stale. Nobody edits the generated files.
5. **The rules are above the code.** Where a widget in `src/design/` and `docs/tui/`
   disagree, the document wins and the widget is fixed. A widget is not correct because it
   is in this repository.
6. **Values are passed, not shared.** Themes are values passed by reference (`Theme`,
   `DesignTheme`), never globals; cards are data descriptors with no domain meaning;
   navigation takes state and an input and returns what happened, with no callbacks into
   the app. A part never reaches into the app.
7. **Re-exports pin shared crates.** `tui_kit::ratatui`, `tui_kit::ratatui_image` and
   `tui_kit::opaline` are re-exported so an app draws into the same `Buffer` types this crate
   was built against. Apps take Ratatui through them rather than their own dependency.
8. **Optional by feature.** Terminal input (crossterm) is behind the default `input`
   feature, so the portable core builds without it (the browser gallery in `web-gallery/`
   does); the shader backend is behind `shader`.

## How the renderer link runs

```
your app (Rust)                              Ink app (node, child process)
───────────────                              ─────────────────────────────
tui_kit_ipc::run(&mut app, &options)
  bind socket, start child  ───────────────▶ connect($SOCKET_VAR)
                            ◀─── hello {protocol, view version, size}
  versions match? welcome / refuse ────────▶ (refused: exit 3, not restarted)
  loop:
    app.view(size) ── snapshot {cursor, view} ─▶ draw it
                            ◀─────────── ack {cursor}
                            ◀── intent {cursor, input}   (the user pressed something)
    app.intent(input) → Flow
  quit:  shutdown ─────────────────────────▶ exit 0
  crash: restart (3 a minute), then Ended::FallBack → app draws with Ratatui
  every exit: terminal modes off, alternate screen left, line settings restored
```

The renderer owns the terminal (its stdin and stdout) while it runs; the Rust process
writes nothing to the terminal in that time. The app keeps every piece of state, so a
restarted renderer, or the Ratatui fallback, shows the same screen.

## Adding something

- **A rule:** in `docs/tui/`, with the next number in its series; the HTML renditions are
  regenerated by the `build_*.py` scripts beside them.
- **A widget that follows the rules:** in `src/design/`, using `DesignTheme` slots, never a
  colour literal, and widths in display cells. Name the rule it implements in its doc.
- **Something for the image path:** in `src/`, drawn through `svg.rs` and `render.rs`.
- **A reusable piece of the Rust ↔ Ink link:** Rust in `crates/tui-kit-ipc` (keep it free of
  terminal libraries), TypeScript in `ink/src` (keep it free of React and Ink). Change the
  message types in Rust and regenerate (`TUI_KIT_TS_UPDATE=1 cargo test -p tui-kit-ipc`).
- **A new kind of part:** its own folder under `crates/` (or beside `ink/` for TypeScript),
  depending on as little as it can, and a line in `tests/boundaries.rs` that keeps it so.

If the change needs to cross one of the lines above, change this document in the same
commit and say why.

## What is not here

The pattern for a whole app built this way (a host crate owning state, one crate per
feature, renderer-neutral views, a Ratatui and an Ink renderer drawing the same frame) is
written up with a starter project outside this repository. tui-kit holds only the parts
such an app shares.
