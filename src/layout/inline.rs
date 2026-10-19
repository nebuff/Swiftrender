//! Inline formatting (`SPEC.md` §6.5).
//!
//! The algorithm is greedy and narrow by design: flatten inline content into a stream of *cells*
//! (words, single spaces, atomic boxes, forced breaks), break that stream into lines, then rebuild
//! the box tree from the result.
//!
//! Two documented M1 simplifications, both of which preserve painted output for the supported
//! subset but are visible in the tree dump:
//!
//! * **Inline descendants are hoisted.** Text inside `<span>` becomes a sibling fragment of that
//!   span rather than a child of it, because M1 paints inline backgrounds as a single fragment
//!   rectangle and never needs the nesting. `<span class=s>ab</span>` therefore dumps as
//!   `span.s` followed by `"ab"`, both positioned identically.
//! * **A text run is split per line.** A text box that wraps becomes one box per line fragment,
//!   which is what makes `TESTS.md` T31 see two runs of 784px and 16px.
//!
//! Whitespace collapsing happens here rather than during box generation because only this layer
//! knows where lines begin and end.

use super::font::metrics;
use super::tree::{Kind, LayoutBox};
use crate::css::{ComputedStyle, TextAlign};

mod place;

use place::{rebuild, Placed};

/// What a cell does to the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellKind {
    /// An unbreakable run of non-whitespace characters.
    Word,
    /// One collapsible space.
    Space,
    /// An atomic inline box (`<img>`), which never breaks internally.
    Atom,
    /// A forced break (`<br>`).
    Break,
}

/// One flattened piece of inline content.
struct Cell {
    /// Index of the text box or atom this cell came from, so fragments can be regrouped.
    source: usize,
    /// The nearest inline element that should get a rect of its own.
    owner: Option<usize>,
    /// What the cell is.
    kind: CellKind,
    /// Text for word and space cells, empty otherwise.
    text: String,
    /// Horizontal extent in pixels.
    width: f64,
    /// Contribution to the line box height (`SPEC.md` §6.5: max over items).
    contribution: f64,
    /// Font size, for placing and sizing a text fragment.
    font_size: i32,
    /// Line height, used when a forced break creates an empty line.
    line_height: f64,
    /// Resolved line-height for the cell's own style, used for empty lines.
    style: ComputedStyle,
    /// For atoms: element name, outer width and outer height.
    atom: Option<(String, f64, f64)>,
}

/// An inline element that receives a fragment rectangle.
struct Owner {
    /// Element name.
    tag: String,
    /// Class names, for the dump.
    classes: Vec<String>,
    /// Computed style.
    style: ComputedStyle,
}

/// One line under construction.
struct Line {
    /// Placed cells and their offsets relative to the content box.
    cells: Vec<(usize, f64)>,
    /// Width consumed, excluding trailing space.
    used: f64,
    /// Tallest item on the line.
    height: f64,
}

impl Line {
    /// An empty line.
    fn new() -> Line {
        Line {
            cells: Vec::new(),
            used: 0.0,
            height: 0.0,
        }
    }

    /// Whether anything has been placed yet.
    fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Finish the line, giving an empty line the height of its break (`SPEC.md` §5.5).
    fn finish(self, default_height: f64) -> Line {
        Line {
            cells: self.cells,
            used: self.used,
            height: if self.height > 0.0 {
                self.height
            } else {
                default_height
            },
        }
    }
}

/// Lay out the inline content of `container` and fill in its children's geometry.
///
/// Returns the total content height contributed by the line boxes, which is what an auto-height
/// block container uses (`SPEC.md` §6.4).
pub fn layout_inline(
    container: &mut LayoutBox,
    content_x: f64,
    content_width: f64,
    content_top: f64,
) -> f64 {
    let mut owners: Vec<Owner> = Vec::new();
    let mut cells: Vec<Cell> = Vec::new();
    let mut source = 0;
    flatten(
        &container.children,
        &mut owners,
        &mut cells,
        None,
        &mut source,
    );

    if cells.is_empty() {
        container.children.clear();
        return 0.0;
    }

    let default_line_height = metrics(&container.style).line_height;
    let lines = break_lines(&cells, content_width, default_line_height);

    let mut placed: Vec<Placed> = Vec::new();
    let mut y = content_top;
    for (line_index, line) in lines.iter().enumerate() {
        let offset = align_offset(container.style.text_align, content_width, line.used);
        for (cell, x) in &line.cells {
            placed.push(Placed {
                cell: *cell,
                x: content_x + offset + x,
                line: line_index,
                line_top: y,
                line_height: line.height,
            });
        }
        y += line.height;
    }
    let content_height = y - content_top;

    rebuild(container, &cells, &owners, &placed);
    content_height
}

