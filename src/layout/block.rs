//! Block layout (`SPEC.md` §6.2–§6.4).
//!
//! Normal flow only: no floats, no positioning, no margin collapsing. A block's children are
//! stacked vertically inside its content box, and a block's auto height is the distance to the last
//! child's **bottom margin edge** (§6.4) — which is why the return value of [`layout`] includes the
//! box's own vertical margins while [`super::tree::LayoutBox::rect`] does not.

use super::inline::layout_inline;
use super::tree::LayoutBox;
use super::Rect;

/// Lay `node` out inside a containing block whose content box starts at `cb_x, cb_y`.
///
/// Returns the vertical space consumed, including this box's top and bottom margins, so the
/// caller can simply advance its cursor (`SPEC.md` §6.3).
pub fn layout(node: &mut LayoutBox, cb_x: f64, cb_y: f64, cb_width: f64) -> f64 {
    let style = node.style.clone();
    let border = style.effective_border();
    let padding = style.padding;

    // §6.2: an explicit width is a *border-box* width in M1 (see SPEC.md §6.2a), so padding and
    // border eat into it rather than growing it; `auto` fills the containing block minus
    // everything that sits outside the content box.
    let content_width = match style.width {
        Some(width) => (width - padding.horizontal() - border.horizontal()).max(0.0),
        None => (cb_width - style.margin.horizontal() - border.horizontal() - padding.horizontal())
            .max(0.0),
    };
    let border_box_width = content_width + padding.horizontal() + border.horizontal();

    let border_x = cb_x + style.margin.left;
    let border_y = cb_y + style.margin.top;
    let content_x = border_x + border.left + padding.left;
    let content_top = border_y + border.top + padding.top;

    let content_height = if node.children.iter().any(LayoutBox::is_inline_level) {
        layout_inline(node, content_x, content_width, content_top)
    } else {
        let mut cursor = content_top;
        for child in node.children.iter_mut() {
            cursor += layout(child, content_x, cursor, content_width);
        }
        cursor - content_top
    };

    // §6.4: an explicit height wins and the box is not shrunk by its content; children may
    // overflow it, and §7.7 means nothing is clipped. For `auto`, the content height is only
    // the distance between content edges, so padding and border must be added back to reach the
    // border-box height the dump reports (SPEC.md §6.2a). Missing this makes a padded box's
    // background too short and lets its text paint outside the box — visible in the first
    // full-feature render even though the Phase 2 geometry tests did not catch it.
    let border_box_height = match style.height {
        Some(height) => height.max(0.0),
        None => content_height + padding.vertical() + border.vertical(),
    };

    node.rect = Rect {
        x: border_x.round() as i32,
        y: border_y.round() as i32,
        width: border_box_width.round() as i32,
        height: border_box_height.round() as i32,
    };

    border_box_height + style.margin.vertical()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::Css;
    use crate::html::parse as parse_html;

    /// Build and lay out a document at 800×600.
    fn boxes(html: &str) -> LayoutBox {
        let document = parse_html(html);
        let mut warnings = Vec::new();
        let css = Css::from_document(&document, &mut warnings);
        let mut root = super::super::boxgen::build(&document, &css, &mut warnings);
        layout(&mut root, 0.0, 0.0, 800.0);
        root
    }

    /// Find a box by tag in pre-order.
    fn find<'a>(node: &'a LayoutBox, tag: &str) -> &'a LayoutBox {
        find_or_none(node, tag).unwrap_or_else(|| panic!("no {tag} box in {node:#?}"))
    }

    /// The `Option`-returning half of [`find`].
    fn find_or_none<'a>(node: &'a LayoutBox, tag: &str) -> Option<&'a LayoutBox> {
        if node.tag == tag {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| find_or_none(child, tag))
    }

    /// §6.2 and §6.4: the whole auto height chain for `<p>hi</p>` (T2).
    #[test]
    fn paragraph_geometry_matches_the_spec() {
        let root = boxes("<p>hi</p>");
        let paragraph = find(&root, "p");
        assert_eq!(
            (
                paragraph.rect.x,
                paragraph.rect.y,
                paragraph.rect.width,
                paragraph.rect.height
            ),
            (8, 24, 784, 20)
        );
        assert_eq!(find(&root, "body").rect.height, 52);
        assert_eq!(find(&root, "html").rect.height, 68);
    }

    /// §6.3: margins do not collapse, so two paragraphs are 32px apart (T26).
    #[test]
    fn margins_do_not_collapse() {
        let root = boxes("<p>a</p><p>b</p>");
        let paragraphs: Vec<&LayoutBox> = root
            .children
            .iter()
            .flat_map(|child| child.children.iter())
            .filter(|child| child.tag == "p")
            .collect();
        assert_eq!(paragraphs.len(), 2, "{root:#?}");
        assert_eq!(paragraphs[0].rect.y, 24);
        assert_eq!(paragraphs[1].rect.y, 76);
    }

    /// §6.2: padding narrows the content box rather than growing the box (T20).
    #[test]
    fn padding_narrows_the_content_box() {
        let root = boxes("<style>div{padding:10px}</style><div><p>x</p></div>");
        let div = find(&root, "div");
        assert_eq!((div.rect.x, div.rect.width), (8, 784));
        assert_eq!(find(&root, "p").rect.x, 18);
        assert_eq!(find(&root, "p").rect.width, 764);
    }

    /// §6.4: an explicit height is not shrunk by content, and children overflow visibly (T24).
    #[test]
    fn explicit_height_survives_overflowing_children() {
        let root = boxes("<style>div{height:10px}</style><div><p>a</p></div>");
        let div = find(&root, "div");
        assert_eq!((div.rect.y, div.rect.height), (8, 10));
        assert_eq!(find(&root, "p").rect.y, 24);
    }

    /// §6.4/§6.2a: an auto height is a *border-box* height, so padding and border are added to
    /// the content height rather than eating into it.
    #[test]
    fn auto_height_includes_padding_and_border() {
        let root = boxes("<style>div{padding:8px;border:1px solid}</style><div><p>x</p></div>");
        let div = find(&root, "div");
        // Content: p's 16px top margin + 20px line + 16px bottom margin = 52.
        assert_eq!(div.rect.height, 52 + 16 + 2, "{root:#?}");
    }
}
