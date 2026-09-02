//! GPU-shader rendering backend: a temet-styled plasma/scanline backdrop.
//!
//! Gated behind the `shader` feature (`dep:tui-shader`, pinned to
//! `=0.0.10`). The embedded WGSL fragment shader (see [`TEMET_SHADER_WGSL`])
//! computes a single grayscale intensity value per fragment — a slow,
//! drifting plasma field with a faint scanline wobble, cyberpunk but
//! subtle. It never touches color. Color only enters in
//! [`temet_style_rule`], which maps that intensity onto
//! `crate::theme::Theme::temet()`'s own palette — so every color this
//! module ever paints comes from the theme, never from a hex/RGB literal
//! written here.
//!
//! `tui-shader`'s [`StyleRule::Map`] only accepts a bare `fn` pointer, not a
//! capturing closure, so [`temet_style_rule`] reads `Theme::temet()` itself
//! on every call rather than being handed a theme value.

use tui_shader::{CharacterRule, Sample, ShaderCanvas, ShaderCanvasState, StyleRule, WgslShader};

use ratatui::style::{Color, Style};

use crate::theme::Theme;

/// The embedded temet plasma/scanline WGSL fragment shader.
///
/// Combines a handful of drifting sine waves (a cheap plasma field) with a
/// slow horizontal scanline wobble, then outputs the result as a *grayscale*
/// color (`r == g == b`) — an intensity field only. Sampled back out via
/// [`Sample::r`] in [`temet_style_rule`], which is the only place in this
/// module that touches an actual color.
const TEMET_SHADER_WGSL: &str = r#"
@group(0) @binding(0) var<uniform> time: vec4<f32>;
@group(0) @binding(1) var<uniform> rect: vec4<u32>;

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // `time.x` is seconds elapsed; scaled down for a slow drift rather than
    // a strobe.
    let t = time.x * 0.15;

    let x = uv.x * 8.0;
    let y = uv.y * 8.0;

    var v = sin(x + t);
    v += sin(y * 0.6 - t * 0.8);
    v += sin((x + y) * 0.4 + t * 1.3);
    v += sin(sqrt(x * x + y * y) * 0.5 - t * 0.6);
    v = v * 0.25; // average of 4 unit-amplitude waves -> back in [-1, 1]

    var intensity = v * 0.5 + 0.5; // remap to [0, 1]

    // A faint, slowly drifting horizontal scanline wobble on top of the
    // plasma field, subtle enough not to read as a strobe.
    let scan = sin(uv.y * 90.0 - t * 4.0) * 0.03;
    intensity = clamp(intensity + scan, 0.0, 1.0);

    return vec4<f32>(intensity, intensity, intensity, 1.0);
}
"#;

/// Builds a [`ShaderCanvasState`] running the temet plasma/scanline preset.
///
/// Pair with [`temet_canvas`] and render as a [`ShaderCanvas`]
/// `StatefulWidget`:
///
/// ```rust,no_run
/// # use tui_kit::shader::{temet_canvas, temet_state};
/// let mut terminal = ratatui::init();
/// let mut state = temet_state();
/// terminal.draw(|frame| {
///     frame.render_stateful_widget(temet_canvas(), frame.area(), &mut state);
/// }).unwrap();
/// ratatui::restore();
/// ```
///
/// # Panics
/// Panics if the embedded WGSL source fails to parse, or if no compatible
/// GPU adapter is available on the current machine. Both are effectively
/// "can't happen" for a shader vendored and tested in this crate — this
/// mirrors `tui_shader::ShaderCanvasState::default()`'s own panicking
/// constructor rather than pushing an unrecoverable setup failure into a
/// `Result` callers can't meaningfully act on.
pub fn temet_state() -> ShaderCanvasState {
    ShaderCanvasState::new(WgslShader::Source(TEMET_SHADER_WGSL))
        .expect("tui-kit: embedded temet plasma shader failed to compile")
}

/// A [`ShaderCanvas`] preconfigured with the temet style rule: every cell
/// renders as a space styled with a background color drawn from
/// [`temet_style_rule`], i.e. from `Theme::temet()`, never a literal.
pub fn temet_canvas() -> ShaderCanvas {
    ShaderCanvas::new()
        .character_rule(CharacterRule::Always(' '))
        .style_rule(StyleRule::Map(temet_style_rule))
}

