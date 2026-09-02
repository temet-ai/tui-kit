//! The temet card look, authored as SVG. This is the single place (along
//! with `theme.rs`) that owns visual styling — colors always come from the
//! `&Theme` argument, never a literal.
//!
//! temet is a sharp-edged design system: radius 0 everywhere (no rounding,
//! no bevels/chamfers), 1px hairline borders, and a UV-purple accent that
//! shows up as a glowing border + corner "reticle" ticks + an on-border
//! title tee only when the card is focused.

use crate::card::Card;
use crate::theme::Theme;

/// Fixed card canvas size in SVG user units. Every card renders at this
/// aspect ratio so a `CardGrid` can lay out uniform sub-rects; `render::rasterize`
/// derives pixel height from this ratio and the caller's target width.
pub(crate) const CARD_W: f32 = 400.0;
pub(crate) const CARD_H: f32 = 248.0;

/// The panel's inset from the canvas edge — the `app`-colored gutter that
/// shows a tiled grid of cards has visible seams, and headroom for the
/// focused glow/corner ticks to render without clipping at the canvas edge.
const PANEL_X: f32 = 16.0;
const PANEL_Y: f32 = 16.0;
const PANEL_W: f32 = CARD_W - PANEL_X * 2.0;
const PANEL_H: f32 = CARD_H - PANEL_Y * 2.0;

/// Bullets beyond this count are dropped rather than overflowing the fixed
/// card canvas (there is no text-reflow/measurement engine here).
const MAX_BULLETS: usize = 3;

