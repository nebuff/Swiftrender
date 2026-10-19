//! Human-readable tree dump for `--emit tree` (`SPEC.md` §8.1).
//!
//! Phase 1 prints the **DOM** produced by [`super::parse`]. From Phase 2 the same command
//! prints the **box tree** with geometry, which is why the header names the artifact
//! explicitly. The dump is fully deterministic: same input, same bytes (`SPEC.md` §8.5).

use super::tree::{Child, Document, Node};

/// Render `doc` as text: one header line, then one line per node.
///
/// `width` and `height` are the configured canvas size; they do not affect the dump yet but
/// keep the header identical to the box-tree version that Phase 2 will emit.
pub fn dump(doc: &Document, width: u32, height: u32) -> String {
    let title = doc.title.as_deref().unwrap_or("(untitled)");
    let mut out = String::new();
    out.push_str(&format!("swiftrender {title} {width}x{height}\n"));
    out.push_str("dom tree (phase 1; box tree arrives with layout)\n");
    write_node(&mut out, &doc.root, 0);
    out
}

/// Append one node and its subtree, indented two spaces per level.
fn write_node(out: &mut String, node: &Node, depth: usize) {
    out.push_str(&"  ".repeat(depth));
    out.push_str(&node.name);
    for class in node.classes() {
        out.push('.');
        out.push_str(class);
    }
    out.push('\n');
    for child in &node.children {
        match child {
            Child::Element(inner) => write_node(out, inner, depth + 1),
            Child::Text(text) => {
                out.push_str(&"  ".repeat(depth + 1));
                out.push_str(&format!("\"{}\"\n", escape(text)));
            }
        }
    }
}

/// Escape control characters so a text run always occupies exactly one line.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '"' => out.push_str("\\\""),
            _ => out.push(ch),
        }
    }
    out
}
