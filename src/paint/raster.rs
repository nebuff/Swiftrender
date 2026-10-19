//! Tree-order painting into a [`Canvas`] (`SPEC.md` §7).
//!
//! The contract is short and the whole module is the implementation of it: a box paints its
//! background, then its border, then its children in tree order. Opaque colours mean later
//! writes simply replace earlier ones, which is what makes a child cover its parent (`T43`).
//!
//! Two pieces of engine behaviour live here rather than in CSS: the `li` bullet (`§6.8`) and the
//! `<img>` placeholder (`§7.5`). Both are properties of the renderer, not the author, so they
//! are painted from the box tree without a computed style property to read.

use crate::css::{BorderStyle, Color};
use crate::layout::font::metrics;
use crate::layout::{BoxTree, Kind, LayoutBox};

use super::canvas::Canvas;
use super::font::{glyph, GLYPH_HEIGHT, GLYPH_WIDTH};
use super::{Rgb, WHITE};

/// Paint `tree` onto a fresh white canvas (`SPEC.md` §7.1–§7.2).
pub fn paint_tree(tree: &BoxTree) -> Canvas {
    let mut canvas = Canvas::new(tree.width, tree.height);
    canvas.clear(WHITE);
    paint_box(&mut canvas, &tree.root);
    canvas
}

/// Paint one box and, recursively, its children in tree order.
fn paint_box(canvas: &mut Canvas, node: &LayoutBox) {
    let rect = node.rect;

    // §7.1/§7.3: the background covers the border box, and transparent paints nothing.
    if let Some(color) = node.style.background {
        canvas.fill_rect(rect.x, rect.y, rect.width, rect.height, rgb(color));
    }

    paint_border(canvas, node);

    if node.kind == Kind::Atomic {
        paint_placeholder(canvas, node);
    }

    // §6.8: the marker is engine behaviour, painted left of the content box. It goes before
    // the children so a child's opaque background can still cover it if the author asks.
    if node.kind == Kind::Block && node.tag == "li" {
        paint_marker(canvas, node);
    }

    if node.kind == Kind::Text {
        paint_text(canvas, node);
    }

    for child in &node.children {
        paint_box(canvas, child);
    }
}

/// Paint a box's four solid border bands, between the padding box and the border box (`§7.4`).
fn paint_border(canvas: &mut Canvas, node: &LayoutBox) {
    if node.style.border_style != BorderStyle::Solid {
        return;
    }
    let border = node.style.effective_border();
    let top = px(border.top);
    let right = px(border.right);
    let bottom = px(border.bottom);
    let left = px(border.left);
    if top == 0 && right == 0 && bottom == 0 && left == 0 {
        return;
    }

    // §7.4: `border-color`'s initial value is the element's own `currentColor`.
    let color = rgb(node.style.border_color.unwrap_or(node.style.color));
    let rect = node.rect;

    canvas.fill_rect(rect.x, rect.y, rect.width, top, color);
    canvas.fill_rect(
        rect.x,
        rect.y + rect.height - bottom,
        rect.width,
        bottom,
        color,
    );
    canvas.fill_rect(
        rect.x,
        rect.y + top,
        left,
        rect.height - top - bottom,
        color,
    );
    canvas.fill_rect(
        rect.x + rect.width - right,
        rect.y + top,
        right,
        rect.height - top - bottom,
        color,
    );
}

/// §7.5: an `<img>` fills its content box with its own background, or the grey placeholder.
fn paint_placeholder(canvas: &mut Canvas, node: &LayoutBox) {
    let border = node.style.effective_border();
    let inset_x = px(border.left) + px(node.style.padding.left);
    let inset_y = px(border.top) + px(node.style.padding.top);
    let width = node.rect.width - inset_x - px(border.right) - px(node.style.padding.right);
    let height = node.rect.height - inset_y - px(border.bottom) - px(node.style.padding.bottom);
    let color = node.style.background.map(rgb).unwrap_or(Rgb {
        r: 0xc0,
        g: 0xc0,
        b: 0xc0,
    });
    canvas.fill_rect(
        node.rect.x + inset_x,
        node.rect.y + inset_y,
        width.max(0),
        height.max(0),
        color,
    );
}

/// §6.8: paint the `li` bullet 16px left of the content box, on the first line.
fn paint_marker(canvas: &mut Canvas, node: &LayoutBox) {
    let border = node.style.effective_border();
    let content_left = node.rect.x + px(border.left) + px(node.style.padding.left);
    let content_top = node.rect.y + px(border.top) + px(node.style.padding.top);
    let metrics = metrics(&node.style);
    let font_size = node.style.font_size;
    let line_height = px(metrics.line_height);
    let cell_width = px(metrics.advance).max(1);
    let top = content_top + px((f64::from(line_height - font_size)) / 2.0);

    // The bullet is a small filled square inside the 8×16 marker cell. It stays clear of the
    // content box's left edge, which is what `T47` checks at x = 40.
    let size = (font_size / 4).max(2);
    let x = content_left - 16 + (cell_width - size) / 2;
    let y = top + (font_size - size) / 2;
    canvas.fill_rect(x, y, size, size, rgb(node.style.color));
}

/// Paint every glyph of a text run, in `color` (`§7.6`).
fn paint_text(canvas: &mut Canvas, node: &LayoutBox) {
    let Some(text) = &node.text else {
        return;
    };
    let metrics = metrics(&node.style);
    let advance = px(metrics.advance).max(1);
    let color = rgb(node.style.color);
    let cell_height = node.style.font_size;

    let mut x = node.rect.x;
    for ch in text.chars() {
        if let Some(mask) = glyph(ch) {
            blit(canvas, x, node.rect.y, advance, cell_height, mask, color);
        }
        x += advance;
    }
}

/// Scale one 8×16 mask to `cell_width` × `cell_height` and paint its set bits (`§6.6`, `§7.6`).
///
/// The scaling is nearest-neighbour by integer division, so a 16px cell is a straight copy and a
/// 32px cell doubles every source pixel. There is no interpolation, hence no anti-aliasing.
fn blit(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    cell_width: i32,
    cell_height: i32,
    mask: &[u8; GLYPH_HEIGHT],
    color: Rgb,
) {
    if cell_width <= 0 || cell_height <= 0 {
        return;
    }
    for dy in 0..cell_height {
        let source_y = dy as usize * GLYPH_HEIGHT / cell_height as usize;
        let row = mask[source_y];
        for dx in 0..cell_width {
            let source_x = dx as usize * GLYPH_WIDTH / cell_width as usize;
            if row & (0x80 >> source_x) != 0 {
                canvas.fill_rect(x + dx, y + dy, 1, 1, color);
            }
        }
    }
}

/// Convert a computed colour into the canvas's channel type.
fn rgb(color: Color) -> Rgb {
    color.into()
}

/// Round an already-computed CSS length to its single integer pixel (`SPEC.md` §6.7).
fn px(value: f64) -> i32 {
    value.round() as i32
}