/// Render `card` as a self-contained SVG document using `theme`'s colors: a
/// sharp-cornered temet panel with an on-border title tee, an idle or
/// UV-glowing focus border, and (when focused) four corner reticle ticks.
pub fn card_to_svg(card: &Card, theme: &Theme, focused: bool) -> String {
    let app = hex(theme.app);
    let panel = hex(theme.panel);
    let border = if focused {
        hex(theme.accent)
    } else {
        hex(theme.border)
    };
    let accent = hex(theme.accent);
    let text = hex(theme.text);
    let muted = hex(theme.muted);
    let faint = hex(theme.faint);
    let title_color = if focused {
        hex(theme.accent)
    } else {
        hex(theme.muted)
    };
    let scanline_color = hex(theme.text);

    let panel_right = PANEL_X + PANEL_W;
    let content_x = PANEL_X + 16.0;
    let content_right = panel_right - 16.0;

    let tag = xml_escape(&card.tag.to_uppercase());
    let title = xml_escape(&truncate(&card.title, 40));
    let subtitle = card
        .subtitle
        .as_deref()
        .map(|s| xml_escape(&truncate(s, 56)));

    // -- title tee: "┤ TAG ├" (or "┤ TAG [⌃x] ├" with a hotkey hint)
    // sitting on the top border line -------------------------------------
    let tee_prefix = format!("\u{2524} {tag} ");
    let tee_suffix = "\u{251c}";
    let hotkey_hint = card.hotkey.as_deref().map(xml_escape);
    // Dimmer than the tag/title tee color so the hint reads as a secondary
    // annotation, not a second title. Focused: a darkened accent (still
    // ultraviolet, never a new literal hue). Unfocused: the theme's own
    // faintest tone.
    let hotkey_color = if focused {
        hex(dim(theme.accent, 0.45))
    } else {
        faint.clone()
    };
    let (tee_hint_span, tee_char_count) = match &hotkey_hint {
        Some(hint) => {
            let hint_text = format!("[\u{2303}{hint}] ");
            let span = format!(r#"<tspan fill="{hotkey_color}">{hint_text}</tspan>"#);
            (
                span,
                tee_prefix.chars().count() + hint_text.chars().count() + tee_suffix.chars().count(),
            )
        }
        None => (
            String::new(),
            tee_prefix.chars().count() + tee_suffix.chars().count(),
        ),
    };
    let tee_text = format!("{tee_prefix}{tee_hint_span}{tee_suffix}");
    // No font metrics are loaded for this pure-string SVG builder (same
    // constraint as `truncate`'s char-count bound), so the backing rect's
    // width is an estimate from a fixed per-char advance for this
    // bold/letter-spaced font-size, not a measured value.
    let tee_width = tee_char_count as f32 * 8.5 + 8.0;
    let tee_backing_x = content_x - 4.0;
    let tee_backing_y = PANEL_Y - 8.0;

    // -- corner reticle ticks (focused only) ------------------------------
    let ticks_svg = if focused {
        corner_ticks(&accent)
    } else {
        String::new()
    };

    // -- bullets -----------------------------------------------------------
    // Sourced from `theme::GlyphSet` (not a literal here) so the bullet/arrow
    // marks stay defined in one place alongside the rest of the palette.
    let glyphs = crate::theme::GlyphSet::default();
    let mut bullets_svg = String::new();
    let shown_bullets = card.bullets.iter().take(MAX_BULLETS).count();
    for (i, bullet) in card.bullets.iter().take(MAX_BULLETS).enumerate() {
        let y = 116.0 + i as f32 * 22.0;
        let bullet_text = xml_escape(&truncate(bullet, 50));
        bullets_svg.push_str(&format!(
            r##"  <text x="{content_x}" y="{y}" font-family="monospace" font-size="13" fill="{accent}">{bullet_glyph}</text>
  <text x="{bx}" y="{y}" font-family="sans-serif" font-size="12.5" fill="{text}">{bullet_text}</text>
"##,
            bx = content_x + 16.0,
            bullet_glyph = glyphs.bullet,
        ));
    }
    let overflow = card.bullets.len().saturating_sub(shown_bullets);
    if overflow > 0 {
        let y = 116.0 + shown_bullets as f32 * 22.0;
        bullets_svg.push_str(&format!(
            r##"  <text x="{bx}" y="{y}" font-family="sans-serif" font-size="11" fill="{faint}">+{overflow} more</text>
"##,
            bx = content_x + 16.0,
        ));
    }

    let action_svg = card
        .action
        .as_deref()
        .map(|action| {
            let action_text = xml_escape(&truncate(action, 36));
            format!(
                r##"  <text x="{content_x}" y="210" font-family="monospace" font-size="12.5" font-weight="700" letter-spacing="1" fill="{accent}">{action_text} {arrow}</text>"##,
                arrow = glyphs.arrow,
            )
        })
        .unwrap_or_default();

    let subtitle_svg = subtitle
        .map(|s| {
            format!(
                r##"  <text x="{content_x}" y="76" font-family="monospace" font-size="12" letter-spacing="1" fill="{muted}">{s}</text>"##
            )
        })
        .unwrap_or_default();

    let border_filter = if focused {
        r#" filter="url(#uvGlow)""#
    } else {
        ""
    };
    let title_filter = if focused {
        r#" filter="url(#titleGlow)""#
    } else {
        ""
    };

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{cw}" height="{ch}" viewBox="0 0 {cw} {ch}">
  <defs>
    <filter id="uvGlow" x="-5%" y="-8%" width="110%" height="116%">
      <feDropShadow dx="0" dy="0" stdDeviation="5" flood-color="{accent}" flood-opacity="0.45"/>
    </filter>
    <filter id="tickGlow" x="-60%" y="-60%" width="220%" height="220%">
      <feDropShadow dx="0" dy="0" stdDeviation="3" flood-color="{accent}" flood-opacity="0.6"/>
    </filter>
    <filter id="titleGlow" x="-20%" y="-150%" width="140%" height="400%">
      <feDropShadow dx="0" dy="0" stdDeviation="2" flood-color="{accent}" flood-opacity="0.55"/>
    </filter>
    <pattern id="scan" width="1" height="2" patternUnits="userSpaceOnUse">
      <rect width="1" height="1" fill="{scanline_color}" opacity="0.03"/>
    </pattern>
  </defs>

  <rect x="0" y="0" width="{cw}" height="{ch}" fill="{app}"/>

  <rect x="{px}" y="{py}" width="{pw}" height="{ph}" fill="{panel}" stroke="{border}" stroke-width="1"{border_filter}/>
  <rect x="{px}" y="{py}" width="{pw}" height="{ph}" fill="url(#scan)"/>
{ticks_svg}
  <rect x="{tee_backing_x}" y="{tee_backing_y}" width="{tee_width}" height="16" fill="{panel}"/>
  <text x="{content_x}" y="20" font-family="monospace" font-size="12" font-weight="700"
        letter-spacing="1.4" fill="{title_color}"{title_filter}>{tee_text}</text>

  <text x="{content_x}" y="58" font-family="sans-serif" font-size="17" font-weight="700" fill="{text}">{title}</text>
{subtitle_svg}

  <line x1="{content_x}" y1="94" x2="{content_right}" y2="94" stroke="{border_idle}" stroke-width="1"/>

{bullets_svg}
{action_svg}
</svg>"##,
        cw = CARD_W,
        ch = CARD_H,
        px = PANEL_X,
        py = PANEL_Y,
        pw = PANEL_W,
        ph = PANEL_H,
        border_idle = hex(theme.border),
        content_right = content_right,
    )
}

