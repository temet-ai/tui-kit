# web-gallery

The `tui-kit` temet card gallery, running in a browser tab. Same cards, same
SVG-to-raster pipeline, same `CardGrid` widget as the native `gallery` example —
only the backend differs: [ratzilla]'s `DomBackend` paints the ratatui `Buffer`
as DOM spans instead of writing escape sequences to a terminal.

Cards arrive in that `Buffer` as **half-blocks**: one upper-half-block glyph per
cell, foreground the top pixel, background the bottom. That is the only
`ratatui-image` protocol that survives the trip — Kitty, iTerm2 and Sixel all
need a terminal emulator to interpret their escape bytes, and there isn't one.

## Building

This is a standalone crate, not a member of the repo-root workspace: its
`Cargo.toml` opens with an empty `[workspace]` table so the root `cargo` never
pulls it into the native build graph. Build it from this directory.

```sh
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown
```

To actually serve it you need [trunk], which reads the `data-trunk` link tag in
`index.html`:

```sh
trunk serve
```

Arrow keys or `hjkl` move the focus ring between cards; `Home`/`End` jump to the
ends.

## Two things the wasm target forces

**No crossterm.** The dependency on `tui-kit` sets `default-features = false`,
which drops its `input` feature. What remains is the portable core —
theme, svg, card, render, cache, widget — so `tui_kit::nav::handle_key` (which
takes a crossterm `KeyEvent`) is unavailable and `src/main.rs` maps ratzilla's
own key events onto `NavState` directly.

**A patched `rustix`.** `ratatui-image` declares `rustix` 0.38 for every
`cfg(not(windows))` target, and rustix 0.38 has no wasm32-unknown-unknown
backend — its `errno` dependency stops the build with a `compile_error!` before
any code here is reached. `vendor/rustix-wasm-shim` stands in for the four
tty-only APIs `ratatui-image` actually names, wired up by a `[patch.crates-io]`
entry scoped to this workspace alone; the native build keeps the real rustix.
The shim's `src/lib.rs` documents what would retire it: an upstream
`ratatui-image` change narrowing that dependency to
`cfg(all(not(windows), not(target_arch = "wasm32")))`.

[ratzilla]: https://github.com/ratatui/ratzilla
[trunk]: https://trunkrs.dev
