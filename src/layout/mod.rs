//! Box generation and layout (`SPEC.md` §5–§6).
//!
//! Pipeline for one document:
//!
//! 1. [`boxgen::build`] walks the DOM, computes a style per element and produces a box tree with
//!    anonymous blocks already in place (§5.3).
//! 2. [`block::layout`] assigns geometry, recursing into [`inline::layout_inline`] wherever a block
//!    container holds inline content (§6.2–§6.5).
//! 3. [`serialize::dump_tree`] renders the result for `--emit tree`.
//!
//! Geometry is computed in `f64` and snapped to `i32` pixels **once**, when a rect is written
//! (`SPEC.md` §6.7). Nothing downstream rounds again, which is what lets every numbered test assert
//! exact integers.

pub mod block;
pub mod boxgen;
pub mod font;
pub mod inline;
pub mod serialize;
pub mod tree;

pub use serialize::dump_tree;
pub use tree::{BoxTree, Kind, LayoutBox};

use crate::css::Css;
use crate::html::Document;

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

/// Build and lay out a document, returning the finished box tree.
///
/// `width` and `height` are the canvas size: they define the containing block for the root box,
/// which is why the dump can print them and the tests can predict every coordinate.
pub fn layout_document(
    document: &Document,
    css: &Css,
    width: u32,
    height: u32,
    warnings: &mut Vec<String>,
) -> BoxTree {
    let mut root = boxgen::build(document, css, warnings);
    block::layout(&mut root, 0.0, 0.0, f64::from(width));
    BoxTree {
        root,
        width,
        height,
        title: document.title.clone(),
    }
}
