//! `tui-kit`: an image-based, temet-styled card-navigation toolset for ratatui TUIs.
//!
//! Cards are authored as SVG, rasterized to PNG via a vendored `resvg`/
//! `tiny-skia` pipeline, and placed inline through the terminal graphics
//! protocol (Kitty/iTerm2/Sixel, with an automatic half-block fallback) using
//! `ratatui-image` — never box-drawing cells.
//!
//! Re-exported at the crate root so downstream consumers build against this
//! crate's own pinned `ratatui`/`ratatui-image` versions rather than a
//! separately-resolved (and potentially skewed) direct dependency.
pub use ratatui;
pub use ratatui_image;

pub mod cache;
pub mod card;
pub mod nav;
pub mod render;
pub mod svg;
pub mod theme;
pub mod widget;

pub use cache::ImageCache;
pub use card::{Card, CardState};
pub use nav::{NavAction, NavState, handle_key};
pub use render::{RenderError, rasterize};
pub use svg::card_to_svg;
pub use theme::{GlyphSet, Metrics, Theme};
pub use widget::CardGrid;
