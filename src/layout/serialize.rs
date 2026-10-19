//! The `--emit tree` box dump (`SPEC.md` §8.1).
//!
//! One line per box, two spaces of indent per depth, then the geometry and the box model. The
//! format is a contract in two directions: `TESTS.md` asserts on it through
//! `tests/common/mod.rs::boxes`, and humans read it to see what the engine thinks it is doing.
//! Changing it means changing both readers in the same commit.
//!
//! Beyond the fields §8.1 mandates (`x/y/w/h`, margins, padding, borders), each line also carries
//! the computed `font` size and `color`, and a text run prints its text in quotes as its label.
//! Both were needed by the numbered tests (T9, T11, T15, T27, T40) and are noted in `SPEC.md`
//! §8.1 as Phase 2 additions.

use super::tree::{BoxTree, LayoutBox};

/// Render `tree` as the box-tree dump.
///
/// # Examples
///
/// ```
/// use swiftrender::css::Css;
/// use swiftrender::html::parse;
/// use swiftrender::layout::{dump_tree, layout_document};
///
/// let document = parse("<p>hi</p>");
/// let mut warnings = Vec::new();
/// let css = Css::from_document(&document, &mut warnings);
/// let tree = layout_document(&document, &css, 800, 600, &mut warnings);
/// let text = dump_tree(&tree);
/// assert!(text.starts_with("swiftrender (untitled) 800x600"));
/// assert!(text.contains("box tree"));
/// ```
pub fn dump_tree(tree: &BoxTree) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "swiftrender {} {}x{}\n",
        tree.title.as_deref().unwrap_or("(untitled)"),
        tree.width,
        tree.height
    ));
    out.push_str("box tree\n");
    write_box(&mut out, &tree.root, 0);
    out
}

/// Append one box and its subtree.
fn write_box(out: &mut String, node: &LayoutBox, depth: usize) {
    let rect = node.rect;
    let style = &node.style;
    let border = style.effective_border();
    out.push_str(&"  ".repeat(depth));
    out.push_str(&node.label());
    out.push_str(&format!(
        "  x={} y={} w={} h={}  margin={} padding={} border={}  font={} color={}\n",
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        edges(style.margin),
        edges(style.padding),
        edges(border),
        style.font_size,
        style.color.to_hex(),
    ));
    for child in &node.children {
        write_box(out, child, depth + 1);
    }
}

/// Format four edge values as `t,r,b,l`, which is the order `BoxInfo` reports.
fn edges(values: crate::css::Edges<f64>) -> String {
    format!(
        "{},{},{},{}",
        values.top.round() as i32,
        values.right.round() as i32,
        values.bottom.round() as i32,
        values.left.round() as i32
    )
}
