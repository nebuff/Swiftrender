//! The parsed document model (`SPEC.md` §2.7): nodes, children and queries.
//!
//! Kept apart from [`super::tree`], which *constructs* the tree. Every later layer (CSS, layout,
//! paint) reads the model; construction happens once, at startup. The split keeps both files
//! under the §10.3 budget as the model grows, and it makes the dependency direction obvious:
//! `tree` depends on `dom`, never the reverse.

use super::tokenizer::Attr;

/// A node in the parsed document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Lowercased tag name.
    pub name: String,
    /// Attributes in source order (duplicates are kept; [`Node::attr`] returns the first).
    pub attrs: Vec<Attr>,
    /// Child elements and text, in document order.
    pub children: Vec<Child>,
}

/// A child of an element: either a nested element or a text run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Child {
    /// A nested element.
    Element(Node),
    /// A text run, with whitespace preserved exactly as authored. Collapsing happens in
    /// layout (`SPEC.md` §6.5), not here.
    Text(String),
}

impl Node {
    /// Create an element with no attributes or children.
    pub fn new(name: &str) -> Node {
        Node {
            name: name.to_string(),
            attrs: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Value of `name`, or `None` for a missing or boolean attribute.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|attr| attr.name == name)
            .and_then(|attr| attr.value.as_deref())
    }

    /// The `class` attribute split into individual class names (`SPEC.md` §3.2).
    pub fn classes(&self) -> Vec<&str> {
        self.attr("class")
            .map(|value| value.split_whitespace().collect())
            .unwrap_or_default()
    }

    /// First child element named `name`.
    pub fn child_element(&self, name: &str) -> Option<&Node> {
        self.children.iter().find_map(|child| match child {
            Child::Element(node) if node.name == name => Some(node),
            _ => None,
        })
    }

    /// All descendant elements named `name`, in document order (pre-order, excluding self).
    pub fn descendants(&self, name: &str) -> Vec<&Node> {
        let mut out = Vec::new();
        collect_named(self, name, &mut out);
        out
    }

    /// Concatenated descendant text, used for `<title>` and `<style>` content.
    pub fn text(&self) -> String {
        let mut out = String::new();
        collect_text(self, &mut out);
        out
    }

    /// Depth of the deepest element in this subtree, counting this node as depth 1.
    pub fn max_depth(&self) -> usize {
        self.children
            .iter()
            .map(|child| match child {
                Child::Element(node) => node.max_depth(),
                Child::Text(_) => 0,
            })
            .max()
            .unwrap_or(0)
            + 1
    }

    /// Whether this subtree contains any text at all.
    pub fn has_text(&self) -> bool {
        self.children.iter().any(|child| match child {
            Child::Element(node) => node.has_text(),
            Child::Text(text) => !text.is_empty(),
        })
    }
}

/// Recursive helper for [`Node::descendants`].
fn collect_named<'a>(node: &'a Node, name: &str, out: &mut Vec<&'a Node>) {
    for child in &node.children {
        if let Child::Element(inner) = child {
            if inner.name == name {
                out.push(inner);
            }
            collect_named(inner, name, out);
        }
    }
}

/// Recursive helper for [`Node::text`].
fn collect_text(node: &Node, out: &mut String) {
    for child in &node.children {
        match child {
            Child::Element(inner) => collect_text(inner, out),
            Child::Text(text) => out.push_str(text),
        }
    }
}

/// A parsed document: always rooted at a synthesized-or-authored `html` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The document root, always named `html` (`SPEC.md` §2.7).
    pub root: Node,
    /// Text of the first `<title>`, if any; used by the `--emit tree` header (§2.8).
    pub title: Option<String>,
    /// Text of every `<style>` element, in document order (§2.9).
    pub styles: Vec<String>,
    /// Diagnostics for `--verbose`: dropped elements and other recoveries (§9.3).
    pub warnings: Vec<String>,
}

impl Document {
    /// The `body` element, guaranteed present after [`parse`].
    pub fn body(&self) -> &Node {
        self.root
            .child_element("body")
            .expect("parse always synthesizes a body")
    }

    /// The `head` element, guaranteed present after [`parse`].
    pub fn head(&self) -> &Node {
        self.root
            .child_element("head")
            .expect("parse always synthesizes a head")
    }
}
