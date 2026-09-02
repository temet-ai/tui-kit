//! Integration test: a real card, through the real SVG template, through
//! the real rasterizer — the same path `CardGrid` uses at render time.

use tui_kit::{card_to_svg, rasterize, Card, Theme};

#[test]
fn temet_card_rasterizes_to_a_nonempty_image() {
    let card = Card {
        tag: "render".to_string(),
        title: "temet card".to_string(),
        subtitle: Some("svg -> png".to_string()),
        bullets: vec![
            "Sharp borders, UV glow, corner ticks".to_string(),
            "Violet-black surface, on-border title tee".to_string(),
        ],
        action: Some("Open".to_string()),
        hotkey: None,
    };
    let svg = card_to_svg(&card, &Theme::temet(), true);
    let img = rasterize(&svg, 840, Some([0x08, 0x06, 0x0f, 0xff])).expect("rasterize");
    assert!(img.width() > 0 && img.height() > 0);
    // Scaling from the SVG's intrinsic width to the 840px target is float
    // math (see render::clamp_dimensions), so the result can land a pixel
    // either side of the target rather than exactly on it.
    assert!(
        img.width().abs_diff(840) <= 1,
        "expected ~840px wide, got {}",
        img.width()
    );
}
