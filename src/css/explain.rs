//! `--explain` — cascade introspection for a human reading a stylesheet (Phase 2.5).
//!
//! This is a **debugging aid, not part of the artifact contract** (`SPEC.md` §9.4). No numbered
//! test asserts its output; one meta-test asserts only that it runs and says something
//! structural. The format is expected to grow, and pinning it would make that growth a chore.
//!
//! Why it exists: "my CSS is being ignored" is the question a hand-written engine gets asked
//! most, and the cascade already knows the answer — it was just impossible to ask.
//!
//! The walk is in document order and carries each element's computed style, so inherited values
//! are reported as they are actually computed rather than reconstructed. For every element that
//! matches the query it prints the matched rules in the order the cascade applies them, each
//! declaration with its outcome, and the resulting computed style.
//!
//! One caveat, stated here rather than discovered later: `applied` means the declaration took
//! effect *at its turn*. A later, more specific rule may override it, and the `computed` line
//! shows which value actually survived.

use std::fmt::Write as _;

use super::cascade::Css;
use super::parse::{matching_rules, Selector};
use super::style::ComputedStyle;
use super::value::Edges;
use crate::html::{Child, Document, Node};

/// Explain `query` against `document` and return the report as text.
///
/// The caller (the CLI) has already validated the selector, so `query` always matches something
/// or nothing — never a parse failure.
pub fn explain(document: &Document, css: &Css, query: &Selector) -> String {
    let mut walker = Walker::new(css, query);
    walker.visit(&document.root, None, "html");
    walker.finish()
}

/// One traversal, carrying the state the report needs, so the recursive call stays three
/// arguments instead of eight.
struct Walker<'a> {
    /// The sheets being applied.
    css: &'a Css,
    /// The selector the user asked about.
    query: &'a Selector,
    /// The report, built as the walk proceeds.
    out: String,
    /// Warnings from style resolution. They are not printed: a dropped declaration already
    /// appears in the report as a `dropped:` line, which is more precise than a warning.
    warnings: Vec<String>,
    /// How many elements matched, so a typo is reported as "nothing matched" instead of as
    /// silence that looks like a successful explanation.
    matches: usize,
}

impl<'a> Walker<'a> {
    /// Start a report for `query`.
    fn new(css: &'a Css, query: &'a Selector) -> Walker<'a> {
        Walker {
            css,
            query,
            out: String::new(),
            warnings: Vec::new(),
            matches: 0,
        }
    }

    /// Walk one element and its subtree, carrying the parent's computed style for inheritance.
    fn visit(&mut self, element: &Node, parent: Option<&ComputedStyle>, path: &str) {
        let style = self.css.style_for(element, parent, &mut self.warnings);
        if self.query.matches(element) {
            self.matches += 1;
            self.report(element, parent, &style, path);
        }
        for child in &element.children {
            if let Child::Element(child) = child {
                let child_path = format!("{path} > {}", label(child));
                self.visit(child, Some(&style), &child_path);
            }
        }
    }

    /// Print the matched rules, their outcomes, and the computed style for one element.
    fn report(
        &mut self,
        element: &Node,
        parent: Option<&ComputedStyle>,
        style: &ComputedStyle,
        path: &str,
    ) {
        let _ = writeln!(self.out, "\nelement {path}");

        // Replay the cascade in the order it runs. The replay is not a guess: it starts from the
        // same inherited base as `style_for` and uses the same `matching_rules` ordering, and the
        // assertion below holds it to that.
        let mut replay = ComputedStyle::inherited_from(parent);
        for (origin, sheet) in [("ua", &self.css.ua), ("author", &self.css.author)] {
            for (score, index, rule) in matching_rules(sheet, element) {
                let selectors = rule
                    .selectors
                    .iter()
                    .map(Selector::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(
                    self.out,
                    "  matched rule: {origin} {selectors} (specificity {score}, source order {index})"
                );
                for declaration in &rule.declarations {
                    match replay.apply(&declaration.name, &declaration.value, parent) {
                        Ok(()) => {
                            let _ = writeln!(
                                self.out,
                                "    applied: {}: {}",
                                declaration.name, declaration.value
                            );
                        }
                        Err(reason) => {
                            let _ = writeln!(
                                self.out,
                                "    dropped: {}: {} ({reason})",
                                declaration.name, declaration.value
                            );
                        }
                    }
                }
            }
        }

        let _ = writeln!(self.out, "  computed: {}", summary(style));
        debug_assert_eq!(
            &replay, style,
            "the explain replay must reproduce the cascade it reports"
        );
    }

    /// Return the report, noting explicitly when nothing matched.
    fn finish(mut self) -> String {
        if self.matches == 0 {
            let _ = writeln!(self.out, "\nno element matches {}", self.query);
        }
        self.out
    }
}

/// `p` or `p.note` — the element as a selector would name it, for the path header.
fn label(node: &Node) -> String {
    let mut text = node.name.clone();
    for class in node.classes() {
        text.push('.');
        text.push_str(class);
    }
    text
}

/// The computed style in one line: the properties a reader is usually asking about.
fn summary(style: &ComputedStyle) -> String {
    format!(
        "display={:?} width={} height={} margin={} padding={} border={:?}/{} color={} \
         background={} font-size={}px line-height={:?} text-align={:?}",
        style.display,
        length(style.width),
        length(style.height),
        edges(&style.margin),
        edges(&style.padding),
        style.border_style,
        edges(&style.border_width),
        style.color.to_hex(),
        style
            .background
            .map(|color| color.to_hex())
            .unwrap_or_else(|| "transparent".to_string()),
        style.font_size,
        style.line_height,
        style.text_align,
    )
}

/// An optional length, with `auto` for the absent case.
fn length(value: Option<f64>) -> String {
    match value {
        Some(length) => format!("{length}px"),
        None => "auto".to_string(),
    }
}

/// Four edges as `top,right,bottom,left`.
fn edges<T: std::fmt::Display>(edges: &Edges<T>) -> String {
    format!(
        "{},{},{},{}",
        edges.top, edges.right, edges.bottom, edges.left
    )
}
