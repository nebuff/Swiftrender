//! Box generation (`SPEC.md` §5).
//!
//! Walks the DOM once, computing each element's style on the way down — the cascade is
//! element-local plus inheritance (`SPEC.md` §3.7), so no separate style tree is needed.
//!
//! Three rules from §5 are enforced here rather than during layout:
//!
//! * `display: none` produces no box and no descendants (§5.1),
//! * a block container's children are either all block or all inline, with runs of inline
//!   children wrapped in anonymous blocks (§5.3),
//! * `<img>` becomes an atomic box and `<br>` a break (§5.5, §5.6).
//!
//! One M1 simplification, documented rather than hidden: an inline element that contains a
//! block-level child is itself treated as a block, so the "all block or all inline" invariant
//! cannot be broken by markup such as `<span><div>x</div></span>`.

use super::tree::{Kind, LayoutBox};
use crate::css::{ComputedStyle, Css, Display};
use crate::html::{Child as DomChild, Document, Node};

/// Build the box tree for a whole document.
pub fn build(document: &Document, css: &Css, warnings: &mut Vec<String>) -> LayoutBox {
    build_element(&document.root, None, css, warnings)
        .expect("the html element never has display: none")
}

/// Build the box for one element, or `None` when it is suppressed (`SPEC.md` §5.1).
fn build_element(
    element: &Node,
    parent: Option<&ComputedStyle>,
    css: &Css,
    warnings: &mut Vec<String>,
) -> Option<LayoutBox> {
    let style = css.style_for(element, parent, warnings);
    if style.display == Display::None {
        return None;
    }

    let kind = match style.display {
        Display::Block => Kind::Block,
        Display::Inline => inline_kind(&element.name),
        Display::None => unreachable!("handled above"),
    };
    let mut node = LayoutBox::new(kind, &element.name, style.clone());
    node.classes = element
        .classes()
        .iter()
        .map(|name| name.to_string())
        .collect();
    node.children = build_children(element, &style, css, warnings);

    // `<span><div>x</div></span>`: promote rather than break the invariant in §5.3.
    if node.kind == Kind::Inline && node.children.iter().any(LayoutBox::is_block_level) {
        node.kind = Kind::Block;
    }
    if node.is_block_level() {
        node.children = wrap_anonymous(node.children, &style);
    }
    Some(node)
}

/// Which kind of inline box an element generates.
fn inline_kind(name: &str) -> Kind {
    match name {
        "img" => Kind::Atomic,
        "br" => Kind::Break,
        _ => Kind::Inline,
    }
}

/// Build all children of `element`, skipping suppressed ones.
fn build_children(
    element: &Node,
    style: &ComputedStyle,
    css: &Css,
    warnings: &mut Vec<String>,
) -> Vec<LayoutBox> {
    let mut out = Vec::new();
    for child in &element.children {
        match child {
            DomChild::Text(text) => {
                // Raw text is kept here; whitespace collapsing happens during inline layout,
                // which is the only place that knows where lines begin and end (§6.5).
                if !text.is_empty() {
                    let mut text_box = LayoutBox::new(Kind::Text, "#text", style.clone());
                    text_box.text = Some(text.clone());
                    out.push(text_box);
                }
            }
            DomChild::Element(node) => {
                if let Some(built) = build_element(node, Some(style), css, warnings) {
                    out.push(built);
                }
            }
        }
    }
    out
}

/// Wrap runs of inline-level children in anonymous blocks when block siblings are present
/// (`SPEC.md` §5.3). A container whose children are all inline needs no wrapper: it *is* the
/// inline formatting context.
fn wrap_anonymous(children: Vec<LayoutBox>, parent_style: &ComputedStyle) -> Vec<LayoutBox> {
    if !children.iter().any(LayoutBox::is_inline_level)
        || !children.iter().any(LayoutBox::is_block_level)
    {
        return children;
    }

    let mut out: Vec<LayoutBox> = Vec::new();
    let mut run: Vec<LayoutBox> = Vec::new();
    for child in children {
        if child.is_inline_level() {
            run.push(child);
        } else {
            if !run.is_empty() {
                out.push(anonymous_block(std::mem::take(&mut run), parent_style));
            }
            out.push(child);
        }
    }
    if !run.is_empty() {
        out.push(anonymous_block(run, parent_style));
    }
    out
}

/// Build one anonymous block around `children`.
fn anonymous_block(children: Vec<LayoutBox>, parent_style: &ComputedStyle) -> LayoutBox {
    let mut style = ComputedStyle::inherited_from(Some(parent_style));
    style.display = Display::Block;
    let mut node = LayoutBox::new(Kind::AnonBlock, "#anon", style);
    node.children = children;
    node
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::parse as parse_html;

    /// Build a tree, asserting that no warning was produced by a well-formed fixture.
    fn tree(html: &str) -> LayoutBox {
        let document = parse_html(html);
        let mut warnings = Vec::new();
        let css = Css::from_document(&document, &mut warnings);
        let tree = build(&document, &css, &mut warnings);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
        tree
    }

    /// Find the first box with `tag` in pre-order.
    fn find<'a>(node: &'a LayoutBox, tag: &str) -> Option<&'a LayoutBox> {
        if node.tag == tag {
            return Some(node);
        }
        node.children.iter().find_map(|child| find(child, tag))
    }

    /// §5.3: three children in, three children out, with two anonymous blocks.
    #[test]
    fn mixed_content_gains_anonymous_blocks() {
        let root = tree("<div>text<p>para</p>tail</div>");
        let div = find(&root, "div").expect("div box");
        let kinds: Vec<Kind> = div.children.iter().map(|child| child.kind).collect();
        assert_eq!(
            kinds,
            vec![Kind::AnonBlock, Kind::Block, Kind::AnonBlock],
            "{:#?}",
            div.children
        );
    }

    /// §5.3: all-inline content needs no wrapper — that is the whole point of the "or all
    /// inline" half of the rule.
    #[test]
    fn all_inline_content_gains_no_wrapper() {
        let root = tree("<div><span>x</span></div>");
        let div = find(&root, "div").expect("div box");
        assert_eq!(div.children.len(), 1);
        assert_eq!(div.children[0].kind, Kind::Inline);
    }

    /// §5.1: `display: none` removes the element and its whole subtree.
    #[test]
    fn display_none_removes_the_subtree() {
        let root = tree("<style>.g{display:none}</style><div class=\"g\">gone</div><p>z</p>");
        assert!(find(&root, "div").is_none());
        assert!(find(&root, "p").is_some());
    }

    /// §2.8: head content is styled away by the UA sheet, so it generates no boxes.
    #[test]
    fn head_content_generates_no_boxes() {
        let root =
            tree("<head><title>t</title><style>p{color:#f00}</style></head><body><p>x</p></body>");
        assert!(find(&root, "title").is_none());
        assert!(find(&root, "style").is_none());
        assert!(find(&root, "p").is_some());
    }

    /// §2.11: an unknown element is inline.
    #[test]
    fn unknown_elements_are_inline() {
        let root = tree("<foo>x</foo>");
        let body = find(&root, "body").expect("body box");
        assert_eq!(body.children.len(), 1);
        assert_eq!(body.children[0].kind, Kind::Inline);
    }
}
