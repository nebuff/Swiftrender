//! Structural SVG output (`SPEC.md` §8.4).
//!
//! SVG is a **structural dump**, not a second rasteriser: it records the boxes the engine built
//! as rectangles and text runs so a human can inspect layout without decoding pixels. It is
//! therefore not required to be pixel-identical to the PPM/PNG output, and the tests assert
//! structure and escaping rather than appearance.

use crate::css::BorderStyle;
use crate::layout::{BoxTree, Kind, LayoutBox};

/// Render `tree` as an SVG document.
pub fn encode(tree: &BoxTree) -> String {
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\">\n",
        tree.width, tree.height
    );
    write_box(&mut out, &tree.root, 1);
    out.push_str("</svg>\n");
    out
}

/// Append one box and its subtree.
fn write_box(out: &mut String, node: &LayoutBox, depth: usize) {
    let indent = "  ".repeat(depth);
    let rect = node.rect;

    if let Some(color) = node.style.background {
        out.push_str(&format!(
            "{indent}<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>\n",
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            color.to_hex()
        ));
    }

    write_borders(out, node, &indent);

    if node.kind == Kind::Atomic && node.style.background.is_none() {
        let border = node.style.effective_border();
        out.push_str(&format!(
            "{indent}<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#c0c0c0\"/>\n",
            rect.x + border.left.round() as i32 + node.style.padding.left.round() as i32,
            rect.y + border.top.round() as i32 + node.style.padding.top.round() as i32,
            (rect.width
                - (border.left + border.right + node.style.padding.left + node.style.padding.right)
                    .round() as i32)
                .max(0),
            (rect.height
                - (border.top + border.bottom + node.style.padding.top + node.style.padding.bottom)
                    .round() as i32)
                .max(0),
        ));
    }

    if node.kind == Kind::Text {
        if let Some(text) = &node.text {
            out.push_str(&format!(
                "{indent}<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>\n",
                rect.x,
                rect.y + node.style.font_size,
                node.style.font_size,
                node.style.color.to_hex(),
                escape(text)
            ));
        }
    }

    for child in &node.children {
        write_box(out, child, depth + 1);
    }
}

/// Emit solid borders as four rectangles (`SPEC.md` §7.4).
fn write_borders(out: &mut String, node: &LayoutBox, indent: &str) {
    if node.style.border_style != BorderStyle::Solid {
        return;
    }
    let border = node.style.effective_border();
    let color = node.style.border_color.unwrap_or(node.style.color).to_hex();
    let rect = node.rect;
    let top = border.top.round() as i32;
    let right = border.right.round() as i32;
    let bottom = border.bottom.round() as i32;
    let left = border.left.round() as i32;
    let middle_height = rect.height - top - bottom;

    for (x, y, width, height) in [
        (rect.x, rect.y, rect.width, top),
        (rect.x, rect.y + rect.height - bottom, rect.width, bottom),
        (rect.x, rect.y + top, left, middle_height),
        (
            rect.x + rect.width - right,
            rect.y + top,
            right,
            middle_height,
        ),
    ] {
        if width > 0 && height > 0 {
            out.push_str(&format!(
                "{indent}<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" fill=\"{color}\"/>\n"
            ));
        }
    }
}

/// XML-escape text content so a document is well formed.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::escape;

    /// Escaping covers all five XML metacharacters, not only the two that usually matter.
    #[test]
    fn escaping_is_complete() {
        assert_eq!(escape("a&b<c>d\"e'f"), "a&amp;b&lt;c&gt;d&quot;e&apos;f");
    }
}
