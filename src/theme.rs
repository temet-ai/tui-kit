//! The temet design-system palette, spacing metrics, and glyphs. Every color
//! literal in this crate lives here (or in `svg.rs`, which reads only from a
//! `Theme`) so a downstream consumer can swap the whole look by passing a
//! different `Theme` value — no globals, no statics, no thread-locals.

use ratatui::style::Color;

/// The card toolset's color palette: temet's design-system tokens. Passed
/// by reference through every render call; never stored globally so
/// multiple themes can coexist (e.g. a settings preview) in the same
/// process. Sharp corners and 1px hairlines only — no rounding, no
/// bevels/chamfers — so the palette never needs a "radius" token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Page background between/behind panels (violet-black, never pure
    /// black).
    pub app: Color,
    /// Card/panel surface fill.
    pub panel: Color,
    /// A lighter surface tone for raised elements.
    pub raised: Color,
    /// Surface tone for a selected (not necessarily focused) element.
    pub selected: Color,
    /// Primary body text.
    pub text: Color,
    /// Secondary text (unfocused title tee).
    pub muted: Color,
    /// Faintest text (footer tee, dimmed hints).
    pub faint: Color,
    /// Idle 1px border.
    pub border: Color,
    /// Primary accent (UV): focused border, glow, title tee, bullets.
    pub accent: Color,
    /// Secondary accent (rust), for emphasis distinct from the primary UV.
    pub accent_2: Color,
    /// Status color: ok/acid.
    pub ok: Color,
    /// Status color: info/ice.
    pub info: Color,
    /// Status color: error/red.
    pub error: Color,
}

impl Theme {
    /// temet: violet-black surfaces, UV-purple accent, sharp 1px borders.
    /// Never teal, never amber. This is the toolset's default theme.
    pub fn temet() -> Self {
        Self {
            app: Color::Rgb(0x08, 0x06, 0x0f),
            panel: Color::Rgb(0x0c, 0x0a, 0x16),
            raised: Color::Rgb(0x13, 0x0e, 0x20),
            selected: Color::Rgb(0x1a, 0x13, 0x2c),
            text: Color::Rgb(0xdc, 0xd6, 0xec),
            muted: Color::Rgb(0xa7, 0x9d, 0xc4),
            faint: Color::Rgb(0x86, 0x7a, 0xa8),
            border: Color::Rgb(0x4a, 0x40, 0x70),
            accent: Color::Rgb(0x9d, 0x5c, 0xff),
            accent_2: Color::Rgb(0xff, 0x5c, 0x2e),
            ok: Color::Rgb(0xcf, 0xf5, 0x4d),
            info: Color::Rgb(0x4d, 0x9f, 0xff),
            error: Color::Rgb(0xff, 0x2d, 0x55),
        }
    }

    /// This theme's page background as opaque `[r, g, b, a]` bytes, for
    /// `render::rasterize`'s canvas fill and `widget.rs`'s rasterize-failure
    /// fallback tile. A non-`Rgb` `Color` (named/indexed) falls back to
    /// temet's own background rather than propagating an unrepresentable
    /// color.
    pub fn bg_rgba(&self) -> [u8; 4] {
        match self.app {
            Color::Rgb(r, g, b) => [r, g, b, 0xff],
            _ => [0x08, 0x06, 0x0f, 0xff],
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::temet()
    }
}

/// Spacing/sizing constants for the card SVG layout, in SVG user units
/// (`card_to_svg` treats 1 unit ~= 1px at the default viewBox scale).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// Inner padding between the card border and its content.
    pub pad: f32,
    /// Height of the on-border title tee's background cutout.
    pub title_tee_height: f32,
    /// Border stroke width — always 1px in the temet system.
    pub border_width: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            pad: 20.0,
            title_tee_height: 16.0,
            border_width: 1.0,
        }
    }
}

/// Small glyph set used in card bullets/actions so callers never hardcode
/// Unicode literals for these marks. Other status glyphs in the temet
/// system (`▣ ⊠ ▲ ◇ ◈ ▾`) are available for callers to compose their own
/// status rows; this set covers only the two this crate's card layout uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphSet {
    /// Bullet marker prefix.
    pub bullet: char,
    /// Action-row arrow marker.
    pub arrow: char,
}

impl Default for GlyphSet {
    fn default() -> Self {
        Self {
            bullet: '\u{25c6}', // ◆
            arrow: '\u{25b8}',  // ▸
        }
    }
}
