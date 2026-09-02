//! `tui-kit`: an image-based, temet-styled card-navigation toolset for ratatui TUIs.
//!
//! Cards are authored as SVG, rasterized to PNG via a vendored `resvg`/
//! `tiny-skia` pipeline, and placed inline through the terminal graphics
//! protocol (Kitty/iTerm2/Sixel, with an automatic half-block fallback) using
//! `ratatui-image` — never box-drawing cells.
//!
//! Alongside that image path, [`design`] is the text-mode **TUI Design**
//! implementation layer: SilkCircuit Neon colors through a capability tier,
//! the one universal layout, bordered panels, a navigation-only help bar, and
//! full-row selection — box-drawing cells only, and no dependency on any of
//! the rasterizer crates the image path uses.
//!
//! Re-exported at the crate root so downstream consumers build against this
//! crate's own pinned `ratatui`/`ratatui-image`/`opaline` versions rather than
//! a separately-resolved (and potentially skewed) direct dependency.
pub use opaline;
pub use ratatui;
pub use ratatui_image;

pub mod cache;
pub mod card;
#[cfg(feature = "input")]
pub mod console;
pub mod design;
// `nav` itself stays unconditional: `NavState`/`NavAction` carry no crossterm
// dependency and `widget::CardGrid` (portable core) uses `NavState` as its
// `StatefulWidget::State`. Only `handle_key` (which takes a crossterm
// `KeyEvent`) is feature-gated inside the module.
pub mod nav;
pub mod render;
#[cfg(feature = "shader")]
pub mod shader;
pub mod statusbar;
pub mod svg;
pub mod tabs;
pub mod theme;
pub mod widget;

pub use cache::ImageCache;
pub use card::{Card, CardState};
#[cfg(feature = "input")]
pub use console::{handle_console_key, ConsoleAction, ConsoleShell, ConsoleState};
pub use design::{DesignTheme, Slot, Tier};
#[cfg(feature = "input")]
pub use nav::handle_key;
pub use nav::{NavAction, NavState};
pub use render::{rasterize, RenderError};
pub use statusbar::StatusBar;
pub use svg::card_to_svg;
pub use tabs::Tabs;
pub use theme::{GlyphSet, Metrics, Theme};
pub use widget::CardGrid;
