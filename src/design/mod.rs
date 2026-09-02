//! **TUI Design** — the text-mode implementation layer.
//!
//! An implementation of the normative rules in `docs/tui/STYLE_GUIDE.md`
//! (`TUI-*`) and `docs/tui/LAYOUT.md`: SilkCircuit Neon colors resolved
//! through a capability tier, the one universal layout, bordered panels titled
//! by the item they show, a navigation-only help bar, and full-row selection.
//!
//! It is a separate module tree from this crate's image-card path on purpose.
//! Nothing under `design::` touches `ratatui-image`, `resvg`, `usvg`,
//! `tiny-skia`, `fontdb`, `image`, or `tui-shader`: it renders box-drawing
//! cells and `Style`s, and its only new dependency is
//! [`opaline`](https://crates.io/crates/opaline), a token theme engine that
//! pulls `ratatui-core`, `serde`, `toml` and `thiserror` and nothing else.
//!
//! # Putting a screen together
//!
//! ```
//! use tui_kit::design::layout::{frame, nav_width_for, FrameSpec, AppBar};
//! use tui_kit::design::help_bar::{HelpBar, KeyHint};
//! use tui_kit::design::panel::Panel;
//! use tui_kit::design::select;
//! use tui_kit::design::theme::{DesignTheme, Tier};
//! use tui_kit::ratatui::layout::Rect;
//! use tui_kit::ratatui::buffer::Buffer;
//! use tui_kit::ratatui::widgets::Widget;
//!
//! // One theme, built at startup and passed by reference.
//! let theme = DesignTheme::with_tier(Tier::TrueColor);
//!
//! let items = ["services", "config", "setup", "sessions"];
//! let spec = FrameSpec::new(nav_width_for(items));
//! let area = Rect::new(0, 0, 80, 24);
//! let f = frame(area, spec);
//!
//! let mut buf = Buffer::empty(area);
//! AppBar::new(&["mike", "sessions"], &theme).render(f.app_bar, &mut buf);
//! Panel::new("mike", &theme).shortcut('m').render(f.nav, &mut buf);
//! Panel::new("sessions", &theme).focused(true).render(f.middle, &mut buf);
//! if !f.detail_is_modal() {
//!     Panel::new("a1b2c3d4", &theme).render(f.detail, &mut buf);
//! }
//! HelpBar::new(
//!     &[KeyHint::new("j/k", "move"), KeyHint::new("?", "help")],
//!     &theme,
//! )
//! .render(f.help_bar, &mut buf);
//!
//! // Rows highlight across their full width, with no marker column.
//! let _selected = select::selected_row(&theme, true);
//! ```

pub mod help_bar;
pub mod layout;
pub mod panel;
pub mod select;
pub mod theme;

pub use help_bar::{HelpBar, KeyHint};
pub use layout::{frame, fits, AppBar, Frame, FrameMode, FrameSpec};
pub use panel::Panel;
pub use theme::{DesignTheme, Slot, SlotValue, TerminalEnv, Tier};
