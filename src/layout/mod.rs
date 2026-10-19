//! Box generation and layout (`SPEC.md` §5–§6).
//!
//! **Phase 2.** Nothing here is implemented yet.
//!
//! Planned surface:
//!
//! * `box_tree::build(&Document, &ComputedStyles, &str) -> BoxTree` — display resolution,
//!   anonymous-block wrapping (§5.3) and `display: none` suppression (§5.1).
//! * `block::layout(...)` — block formatting context, no margin collapsing (§6.3).
//! * `inline::layout(...)` — greedy line breaking, whitespace collapsing and `text-align`
//!   (§6.5).
//! * `font::metrics(font_size) -> GlyphMetrics` — the advance/cell formula of §6.6.
//! * `snap(f64) -> i32` — round-half-up onto the integer pixel grid, applied exactly once
//!   (§6.7). Every `TESTS.md` geometry assertion is on snapped values, so this function is
//!   the single place rounding is allowed to happen.

/// Geometry of one box in integer pixels after snapping (`SPEC.md` §6.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    /// Left edge in pixels.
    pub x: i32,
    /// Top edge in pixels.
    pub y: i32,
    /// Width in pixels.
    pub width: i32,
    /// Height in pixels.
    pub height: i32,
}
