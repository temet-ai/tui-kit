//! The `CardGrid` widget: lays out N cards into a grid and places each
//! card's rasterized image into its sub-rect via `ratatui-image` — real
//! terminal graphics (or its automatic half-block fallback), never
//! box-drawing cells.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::StatefulWidget;
use ratatui_image::StatefulImage;
use ratatui_image::picker::Picker;

use crate::cache::ImageCache;
use crate::card::Card;
use crate::nav::NavState;
use crate::render;
use crate::svg;
use crate::theme::Theme;

/// A grid of temet-styled cards, each placed as a rasterized image sized to
/// its cell. Column count and the focused index come from the
/// [`NavState`] passed to `render` — `CardGrid` itself carries no nav
/// state, so it stays a cheap per-frame value built from borrows.
pub struct CardGrid<'a> {
    cards: &'a [Card],
    theme: &'a Theme,
    picker: &'a Picker,
    cache: &'a mut ImageCache,
}

impl<'a> CardGrid<'a> {
    /// Build a grid over `cards`, styled by `theme`, rasterized/sized via
    /// `picker`'s detected font metrics, caching encoded protocols in
    /// `cache`.
    pub fn new(
        cards: &'a [Card],
        theme: &'a Theme,
        picker: &'a Picker,
        cache: &'a mut ImageCache,
    ) -> Self {
        Self {
            cards,
            theme,
            picker,
            cache,
        }
    }
}

impl<'a> StatefulWidget for CardGrid<'a> {
    type State = NavState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut NavState) {
        let len = self.cards.len().min(state.len);
        if len == 0 || area.width == 0 || area.height == 0 {
            return;
        }
        let columns = state.columns.max(1);
        let rows = len.div_ceil(columns);

        let row_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Ratio(1, rows as u32); rows])
            .split(area);

        let (font_w, _font_h) = self.picker.font_size();

        for row in 0..rows {
            let row_area = row_areas[row];
            let cols_in_row = columns.min(len - row * columns);
            let col_areas = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Ratio(1, columns as u32); columns])
                .split(row_area);

            for col in 0..cols_in_row {
                let idx = row * columns + col;
                let card = &self.cards[idx];
                let sub_rect = col_areas[col];
                if sub_rect.width == 0 || sub_rect.height == 0 {
                    continue;
                }
                let focused = idx == state.selected;
                let identity = card.content_hash();
                let target_width_px = (sub_rect.width as u32 * font_w as u32).max(1);

                let theme = self.theme;
                let picker = self.picker;
                let protocol = self.cache.get_or_insert_with(
                    identity,
                    focused,
                    sub_rect,
                    picker,
                    || {
                        let svg = svg::card_to_svg(card, theme, focused);
                        render::rasterize(&svg, target_width_px, Some(theme.bg_rgba()))
                            .unwrap_or_else(|_| fallback_image(theme, target_width_px))
                    },
                );

                StatefulImage::default().render(sub_rect, buf, protocol);
            }
        }
    }
}

/// A flat tile in the theme's background color, used only if `rasterize`
/// ever fails on our own well-formed generated SVG (so a render failure
/// degrades to a blank card instead of dropping the whole grid).
fn fallback_image(theme: &Theme, width_px: u32) -> image::DynamicImage {
    let [r, g, b, a] = theme.bg_rgba();
    let w = width_px.max(1);
    let h = ((w as f32 * svg::CARD_H / svg::CARD_W) as u32).max(1);
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        w,
        h,
        image::Rgba([r, g, b, a]),
    ))
}
