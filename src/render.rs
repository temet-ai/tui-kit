//! SVG -> PNG -> [`image::DynamicImage`] rasterization via `resvg`/`usvg`/
//! `tiny-skia`.
//!
//! Adapted from `xai-grok-mermaid`'s `raster.rs` and `lib.rs`
//! (grok-build, Apache-2.0: <https://github.com>, internal path
//! `crates/codegen/xai-grok-mermaid`), self-contained here with no
//! dependency on grok-build itself. Bundled font: Roboto Regular
//! (Apache-2.0, `assets/Roboto-LICENSE.txt`).
//!
//! Configured for untrusted input, same as the source: a bundled-first font
//! database (system fonts are consulted only as glyph fallback for non-ASCII
//! text), no external image resolvers (no `file://`/`http://`/local-path
//! reads), and hard output-size caps.

use std::sync::Arc;
use std::sync::OnceLock;

// usvg/tiny_skia come from `resvg`'s re-exports so their versions match
// `resvg::render` exactly, mirroring how grok-build avoids a separate `usvg`
// pin going out of sync with the one `resvg` links against.
use resvg::tiny_skia;
use resvg::usvg;

/// Bundled Roboto Regular (Apache-2.0); system fonts are consulted only as
/// glyph fallback for characters it lacks.
const BUNDLED_FONT: &[u8] = include_bytes!("../assets/Roboto-Regular.ttf");

/// Hard ceiling on output area, applied regardless of requested size, to
/// bound memory over untrusted/huge SVGs (32 MP ~= a 5657x5657 image).
pub const MAX_OUTPUT_MEGAPIXELS: f32 = 32.0;

/// Hard ceiling on either output axis, so an extreme-aspect SVG can't pin
/// one dimension to a huge value even when the area cap leaves headroom.
const MAX_OUTPUT_DIMENSION: u32 = 16_384;

/// Errors from [`rasterize`].
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// The SVG could not be parsed, had zero size, or failed to encode.
    #[error("rasterize failed: {0}")]
    Rasterize(String),
}

struct FontSet {
    db: Arc<fontdb::Database>,
    family: String,
    bundled_id: fontdb::ID,
}

fn build_font_set(with_system_fonts: bool) -> FontSet {
    let mut db = fontdb::Database::new();
    db.load_font_data(BUNDLED_FONT.to_vec());
    let (bundled_id, family) = db
        .faces()
        .next()
        .map(|face| {
            (
                face.id,
                face.families
                    .first()
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| "sans-serif".to_string()),
            )
        })
        .expect("the bundled font must parse to at least one face");
    if with_system_fonts {
        db.load_system_fonts();
    }
    // Card SVGs use generic font-family lists (e.g. "sans-serif",
    // "monospace"). None of those generic names are loaded, so resolution
    // would otherwise fall to fontdb's unloaded default (Arial/Times/...),
    // dropping every glyph. Point all generics at the bundled face so any
    // list resolves to Roboto.
    db.set_serif_family(&family);
    db.set_sans_serif_family(&family);
    db.set_monospace_family(&family);
    db.set_cursive_family(&family);
    db.set_fantasy_family(&family);
    FontSet {
        db: Arc::new(db),
        family,
        bundled_id,
    }
}

/// Parse and index the bundled font once; the `Arc` is shared into every
/// `Options` per render with an O(1) refcount bump (no per-render clone).
fn bundled_font() -> &'static FontSet {
    static FONT: OnceLock<FontSet> = OnceLock::new();
    FONT.get_or_init(|| build_font_set(false))
}

fn font_with_system_fallback() -> &'static FontSet {
    static FONT: OnceLock<FontSet> = OnceLock::new();
    FONT.get_or_init(|| build_font_set(true))
}

fn font_set_for(svg: &str) -> &'static FontSet {
    if svg.is_ascii() {
        bundled_font()
    } else {
        font_with_system_fallback()
    }
}

fn pinned_resolver(bundled_id: fontdb::ID) -> usvg::FontResolver<'static> {
    usvg::FontResolver {
        select_font: Box::new(move |_font, _db| Some(bundled_id)),
        select_fallback: usvg::FontResolver::default_fallback_selector(),
    }
}

