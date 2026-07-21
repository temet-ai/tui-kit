//! Cache the resized/encoded image protocol so navigating the grid (or a
//! static frame) doesn't re-rasterize a card's SVG on every draw. Keyed by
//! `(card identity, focused, area)` — `card identity` is
//! [`crate::card::Card::content_hash`], so the key never needs a
//! caller-supplied id and two structurally-identical cards share a cache
//! entry.

use std::collections::HashMap;

use ratatui::layout::Rect;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CacheKey {
    identity: u64,
    focused: bool,
    area: Rect,
}

/// A cache of encoded image protocols, one per `(card, focused, area)`
/// combination seen so far. A plain growing `HashMap` — callers that render
/// a bounded, static set of cards never need eviction; a caller that swaps
/// its whole card set repeatedly should call [`ImageCache::clear`].
#[derive(Default)]
pub struct ImageCache {
    entries: HashMap<CacheKey, StatefulProtocol>,
}

impl ImageCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the cached protocol for `(identity, focused, area)`, building
    /// one via `picker` from `build_image()` on a cache miss. `build_image`
    /// (the SVG-to-PNG rasterize path) runs only on a miss.
    pub fn get_or_insert_with(
        &mut self,
        identity: u64,
        focused: bool,
        area: Rect,
        picker: &Picker,
        build_image: impl FnOnce() -> image::DynamicImage,
    ) -> &mut StatefulProtocol {
        let key = CacheKey {
            identity,
            focused,
            area,
        };
        self.entries
            .entry(key)
            .or_insert_with(|| picker.new_resize_protocol(build_image()))
    }

    /// Number of cached protocols.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache currently holds no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drop every cached protocol (e.g. after swapping in a whole new card
    /// set, or a theme change that invalidates every rasterized SVG).
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_hit_does_not_rebuild() {
        let picker = Picker::from_fontsize((8, 16));
        let mut cache = ImageCache::new();
        let area = Rect::new(0, 0, 20, 10);

        let mut builds = 0;
        cache.get_or_insert_with(1, false, area, &picker, || {
            builds += 1;
            image::DynamicImage::new_rgba8(4, 4)
        });
        assert_eq!(cache.len(), 1);

        // Same key again: must not call the builder.
        cache.get_or_insert_with(1, false, area, &picker, || {
            builds += 1;
            image::DynamicImage::new_rgba8(4, 4)
        });
        assert_eq!(builds, 1, "a cache hit must not re-rasterize");
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn distinct_focus_state_is_a_distinct_entry() {
        let picker = Picker::from_fontsize((8, 16));
        let mut cache = ImageCache::new();
        let area = Rect::new(0, 0, 20, 10);

        cache.get_or_insert_with(1, false, area, &picker, || image::DynamicImage::new_rgba8(4, 4));
        cache.get_or_insert_with(1, true, area, &picker, || image::DynamicImage::new_rgba8(4, 4));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn clear_drops_all_entries() {
        let picker = Picker::from_fontsize((8, 16));
        let mut cache = ImageCache::new();
        let area = Rect::new(0, 0, 20, 10);
        cache.get_or_insert_with(1, false, area, &picker, || image::DynamicImage::new_rgba8(4, 4));
        assert!(!cache.is_empty());
        cache.clear();
        assert!(cache.is_empty());
    }
}
