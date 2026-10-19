//! Selector and rule types, plus the one definition of cascade match order (`SPEC.md` §3.2–§3.3).
//!
//! These types live here rather than beside the scanner so that scanning, the cascade and
//! `--explain` all agree about what a selector is and which rule applies last. `matching_rules`
//! is deliberately not duplicated anywhere: an explainer with its own copy of the ordering is an
//! explainer that can disagree with the renderer.

use std::fmt;

use crate::html::Node;

/// A simple selector: an optional tag name plus an optional single class (`SPEC.md` §3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    /// Tag name, lowercased, when the selector named one.
    pub tag: Option<String>,
    /// Class name, when the selector named one.
    pub class: Option<String>,
}

impl Selector {
    /// Parse `p`, `.note` or `h1.note`, returning `None` for anything else.
    pub fn parse(text: &str) -> Option<Selector> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let (tag_part, class_part) = match text.split_once('.') {
            Some((tag, class)) => (tag, Some(class)),
            None => (text, None),
        };
        let class = match class_part {
            Some(class) => {
                // A second dot is a compound selector we do not support.
                if class.is_empty() || class.contains('.') || !is_ident(class) {
                    return None;
                }
                Some(class.to_string())
            }
            None => None,
        };
        let tag = if tag_part.is_empty() {
            // `.class` with no tag part is fine; a bare `.` is not.
            class.as_ref()?;
            None
        } else {
            if !is_ident(tag_part) {
                return None;
            }
            Some(tag_part.to_ascii_lowercase())
        };
        Some(Selector { tag, class })
    }

    /// Whether this selector matches `element`.
    pub fn matches(&self, element: &Node) -> bool {
        if let Some(tag) = &self.tag {
            if &element.name != tag {
                return false;
            }
        }
        if let Some(class) = &self.class {
            if !element.classes().iter().any(|name| name == class) {
                return false;
            }
        }
        true
    }

    /// Specificity as a single sortable score: classes dominate tags (`SPEC.md` §3.3).
    pub fn score(&self) -> u32 {
        let classes = u32::from(self.class.is_some());
        let tags = u32::from(self.tag.is_some());
        classes * 16 + tags
    }
}

/// Render a selector back to the text a stylesheet would have used.
///
/// Used by `--explain`, which quotes the rule as written rather than describing it in prose.
impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(tag) = &self.tag {
            write!(f, "{tag}")?;
        }
        if let Some(class) = &self.class {
            write!(f, ".{class}")?;
        }
        Ok(())
    }
}

/// Whether `text` is a plain identifier (tag or class name).
fn is_ident(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// One `property: value` pair, kept as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// Lowercased property name.
    pub name: String,
    /// Raw value text, with any `!important` still attached (the cascade strips it).
    pub value: String,
}

/// A rule: one or more selectors sharing a declaration block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Selectors that all match the same declarations.
    pub selectors: Vec<Selector>,
    /// Declarations in source order.
    pub declarations: Vec<Declaration>,
}

/// A parsed stylesheet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stylesheet {
    /// Rules in document order.
    pub rules: Vec<Rule>,
}

/// Every rule of `sheet` that matches `element`, in the order the cascade applies them.
///
/// Ascending `(specificity, source order)`, so the last entry wins (`SPEC.md` §3.3). This is the
/// single definition of "cascade order": [`apply_sheet`] applies it, and `--explain` reports it
/// (`super::explain`). A second copy of the sort in the explainer would be a second source of
/// truth, and the first thing a debugging aid must not do is lie about the thing it explains.
pub fn matching_rules<'a>(sheet: &'a Stylesheet, element: &Node) -> Vec<(u32, usize, &'a Rule)> {
    let mut matched: Vec<(u32, usize, &Rule)> = Vec::new();
    for (index, rule) in sheet.rules.iter().enumerate() {
        let best = rule
            .selectors
            .iter()
            .filter(|selector| selector.matches(element))
            .map(Selector::score)
            .max();
        if let Some(score) = best {
            matched.push((score, index, rule));
        }
    }
    matched.sort_by_key(|(score, index, _)| (*score, *index));
    matched
}
