//! Built-in font metrics (`SPEC.md` §6.6).
//!
//! M1 has exactly one font: an 8×16 bitmap, monospaced, scalable by integer factors. Everything
//! measurable about text comes from these three numbers, which is why they live in one place with
//! their own tests rather than being recomputed at each call site.
//!
//! Two rules the tests lean on:
//!
//! * `advance = max(1, round(font_size / 2))`, so 16px text advances 8px per glyph and 32px text
//!   advances 16px (`TESTS.md` T2, T27, T40).
//! * The glyph cell is `advance × font_size` and is **vertically centred** in the line box
//!   (`SPEC.md` §6.5), which is why a 16px glyph in a 20px line starts 2px down, not at the top.

use crate::css::ComputedStyle;

/// Everything the layout needs to know about how text measures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// Horizontal distance between glyph origins, in pixels.
    pub advance: f64,
    /// Height of the glyph cell, equal to the font size.
    pub cell_height: f64,
    /// Height of one line box, from `line-height` resolved against `font-size`.
    pub line_height: f64,
}

/// Compute the metrics for `style`.
///
/// # Examples
///
/// ```
/// use swiftrender::css::ComputedStyle;
/// use swiftrender::layout::font::metrics;
/// let style = ComputedStyle::initial();
/// assert_eq!(metrics(&style).advance, 8.0);
/// assert_eq!(metrics(&style).line_height, 20.0); // line-height: normal = 1.25 × 16
/// ```
pub fn metrics(style: &ComputedStyle) -> Metrics {
    let font_size = f64::from(style.font_size);
    Metrics {
        advance: (font_size / 2.0).round().max(1.0),
        cell_height: font_size,
        line_height: style.line_height.resolve(font_size),
    }
}

/// Width of a collapsed text run with `glyphs` characters.
pub fn text_width(glyphs: usize, metrics: Metrics) -> f64 {
    glyphs as f64 * metrics.advance
}

/// The glyph cell's top edge inside a line box of height `line_height`, rounded once
/// (`SPEC.md` §6.5, §6.7).
pub fn cell_top(line_top: f64, line_height: f64, cell_height: f64) -> f64 {
    line_top + ((line_height - cell_height) / 2.0).round()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::{Display, LineHeight, TextAlign};

    /// A style with the given font size and default everything else.
    fn style_with(font_size: i32, line_height: LineHeight) -> ComputedStyle {
        let mut style = ComputedStyle::initial();
        style.display = Display::Block;
        style.text_align = TextAlign::Left;
        style.font_size = font_size;
        style.line_height = line_height;
        style
    }

    /// §6.6: the advance is half the font size, rounded, and never zero.
    #[test]
    fn advance_is_half_the_font_size() {
        assert_eq!(metrics(&style_with(16, LineHeight::Normal)).advance, 8.0);
        assert_eq!(metrics(&style_with(32, LineHeight::Normal)).advance, 16.0);
        assert_eq!(metrics(&style_with(20, LineHeight::Normal)).advance, 10.0);
        assert_eq!(metrics(&style_with(1, LineHeight::Normal)).advance, 1.0);
    }

    /// §3.8: `normal` is 1.25 × font-size; a factor multiplies; a length is absolute.
    #[test]
    fn line_height_forms_resolve_against_font_size() {
        assert_eq!(
            metrics(&style_with(16, LineHeight::Normal)).line_height,
            20.0
        );
        assert_eq!(
            metrics(&style_with(32, LineHeight::Normal)).line_height,
            40.0
        );
        assert_eq!(
            metrics(&style_with(16, LineHeight::Number(1.5))).line_height,
            24.0
        );
        assert_eq!(
            metrics(&style_with(32, LineHeight::Px(10.0))).line_height,
            10.0
        );
    }

    /// §6.5: the cell is centred, and the rounding happens here rather than at paint time.
    #[test]
    fn cells_are_centred_in_their_line_box() {
        assert_eq!(cell_top(24.0, 20.0, 16.0), 26.0);
        assert_eq!(cell_top(24.0, 40.0, 32.0), 28.0);
        assert_eq!(cell_top(0.0, 20.0, 10.0), 5.0);
    }

    /// A wider glyph run measures further.
    #[test]
    fn width_scales_with_glyph_count() {
        let metrics = metrics(&style_with(16, LineHeight::Normal));
        assert_eq!(text_width(2, metrics), 16.0);
        assert_eq!(text_width(3, metrics), 24.0);
        assert_eq!(text_width(7, metrics), 56.0);
    }
}
