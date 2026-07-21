//! Card data descriptor. The toolset knows nothing about any consuming
//! app's domain types — a `Card` is a plain, app-agnostic value the caller
//! builds from whatever it actually models (a menu entry, a session, a
//! workspace...).

/// The content of a single navigable card. Purely descriptive: no styling,
/// no layout, no behavior — those live in `theme.rs`, `svg.rs`, and `nav.rs`
/// respectively.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Card {
    /// Short uppercase tag shown in the bracketed chip (e.g. "RENDER").
    pub tag: String,
    /// Card title.
    pub title: String,
    /// Optional subtitle beneath the title.
    pub subtitle: Option<String>,
    /// Bullet lines rendered with an accent square marker.
    pub bullets: Vec<String>,
    /// Optional footer action label (e.g. "Open").
    pub action: Option<String>,
    /// Optional single hotkey letter (e.g. "p"), shown in the on-border
    /// title tee as `[⌃p]` when set.
    pub hotkey: Option<String>,
}

impl Card {
    /// A stable content hash usable as the "card identity" half of an
    /// [`crate::cache::ImageCache`] key, so the cache key never needs a
    /// caller-supplied id.
    pub fn content_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

/// Per-card render state that isn't part of the card's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CardState {
    /// Whether this card is the currently focused/selected one.
    pub focused: bool,
}