/// Rasterize `svg` to a [`image::DynamicImage`], scaled so its width matches
/// `target_width_px` (height follows the SVG's intrinsic aspect ratio).
/// `bg` fills the canvas opaquely (`[r, g, b, a]`); `None` leaves it
/// transparent.
///
/// # Errors
///
/// Returns [`RenderError::Rasterize`] if the SVG cannot be parsed, has zero
/// size, or cannot be decoded back into an image after encoding.
pub fn rasterize(
    svg: &str,
    target_width_px: u32,
    bg: Option<[u8; 4]>,
) -> Result<image::DynamicImage, RenderError> {
    let font = font_set_for(svg);

    let mut opt = usvg::Options {
        fontdb: Arc::clone(&font.db),
        font_family: font.family.clone(),
        font_resolver: pinned_resolver(font.bundled_id),
        ..Default::default()
    };
    // SECURITY: usvg's default string resolver reads image hrefs off disk
    // (`std::fs::read`). Replace it with a no-op so a crafted SVG can never
    // read local files or reach the network. In-memory data-URLs stay
    // supported.
    opt.image_href_resolver.resolve_string = Box::new(|_href, _opt| None);

    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| RenderError::Rasterize(e.to_string()))?;

    let size = tree.size();
    let (base_w, base_h) = (size.width(), size.height());
    if base_w <= 0.0 || base_h <= 0.0 {
        return Err(RenderError::Rasterize("svg has zero size".to_string()));
    }

    let mut scale = if target_width_px > 0 {
        target_width_px as f32 / base_w
    } else {
        1.0
    };
    if !scale.is_finite() || scale <= 0.0 {
        scale = 1.0;
    }
    let max_area = MAX_OUTPUT_MEGAPIXELS * 1_000_000.0;
    let area = (base_w * scale) * (base_h * scale);
    if area > max_area {
        scale *= (max_area / area).sqrt();
    }
    scale = scale.max(f32::MIN_POSITIVE);

    let (width_px, height_px) = clamp_dimensions(base_w * scale, base_h * scale);

    let mut pixmap = tiny_skia::Pixmap::new(width_px, height_px).ok_or_else(|| {
        RenderError::Rasterize(format!("invalid pixmap size {width_px}x{height_px}"))
    })?;
    if let Some([r, g, b, a]) = bg {
        pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, a));
    }

    // Scale each axis to fill the chosen pixmap exactly. After the integer
    // clamps the two axis scales can differ by up to ~1/base_dim, a
    // sub-pixel aspect skew for typical sizes; an exact fill (no transparent
    // margins) is preferred over perfect aspect preservation.
    let transform =
        tiny_skia::Transform::from_scale(width_px as f32 / base_w, height_px as f32 / base_h);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let png = pixmap
        .encode_png()
        .map_err(|e| RenderError::Rasterize(e.to_string()))?;
    let img = image::load_from_memory(&png).map_err(|e| RenderError::Rasterize(e.to_string()))?;
    Ok(img)
}

/// Convert floored float dimensions into the final integer pixmap size,
/// enforcing the hard caps.
///
/// The float area is capped in `rasterize` before this runs, but flooring a
/// sub-1px axis up to 1 (via `.max(1)`) can inflate the integer product past
/// the cap for an extreme-aspect SVG. So after floor+`max(1)` this
/// additionally (a) caps each axis at [`MAX_OUTPUT_DIMENSION`] and (b)
/// shrinks the larger axis until `width * height <= MAX_OUTPUT_MEGAPIXELS`.
fn clamp_dimensions(width_f: f32, height_f: f32) -> (u32, u32) {
    let mut width_px = (width_f.floor() as u32).clamp(1, MAX_OUTPUT_DIMENSION);
    let mut height_px = (height_f.floor() as u32).clamp(1, MAX_OUTPUT_DIMENSION);

    let max_area = (MAX_OUTPUT_MEGAPIXELS * 1_000_000.0) as u64;
    if width_px as u64 * height_px as u64 > max_area {
        if width_px >= height_px {
            width_px = ((max_area / height_px as u64) as u32).max(1);
        } else {
            height_px = ((max_area / width_px as u64) as u32).max(1);
        }
    }
    (width_px, height_px)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG_100X50: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50"><rect x="0" y="0" width="10" height="10" fill="#0000ff"/></svg>"##;

    #[test]
    fn rasterize_scales_to_target_width() {
        let img = rasterize(SVG_100X50, 200, None).expect("rasterize");
        assert_eq!(img.width(), 200);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn zero_size_svg_is_error() {
        let zero = r##"<svg xmlns="http://www.w3.org/2000/svg" width="0" height="0" viewBox="0 0 0 0"></svg>"##;
        assert!(matches!(
            rasterize(zero, 200, None),
            Err(RenderError::Rasterize(_))
        ));
    }

    #[test]
    fn invalid_svg_is_error() {
        assert!(matches!(
            rasterize("not svg", 200, None),
            Err(RenderError::Rasterize(_))
        ));
    }

    #[test]
    fn megapixel_cap_bounds_huge_request() {
        let img = rasterize(SVG_100X50, 200_000, None).expect("rasterize");
        let area = img.width() as u64 * img.height() as u64;
        assert!(area <= (MAX_OUTPUT_MEGAPIXELS as u64) * 1_000_000);
    }

    #[test]
    fn opaque_background_fills_canvas() {
        let img = rasterize(SVG_100X50, 100, Some([255, 0, 0, 255]))
            .expect("rasterize")
            .to_rgba8();
        let px = img.get_pixel(95, 45);
        assert_eq!(px.0, [255, 0, 0, 255]);
    }
}