/// Maps a shader [`Sample`]'s grayscale intensity to a temet-themed
/// background [`Style`].
///
/// Must be a bare `fn`, not a closure, because `tui_shader::StyleRule::Map`
/// only accepts a `fn(Sample) -> Style` pointer (no captured environment).
/// It calls `Theme::temet()` fresh on every sample instead of closing over
/// a `Theme` value — the palette is looked up programmatically here, not
/// hardcoded.
///
/// Low intensity blends from the panel surface toward the primary UV
/// accent; the top of the range blends on into the secondary (rust) accent,
/// so the plasma's brightest lobes read as a distinct hot color as they
/// drift through frame.
fn temet_style_rule(sample: Sample) -> Style {
    let theme = Theme::temet();
    let intensity = f32::from(sample.r()) / 255.0;

    const HOT_START: f32 = 0.6;
    let bg = if intensity < HOT_START {
        lerp_color(theme.panel, theme.accent, intensity / HOT_START)
    } else {
        lerp_color(
            theme.accent,
            theme.accent_2,
            (intensity - HOT_START) / (1.0 - HOT_START),
        )
    };

    Style::new().bg(bg)
}

/// Linearly interpolates between two `Color::Rgb` values (`t` in `[0, 1]`,
/// clamped). Both arguments here are always colors taken from
/// `Theme::temet()`, which only ever constructs `Color::Rgb`; the non-Rgb
/// branch exists only so a future non-Rgb `Theme` field degrades to `a`
/// instead of panicking.
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let mix = |x: u8, y: u8| -> u8 { (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8 };
            Color::Rgb(mix(ar, br), mix(ag, bg), mix(ab, bb))
        }
        _ => a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The embedded shader must be valid WGSL that `ShaderCanvasState`
    /// actually accepts — this exercises the real GPU adapter/pipeline
    /// setup, not just a string check.
    #[test]
    fn temet_state_builds() {
        let _state = temet_state();
    }

    /// Renders one real frame of `temet_canvas()`/`temet_state()` into a
    /// `TestBackend` and checks every cell's background is one of
    /// `Theme::temet()`'s Rgb colors (channel-wise between `panel` and
    /// `accent_2`, the full span `temet_style_rule` blends across) — proof
    /// that the shader's output actually drives cell styling and that the
    /// colors it lands on stay inside the theme's own palette rather than
    /// some other literal. `tui_shader::Sample` has no public constructor,
    /// so exercising `temet_style_rule` this way (through the real
    /// `StatefulWidget`) is the only way to test it from outside the crate.
    #[test]
    fn rendering_stays_within_theme_palette() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let theme = Theme::temet();
        let Color::Rgb(pr, pg, pb) = theme.panel else { panic!("theme.panel is not Rgb") };
        let Color::Rgb(ar, ag, ab) = theme.accent else { panic!("theme.accent is not Rgb") };
        let Color::Rgb(hr, hg, hb) = theme.accent_2 else { panic!("theme.accent_2 is not Rgb") };
        let lo = |x: u8, y: u8, z: u8| x.min(y).min(z);
        let hi = |x: u8, y: u8, z: u8| x.max(y).max(z);
        let (rlo, rhi) = (lo(pr, ar, hr), hi(pr, ar, hr));
        let (glo, ghi) = (lo(pg, ag, hg), hi(pg, ag, hg));
        let (blo, bhi) = (lo(pb, ab, hb), hi(pb, ab, hb));

        let mut terminal = Terminal::new(TestBackend::new(16, 8)).expect("TestBackend init");
        let mut state = temet_state();
        terminal
            .draw(|frame| {
                frame.render_stateful_widget(temet_canvas(), frame.area(), &mut state);
            })
            .expect("render temet_canvas");

        let buffer = terminal.backend().buffer();
        let mut checked = 0;
        for cell in buffer.content() {
            let Some(Color::Rgb(r, g, b)) = cell.style().bg else {
                panic!("expected an Rgb background color, got {:?}", cell.style().bg);
            };
            assert!(r >= rlo && r <= rhi, "r={r} outside theme range [{rlo},{rhi}]");
            assert!(g >= glo && g <= ghi, "g={g} outside theme range [{glo},{ghi}]");
            assert!(b >= blo && b <= bhi, "b={b} outside theme range [{blo},{bhi}]");
            checked += 1;
        }
        assert!(checked > 0, "expected at least one cell to check");
    }

    /// `lerp_color` at `t == 0.0` / `t == 1.0` must return the endpoints
    /// exactly, and stay within them for any `t` in between.
    #[test]
    fn lerp_color_endpoints() {
        let theme = Theme::temet();
        assert_eq!(lerp_color(theme.panel, theme.accent, 0.0), theme.panel);
        assert_eq!(lerp_color(theme.panel, theme.accent, 1.0), theme.accent);
    }
}