/// Four 2px L-bracket "reticle" ticks just outside the panel's corners,
/// shown only when the card is focused.
fn corner_ticks(accent: &str) -> String {
    let x0 = PANEL_X;
    let y0 = PANEL_Y;
    let x1 = PANEL_X + PANEL_W;
    let y1 = PANEL_Y + PANEL_H;
    // Offset outside the border, and short L-arms (~1.5ch x ~.7em at the
    // card's body font size).
    let o = 5.0;
    let ah = 12.0;
    let av = 9.0;
    format!(
        r##"  <path d="M{tlx} {tly1} L{tlx} {tly0} L{tlx2} {tly0}" fill="none" stroke="{accent}" stroke-width="2" filter="url(#tickGlow)"/>
  <path d="M{trx1} {try0} L{trx} {try0} L{trx} {try1}" fill="none" stroke="{accent}" stroke-width="2" filter="url(#tickGlow)"/>
  <path d="M{blx} {bly1} L{blx} {bly0} L{blx2} {bly0}" fill="none" stroke="{accent}" stroke-width="2" filter="url(#tickGlow)"/>
  <path d="M{brx1} {bry0} L{brx} {bry0} L{brx} {bry1}" fill="none" stroke="{accent}" stroke-width="2" filter="url(#tickGlow)"/>
"##,
        tlx = x0 - o,
        tly0 = y0 - o,
        tly1 = y0 - o + av,
        tlx2 = x0 - o + ah,
        trx = x1 + o,
        try0 = y0 - o,
        try1 = y0 - o + av,
        trx1 = x1 + o - ah,
        blx = x0 - o,
        bly0 = y1 + o,
        bly1 = y1 + o - av,
        blx2 = x0 - o + ah,
        brx = x1 + o,
        bry0 = y1 + o,
        bry1 = y1 + o - av,
        brx1 = x1 + o - ah,
    )
}

/// Truncate to at most `max_chars` characters (char-count, not byte-count,
/// so multi-byte UTF-8 never splits mid-codepoint), appending an ellipsis
/// when truncated — there is no text-reflow engine, so overlong strings
/// would otherwise overrun the fixed card canvas.
fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

/// Escape the five XML predefined entities so arbitrary card text can never
/// break out of an SVG text node or attribute.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Darkens `c` toward black by `factor` (0.0 = unchanged, 1.0 = black), so a
/// dimmer variant of a theme color (e.g. the hotkey hint vs. the title tee)
/// can be derived without introducing a new color literal.
fn dim(c: ratatui::style::Color, factor: f32) -> ratatui::style::Color {
    match c {
        ratatui::style::Color::Rgb(r, g, b) => {
            let scale = 1.0 - factor;
            ratatui::style::Color::Rgb(
                (r as f32 * scale) as u8,
                (g as f32 * scale) as u8,
                (b as f32 * scale) as u8,
            )
        }
        other => other,
    }
}

fn hex(c: ratatui::style::Color) -> String {
    match c {
        ratatui::style::Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        // Non-RGB Color variants (e.g. named/indexed) fall back to temet's
        // own UV accent (never teal, never amber) so a mis-themed card
        // still renders visibly rather than silently going transparent.
        _ => "#9d5cff".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Card;

    fn sample() -> Card {
        Card {
            tag: "render".to_string(),
            title: "temet card".to_string(),
            subtitle: Some("svg -> png".to_string()),
            bullets: vec!["one".to_string(), "two".to_string()],
            action: Some("Open".to_string()),
            hotkey: None,
        }
    }

    #[test]
    fn escapes_hostile_text() {
        let mut card = sample();
        card.title = "<script>&\"'".to_string();
        let svg = card_to_svg(&card, &Theme::temet(), false);
        assert!(!svg.contains("<script>"));
        assert!(svg.contains("&lt;script&gt;"));
    }

    #[test]
    fn focused_uses_accent_border() {
        let card = sample();
        let theme = Theme::temet();
        let unfocused = card_to_svg(&card, &theme, false);
        let focused = card_to_svg(&card, &theme, true);
        assert_ne!(unfocused, focused);
    }

    #[test]
    fn focused_shows_corner_ticks_unfocused_does_not() {
        // Both variants declare the `tickGlow` filter in `<defs>` (static
        // markup), so check for its *usage* on a tick `<path>` rather than
        // the bare id, which would match the unfocused variant too.
        let card = sample();
        let theme = Theme::temet();
        let unfocused = card_to_svg(&card, &theme, false);
        let focused = card_to_svg(&card, &theme, true);
        let usage = r#"filter="url(#tickGlow)""#;
        assert!(!unfocused.contains(usage));
        assert!(focused.contains(usage));
    }

    #[test]
    fn produces_well_formed_svg_root() {
        let svg = card_to_svg(&sample(), &Theme::temet(), false);
        assert!(svg.trim_start().starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
    }

    #[test]
    fn hotkey_hint_renders_only_when_set() {
        let mut card = sample();
        card.hotkey = Some("p".to_string());
        let with_hint = card_to_svg(&card, &Theme::temet(), false);
        assert!(with_hint.contains("["));
        assert!(with_hint.contains("\u{2303}p"));

        card.hotkey = None;
        let without_hint = card_to_svg(&card, &Theme::temet(), false);
        assert!(!without_hint.contains("[\u{2303}"));
    }

    #[test]
    fn never_uses_teal_or_amber() {
        let theme = Theme::temet();
        for focused in [false, true] {
            let svg = card_to_svg(&sample(), &theme, focused);
            assert!(!svg.contains("#00d9c0"), "teal literal leaked into SVG");
            assert!(
                !svg.contains("#ffbf00") && !svg.contains("#ffb000"),
                "amber literal leaked into SVG"
            );
        }
    }
}
