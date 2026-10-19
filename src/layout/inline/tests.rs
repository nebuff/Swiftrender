//! Unit tests for inline formatting.
//!
//! Kept in their own file so `inline.rs` stays inside the 400-line budget
//! (`SPEC.md` §10.1) without giving up access to private items.

use super::*;
use crate::css::{ComputedStyle, Css};
use crate::html::parse as parse_html;
use crate::layout::tree::LayoutBox;

/// Build a laid-out `<p>` for the body of a document with `html`.
fn paragraph(html: &str) -> LayoutBox {
    let document = parse_html(html);
    let mut warnings = Vec::new();
    let css = Css::from_document(&document, &mut warnings);
    let mut tree = super::super::boxgen::build(&document, &css, &mut warnings);
    let body = tree
        .children
        .iter_mut()
        .find(|child| child.tag == "body")
        .expect("body box");
    let paragraph = body
        .children
        .iter_mut()
        .find(|child| child.tag == "p")
        .expect("p box");
    let style = paragraph.style.clone();
    let height = layout_inline(paragraph, 8.0, 784.0, 24.0);
    assert!(style.font_size > 0);
    paragraph.rect.height = height as i32;
    paragraph.clone()
}

/// §6.5: whitespace collapses and is trimmed, so `"  a   b  "` becomes `"a b"`.
#[test]
fn whitespace_collapses_and_trims() {
    let p = paragraph("<p>  a   b  </p>");
    assert_eq!(p.children.len(), 1, "{:#?}", p.children);
    let text = &p.children[0];
    assert_eq!(text.text.as_deref(), Some("a b"));
    assert_eq!(text.rect.width, 24);
    assert_eq!(text.rect.x, 8);
}

/// §6.5: a line breaks before the word that would overflow, each line is its own fragment, and
/// a line that is *exactly* full does not break early (T31).
///
/// Thirty-three `ab ` words measure `33 × 16 + 32 × 8 = 784`, exactly the content width, so the
/// fixture pins the boundary rather than just the common case.
#[test]
fn lines_break_greedily_and_split_the_run() {
    let p = paragraph(&format!("<p>{}</p>", "ab ".repeat(60)));
    let fragments: Vec<&LayoutBox> = p.children.iter().filter(|c| c.kind == Kind::Text).collect();
    assert_eq!(fragments.len(), 2, "{:#?}", p.children);
    assert_eq!(fragments[0].rect.width, 784);
    assert_eq!(fragments[1].rect.width, 640);
    assert_eq!(p.rect.height, 40);
}

/// §6.5: an unbreakable word overflows instead of being split (T32).
#[test]
fn an_unbreakable_word_overflows() {
    let p = paragraph(&format!("<p>{}</p>", "a".repeat(200)));
    let fragments: Vec<&LayoutBox> = p.children.iter().filter(|c| c.kind == Kind::Text).collect();
    assert_eq!(fragments.len(), 1);
    assert_eq!(fragments[0].rect.width, 1600);
}

/// §5.5: a forced break produces an empty line, which still has height (T36).
#[test]
fn forced_breaks_make_empty_lines_with_height() {
    let p = paragraph("<p>a<br><br>b</p>");
    assert_eq!(p.rect.height, 60, "{:#?}", p.children);
}

/// §6.5: centring shifts the line by half the leftover space, rounded once (T11).
#[test]
fn centring_rounds_once() {
    let mut style = ComputedStyle::initial();
    style.text_align = TextAlign::Center;
    assert_eq!(align_offset(style.text_align, 784.0, 8.0), 388.0);
    assert_eq!(align_offset(TextAlign::Right, 784.0, 24.0), 760.0);
    assert_eq!(align_offset(TextAlign::Left, 784.0, 24.0), 0.0);
}
