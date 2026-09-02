# tui-kit

`tui-kit` is the reusable home of **TUI Design**, the normative, application-neutral
contract for Ratatui interfaces. Start with:

- [`docs/tui/STYLE_GUIDE.md`](docs/tui/STYLE_GUIDE.md) — normative TUI Design rules
- [`docs/tui/LAYOUT.md`](docs/tui/LAYOUT.md) — the one universal layout
- [`docs/tui/PANELS.md`](docs/tui/PANELS.md) — editable proposal worksheet for the next
  panel-internal geometry phase; not normative
- [`docs/tui/REFERENCES.md`](docs/tui/REFERENCES.md) — sources and implementation references

The Rust crate is an implementation library that is still being brought into conformance
with TUI Design. Existing widgets are **not automatically conformant merely because they
live in this repository**. Conformance is established against the normative documentation
and verified rendered behavior.

The current implementation began as an image-based, TEMET-styled card-navigation toolset
for [Ratatui](https://ratatui.rs) TUIs. Cards are authored as SVG, rasterized to PNG through
a vendored `resvg`/`tiny-skia` pipeline, and placed inline with
[`ratatui-image`](https://docs.rs/ratatui-image). This describes the current crate, not the
normative direction of TUI Design.

## Current image-primary implementation

The current visual look — TEMET's sharp-edged design system: violet-black surfaces, 1px hairline
borders (radius 0, no rounding/bevels), a UV-purple focus glow, corner reticle ticks, and an
on-border title tee — lives entirely in `src/svg.rs`, not in styled `Span`s. `render::rasterize`
turns that SVG into a
`DynamicImage`, and `widget::CardGrid` hands it to `ratatui-image`'s `StatefulImage` for
placement. On a terminal with Kitty, iTerm2, or Sixel graphics support you get the real
raster image; `ratatui-image` detects the terminal's capability at runtime and falls back
automatically to unicode half-blocks when none is available — no separate no-graphics code
path to maintain in this crate.

## Three reuse boundaries

1. **Theme is injected, never global.** `Theme` (`src/theme.rs`) is a plain value passed by
   reference through every call — `card_to_svg`, `CardGrid::new`. No statics, no
   thread-locals. Swap the whole look by constructing a different `Theme`.
2. **`Card` is a data descriptor, not a domain type.** `src/card.rs` knows nothing about
   what a "card" means to your app — build one from a menu entry, a session, a workspace,
   whatever you actually model.
3. **Navigation is data-in, action-out.** `nav::handle_key(&mut NavState, KeyEvent) ->
   Option<NavAction>` mutates state and returns what happened; it never calls back into
   application code and takes no closures. Your app matches on `NavAction::{Focus,
   Activate, Back}` and decides what to do.

## Adding as a git dependency

```toml
[dependencies]
tui-kit = { git = "https://github.com/<you>/tui-kit" }
```

Use the re-exported `tui_kit::ratatui` / `tui_kit::ratatui_image` (not your own separate
deps on those crates) so your `Buffer`/protocol types can never version-skew against this
crate's:

```rust
use tui_kit::{Card, CardGrid, ImageCache, NavState, Theme, handle_key};
use tui_kit::ratatui_image::picker::Picker;
```

## Portability

The rasterizer (`src/render.rs`) is fully vendored: `resvg`/`tiny-skia`/`fontdb`/`usvg` plus
a bundled Roboto Regular font (`assets/Roboto-Regular.ttf`, Apache-2.0) embedded via
`include_bytes!`. There is no dependency on grok-build or any other host project, and no
system-font requirement for ASCII card text — it renders identically on any machine with a
Rust toolchain. Non-ASCII text additionally consults system fonts as glyph fallback only.

## Try it

```sh
cargo run --example gallery              # interactive: arrows/hjkl to move, q/Esc to quit
cargo run --example gallery -- --print   # headless: writes gallery_focused.png / gallery_unfocused.png
```
