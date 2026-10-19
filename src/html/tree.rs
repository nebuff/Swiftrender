//! Tree construction (`SPEC.md` §2.6–§2.11).
//!
//! The builder is a single stack machine. It implements the *documented* recovery rules of
//! `SPEC.md` §2.6 and nothing more: there are no insertion modes, no foster parenting and no
//! tables. Two consequences are deliberate and visible in `--emit tree`:
//!
//! * Element nesting depth is capped (see [`crate::MAX_DEPTH`]); deeper elements are dropped
//!   with a warning instead of overflowing the stack.
//! * `<html>` and `<body>` are synthesized when missing, so the tree root is always `html`.
//!
//! The model these functions build lives in [`super::dom`]; this file only ever constructs it.

use super::dom::{Child, Document, Node};
use super::tokenizer::{tokenize, Attr, Token};
use crate::MAX_DEPTH;

/// Elements that never have children or an end tag (`SPEC.md` §2.5).
pub const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Element depth cap during construction.
///
/// `MAX_DEPTH - 2` reserves the two levels that [`wrap`] always synthesizes (`html` and
/// `body`), so the invariant tested by `TESTS.md` T8 — final tree depth ≤ [`crate::MAX_DEPTH`]
/// — holds for the whole document, not just the author's markup.
const BUILD_DEPTH_CAP: usize = MAX_DEPTH - 2;

/// Parse `input` into a [`Document`].
///
/// # Examples
///
/// ```
/// use swiftrender::html::parse;
/// let doc = parse("<p>hi</p>");
/// assert_eq!(doc.body().children.len(), 1);
/// assert!(doc.title.is_none());
/// ```
pub fn parse(input: &str) -> Document {
    let mut builder = Builder::default();
    for token in tokenize(input) {
        builder.push(token);
    }
    let (children, warnings) = builder.finish();
    let root = wrap(children);

    let title = root
        .descendants("title")
        .first()
        .map(|node| node.text())
        .filter(|text| !text.is_empty());
    let styles = root
        .descendants("style")
        .iter()
        .map(|node| node.text())
        .collect();

    Document {
        root,
        title,
        styles,
        warnings,
    }
}

/// Stack machine that turns a flat token stream into a forest.
#[derive(Default)]
struct Builder {
    /// Currently open elements, outermost first.
    stack: Vec<Node>,
    /// Names dropped because of the depth cap, so their end tags are ignored too.
    dropped: Vec<String>,
    /// Completed top-level children.
    roots: Vec<Child>,
    /// Collected diagnostics.
    warnings: Vec<String>,
    /// Whether the depth warning has already been emitted (once per document).
    warned_depth: bool,
}

impl Builder {
    /// Feed one token to the builder.
    fn push(&mut self, token: Token) {
        match token {
            Token::StartTag {
                name,
                attrs,
                self_closing,
            } => self.open(name, attrs, self_closing),
            Token::EndTag { name } => self.close(&name),
            Token::Text(text) => self.append_text(&text),
        }
    }

    /// Open an element, or record it as void/self-closing/dropped.
    fn open(&mut self, name: String, attrs: Vec<Attr>, self_closing: bool) {
        if VOID_ELEMENTS.contains(&name.as_str()) || self_closing {
            let node = Node {
                name,
                attrs,
                children: Vec::new(),
            };
            self.append(Child::Element(node));
            return;
        }
        if self.stack.len() >= BUILD_DEPTH_CAP {
            if !self.warned_depth {
                self.warned_depth = true;
                self.warnings.push(format!(
                    "element nesting deeper than {MAX_DEPTH} was dropped"
                ));
            }
            self.dropped.push(name);
            return;
        }
        self.stack.push(Node {
            name,
            attrs,
            children: Vec::new(),
        });
    }

    /// Close the nearest open element named `name`, implicitly closing anything inside it.
    fn close(&mut self, name: &str) {
        if let Some(position) = self.stack.iter().rposition(|node| node.name == name) {
            while self.stack.len() > position {
                let node = self.stack.pop().expect("non-empty above position");
                self.append(Child::Element(node));
            }
        } else if self.dropped.last().map(String::as_str) == Some(name) {
            self.dropped.pop();
        }
        // An end tag with no open match is ignored (§2.6).
    }

    /// Append text to the innermost open element, merging with a preceding text run.
    fn append_text(&mut self, text: &str) {
        match self.stack.last_mut() {
            Some(node) => match node.children.last_mut() {
                Some(Child::Text(existing)) => existing.push_str(text),
                _ => node.children.push(Child::Text(text.to_string())),
            },
            None => self.append(Child::Text(text.to_string())),
        }
    }

    /// Append a finished child to the innermost open element, or to the root list.
    fn append(&mut self, child: Child) {
        match self.stack.last_mut() {
            Some(node) => node.children.push(child),
            None => self.roots.push(child),
        }
    }

    /// Close everything still open and hand back the top-level children and diagnostics.
    fn finish(&mut self) -> (Vec<Child>, Vec<String>) {
        self.close_remaining();
        (
            std::mem::take(&mut self.roots),
            std::mem::take(&mut self.warnings),
        )
    }

    /// Close all open elements at end of input (§2.6).
    fn close_remaining(&mut self) {
        while let Some(node) = self.stack.pop() {
            self.append(Child::Element(node));
        }
    }
}

/// Wrap the top-level children in `html` (and `body`), synthesizing whatever is missing.
fn wrap(children: Vec<Child>) -> Node {
    let mut html: Option<Node> = None;
    let mut strays: Vec<Child> = Vec::new();
    for child in children {
        match child {
            Child::Element(node) if node.name == "html" && html.is_none() => html = Some(node),
            other => strays.push(other),
        }
    }

    match html {
        Some(mut html) => {
            let mut inner = fit_body(std::mem::take(&mut html.children));
            // Strays that appeared outside <html> belong at the front of the body.
            strays.append(&mut inner.body.children);
            inner.body.children = strays;
            html.children = vec![Child::Element(inner.head), Child::Element(inner.body)];
            html
        }
        None => Node {
            name: "html".to_string(),
            attrs: Vec::new(),
            children: vec![
                Child::Element(Node::new("head")),
                Child::Element(Node {
                    name: "body".to_string(),
                    attrs: Vec::new(),
                    children: strays,
                }),
            ],
        },
    }
}

/// Head and body extracted from an `<html>` element's children.
struct Inner {
    /// The `head` element, synthesized when absent.
    head: Node,
    /// The `body` element, synthesized when absent; stray children are moved into it.
    body: Node,
}

/// Split an `html` element's children into `head` and `body`, preserving document order by
/// moving stray children into the body at the position where they appeared.
fn fit_body(children: Vec<Child>) -> Inner {
    let mut head: Option<Node> = None;
    let mut body: Option<Node> = None;
    let mut before: Vec<Child> = Vec::new();
    let mut after: Vec<Child> = Vec::new();

    for child in children {
        let name = match &child {
            Child::Element(node) => node.name.clone(),
            Child::Text(_) => String::new(),
        };
        if name == "head" && head.is_none() {
            if let Child::Element(node) = child {
                head = Some(node);
            }
        } else if name == "body" && body.is_none() {
            if let Child::Element(node) = child {
                body = Some(node);
            }
        } else if body.is_some() {
            after.push(child);
        } else {
            before.push(child);
        }
    }

    let mut body = body.unwrap_or_else(|| Node::new("body"));
    before.append(&mut body.children);
    before.append(&mut after);
    body.children = before;

    Inner {
        head: head.unwrap_or_else(|| Node::new("head")),
        body,
    }
}