/// Flatten a subtree of inline content into cells, registering inline owners on the way.
fn flatten(
    children: &[LayoutBox],
    owners: &mut Vec<Owner>,
    cells: &mut Vec<Cell>,
    current_owner: Option<usize>,
    source: &mut usize,
) {
    for child in children {
        match child.kind {
            Kind::Text => {
                let id = *source;
                *source += 1;
                push_text_cells(
                    child.text.as_deref().unwrap_or(""),
                    &child.style,
                    current_owner,
                    id,
                    cells,
                );
            }
            Kind::Atomic => {
                let id = *source;
                *source += 1;
                let style = &child.style;
                let width = style.width.unwrap_or(150.0);
                let height = style.height.unwrap_or(50.0);
                let metrics = metrics(style);
                cells.push(Cell {
                    source: id,
                    owner: current_owner,
                    kind: CellKind::Atom,
                    text: String::new(),
                    width,
                    contribution: height.max(metrics.line_height),
                    font_size: style.font_size,
                    line_height: metrics.line_height,
                    style: style.clone(),
                    atom: Some((child.tag.clone(), width, height)),
                });
            }
            Kind::Break => {
                let metrics = metrics(&child.style);
                cells.push(Cell {
                    source: usize::MAX,
                    owner: current_owner,
                    kind: CellKind::Break,
                    text: String::new(),
                    width: 0.0,
                    contribution: metrics.line_height,
                    font_size: child.style.font_size,
                    line_height: metrics.line_height,
                    style: child.style.clone(),
                    atom: None,
                });
            }
            Kind::Inline => {
                let index = owners.len();
                owners.push(Owner {
                    tag: child.tag.clone(),
                    classes: child.classes.clone(),
                    style: child.style.clone(),
                });
                flatten(&child.children, owners, cells, Some(index), source);
            }
            _ => {}
        }
    }
}

/// Turn one text run into word and space cells, collapsing whitespace runs (`SPEC.md` §6.5).
fn push_text_cells(
    raw: &str,
    style: &ComputedStyle,
    owner: Option<usize>,
    source: usize,
    cells: &mut Vec<Cell>,
) {
    let metrics = metrics(style);
    let mut word = String::new();

    let flush_word = |word: &mut String, cells: &mut Vec<Cell>| {
        if word.is_empty() {
            return;
        }
        cells.push(Cell {
            source,
            owner,
            kind: CellKind::Word,
            text: std::mem::take(word),
            width: 0.0, // measured below, once the text is known
            contribution: metrics.line_height,
            font_size: style.font_size,
            line_height: metrics.line_height,
            style: style.clone(),
            atom: None,
        });
    };

    for ch in raw.chars() {
        if ch.is_whitespace() {
            flush_word(&mut word, cells);
            cells.push(Cell {
                source,
                owner,
                kind: CellKind::Space,
                text: " ".to_string(),
                width: metrics.advance,
                contribution: metrics.line_height,
                font_size: style.font_size,
                line_height: metrics.line_height,
                style: style.clone(),
                atom: None,
            });
        } else {
            word.push(ch);
        }
    }
    flush_word(&mut word, cells);

    // Measure every word cell we just pushed from this run.
    for cell in cells.iter_mut().rev() {
        if cell.source != source {
            break;
        }
        if cell.kind == CellKind::Word {
            cell.width = super::font::text_width(cell.text.chars().count(), metrics);
        }
    }
}

/// Break cells into lines greedily (`SPEC.md` §6.5).
fn break_lines(cells: &[Cell], content_width: f64, default_line_height: f64) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let mut line = Line::new();
    let mut x = 0.0;
    let mut pending_space: Option<usize> = None;
    let mut ended_with_break = false;

    for (index, cell) in cells.iter().enumerate() {
        match cell.kind {
            CellKind::Break => {
                lines.push(line.finish(cell.line_height));
                line = Line::new();
                x = 0.0;
                pending_space = None;
                ended_with_break = true;
            }
            CellKind::Space => {
                // Only one space survives a run, and only if a word follows on the same line.
                pending_space = Some(index);
            }
            CellKind::Word | CellKind::Atom => {
                let space_width = match (pending_space, line.is_empty()) {
                    (Some(space), false) => cells[space].width,
                    _ => 0.0,
                };
                if !line.is_empty() && line.used + space_width + cell.width > content_width {
                    lines.push(line.finish(default_line_height));
                    line = Line::new();
                    x = 0.0;
                    pending_space = None;
                }
                if let Some(space) = pending_space {
                    if !line.is_empty() {
                        line.cells.push((space, x));
                        x += cells[space].width;
                        line.used += cells[space].width;
                    }
                    pending_space = None;
                }
                line.cells.push((index, x));
                x += cell.width;
                line.used += cell.width;
                line.height = line.height.max(cell.contribution);
                ended_with_break = false;
            }
        }
    }

    if !line.is_empty() || ended_with_break {
        lines.push(line.finish(default_line_height));
    }
    lines
}

/// The horizontal shift applied to a whole line by `text-align` (`SPEC.md` §6.5).
fn align_offset(align: TextAlign, content_width: f64, used: f64) -> f64 {
    match align {
        TextAlign::Left => 0.0,
        TextAlign::Center => ((content_width - used) / 2.0).round(),
        TextAlign::Right => (content_width - used).max(0.0),
    }
}

#[cfg(test)]
mod tests;
