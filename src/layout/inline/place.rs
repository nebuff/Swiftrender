//! Placement half of inline formatting: rewriting the box tree from the line-breaking result.
//!
//! Split out of `inline.rs` so both files stay inside the 400-line budget (`SPEC.md` §10.1). This
//! half owns exactly three things: turning placed cells back into `LayoutBox`es, accumulating
//! per-line text fragments, and giving each inline element the rectangle covering its fragments.

use super::super::font::cell_top;
use super::super::tree::{Kind, LayoutBox};
use super::super::Rect;
use super::{Cell, CellKind, Owner};
use crate::css::ComputedStyle;

/// A cell placed on a line.
pub(super) struct Placed {
    /// Index into the cell list.
    pub(super) cell: usize,
    /// Absolute left edge in pixels.
    pub(super) x: f64,
    /// Zero-based line index.
    pub(super) line: usize,
    /// Absolute top of the line box.
    pub(super) line_top: f64,
    /// Height of the line box.
    pub(super) line_height: f64,
}
/// A text fragment being accumulated across consecutive cells of one source on one line.
struct PendingText {
    /// Source text box this fragment came from.
    source: usize,
    /// Line index.
    line: usize,
    /// Left edge.
    x: f64,
    /// Glyph cell top.
    top: f64,
    /// Accumulated width.
    width: f64,
    /// Accumulated text.
    text: String,
    /// Style to copy onto the fragment box.
    style: ComputedStyle,
}

/// Replace the container's inline children with positioned fragments (`SPEC.md` §6.5, §6.7).
pub(super) fn rebuild(
    container: &mut LayoutBox,
    cells: &[Cell],
    owners: &[Owner],
    placed: &[Placed],
) {
    let mut children: Vec<LayoutBox> = Vec::new();
    let mut owner_slot: Vec<Option<usize>> = vec![None; owners.len()];
    let mut owner_extent: Vec<Option<(f64, f64, f64, f64)>> = vec![None; owners.len()];
    let mut pending: Option<PendingText> = None;

    for spot in placed {
        let cell = &cells[spot.cell];
        if let Some(owner) = cell.owner {
            if owner_slot[owner].is_none() {
                let source = &owners[owner];
                let mut node = LayoutBox::new(Kind::Inline, &source.tag, source.style.clone());
                node.classes = source.classes.clone();
                children.push(node);
                owner_slot[owner] = Some(children.len() - 1);
            }
            let extent = (
                spot.x,
                spot.x + cell.width,
                spot.line_top,
                spot.line_top + spot.line_height,
            );
            owner_extent[owner] = Some(match owner_extent[owner] {
                None => extent,
                Some(current) => (
                    current.0.min(extent.0),
                    current.1.max(extent.1),
                    current.2.min(extent.2),
                    current.3.max(extent.3),
                ),
            });
        }

        match cell.kind {
            CellKind::Word | CellKind::Space => {
                let top = cell_top(spot.line_top, spot.line_height, cell.font_size as f64);
                let same_fragment = matches!(
                    &pending,
                    Some(text) if text.source == cell.source && text.line == spot.line
                );
                if same_fragment {
                    if let Some(text) = pending.as_mut() {
                        text.text.push_str(&cell.text);
                        text.width = spot.x + cell.width - text.x;
                    }
                } else {
                    flush_text(&mut pending, &mut children);
                    pending = Some(PendingText {
                        source: cell.source,
                        line: spot.line,
                        x: spot.x,
                        top,
                        width: cell.width,
                        text: cell.text.clone(),
                        style: cell.style.clone(),
                    });
                }
            }
            CellKind::Atom => {
                flush_text(&mut pending, &mut children);
                if let Some((tag, width, height)) = &cell.atom {
                    let mut node = LayoutBox::new(Kind::Atomic, tag, cell.style.clone());
                    node.rect = Rect {
                        x: spot.x.round() as i32,
                        y: (spot.line_top + ((spot.line_height - height) / 2.0).round()).round()
                            as i32,
                        width: width.round() as i32,
                        height: height.round() as i32,
                    };
                    children.push(node);
                }
            }
            CellKind::Break => flush_text(&mut pending, &mut children),
        }
    }
    flush_text(&mut pending, &mut children);

    for (index, slot) in owner_slot.iter().enumerate() {
        if let (Some(child), Some(extent)) = (slot, owner_extent[index]) {
            children[*child].rect = Rect {
                x: extent.0.round() as i32,
                y: extent.2.round() as i32,
                width: (extent.1 - extent.0).round() as i32,
                height: (extent.3 - extent.2).round() as i32,
            };
        }
    }

    container.children = children;
}

/// Emit the accumulated text fragment, if any.
///
/// The fragment keeps the source box's inherited properties (colour, font, alignment) but drops
/// its box model: a text run has no margins, padding or border of its own, and leaving the parent
/// paragraph's margins on it made the dump read as though the text were an inset box.
fn flush_text(pending: &mut Option<PendingText>, children: &mut Vec<LayoutBox>) {
    let Some(text) = pending.take() else { return };
    let mut style = text.style;
    style.margin = crate::css::Edges::all(0.0);
    style.padding = crate::css::Edges::all(0.0);
    style.border_width = crate::css::Edges::all(0.0);
    style.border_style = crate::css::BorderStyle::None;
    style.background = None;
    let mut node = LayoutBox::new(Kind::Text, "#text", style);
    node.rect = Rect {
        x: text.x.round() as i32,
        y: text.top.round() as i32,
        width: text.width.round() as i32,
        height: 0, // set below, from the font size
    };
    node.rect.height = node.style.font_size;
    node.text = Some(text.text);
    children.push(node);
}
