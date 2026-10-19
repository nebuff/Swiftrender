//! Stylesheet parsing (`SPEC.md` §3.1–§3.3).
//!
//! A hand-written scanner with one rule: **drop whole rules, never guess.** Everything outside
//! the supported subset — at-rules, descendant combinators, id selectors, pseudo-classes —
//! removes the entire rule and records why, so `--verbose` can show a reader exactly which line
//! of their CSS the engine ignored (`SPEC.md` §3.2, §9.3).

use std::fmt;

use super::style::ComputedStyle;
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

/// Parse a stylesheet, appending a warning for every rule that was dropped.
///
/// # Examples
///
/// ```
/// use swiftrender::css::parse::parse;
/// let mut warnings = Vec::new();
/// let sheet = parse("p { margin: 0 } #x { color: #f00 }", &mut warnings);
/// assert_eq!(sheet.rules.len(), 1);
/// assert_eq!(warnings.len(), 1);
/// ```
pub fn parse(input: &str, warnings: &mut Vec<String>) -> Stylesheet {
    let text = strip_comments(input);
    let bytes = text.as_bytes();
    let mut rules = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        // A stray `}` is the tail of a nested block, such as the one left behind by an at-rule.
        while i < bytes.len()
            && (bytes[i].is_ascii_whitespace() || bytes[i] == b';' || bytes[i] == b'}')
        {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'@' {
            let end = text[i..]
                .find('}')
                .map(|at| i + at + 1)
                .unwrap_or(bytes.len());
            warnings.push(format!("dropped at-rule {}", snippet(&text[i..end])));
            i = end;
            continue;
        }
        let selector_end = match text[i..].find('{') {
            Some(at) => i + at,
            None => {
                warnings.push(format!("dropped unterminated rule {}", snippet(&text[i..])));
                break;
            }
        };
        let block_start = selector_end + 1;
        let block_end = match text[block_start..].find('}') {
            Some(at) => block_start + at,
            None => {
                warnings.push(format!("dropped unterminated rule {}", snippet(&text[i..])));
                break;
            }
        };

        let selector_text = &text[i..selector_end];
        let declarations = parse_declarations(&text[block_start..block_end]);
        match parse_selector_list(selector_text) {
            Some(selectors) => rules.push(Rule {
                selectors,
                declarations,
            }),
            None => warnings.push(format!(
                "dropped rule with unsupported selector {}",
                snippet(selector_text)
            )),
        }
        i = block_end + 1;
    }

    Stylesheet { rules }
}

/// Parse the declarations inside a rule body, or a standalone `style` fragment.
pub fn parse_declarations(input: &str) -> Vec<Declaration> {
    let text = strip_comments(input);
    let mut out = Vec::new();
    for piece in text.split(';') {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        if let Some((name, value)) = piece.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim();
            if !name.is_empty() && !value.is_empty() {
                out.push(Declaration {
                    name,
                    value: value.to_string(),
                });
            }
        }
    }
    out
}

/// Parse a comma-separated selector list; `None` when any part is unsupported.
fn parse_selector_list(text: &str) -> Option<Vec<Selector>> {
    let mut selectors = Vec::new();
    for part in text.split(',') {
        // Trim first: the spaces around `a, b` are formatting, but a space *inside* a selector is
        // a descendant combinator, which is unsupported along with the other combinators, the
        // universal selector, id selectors, attribute selectors and pseudo-classes (§3.2).
        let part = part.trim();
        if part.chars().any(|c| c.is_ascii_whitespace())
            || part.contains(['>', '+', '~', '*', '#', '[', ']', ':'])
        {
            return None;
        }
        selectors.push(Selector::parse(part)?);
    }
    if selectors.is_empty() {
        None
    } else {
        Some(selectors)
    }
}

/// Remove `/* … */` comments, replacing them with a space so tokens cannot fuse.
fn strip_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => {
                out.push(' ');
                rest = &rest[start + 2 + end + 2..];
            }
            None => {
                out.push(' ');
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// A short single-line excerpt of `text`, for warnings.
fn snippet(text: &str) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.len() > 48 {
        format!("{:?}…", &flat[..48])
    } else {
        format!("{flat:?}")
    }
}

/// Parse the author stylesheet fragments collected from every `<style>` element (§2.9).
pub fn parse_author(fragments: &[String], warnings: &mut Vec<String>) -> Stylesheet {
    let mut sheet = Stylesheet::default();
    for fragment in fragments {
        let parsed = parse(fragment, warnings);
        for rule in parsed.rules {
            sheet.rules.push(rule);
        }
    }
    sheet
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

/// Apply every declaration of `sheet` that matches `element`, in ascending precedence.
///
/// Precedence is (origin, specificity, source order), so the caller can feed the UA sheet first
/// and the author sheet second and rely on later, more specific declarations overwriting earlier
/// ones (`SPEC.md` §3.3, §3.4). Dropped declarations are reported through `warnings` (§3.8).
pub fn apply_sheet(
    sheet: &Stylesheet,
    element: &Node,
    style: &mut ComputedStyle,
    parent: Option<&ComputedStyle>,
    warnings: &mut Vec<String>,
) {
    for (_, _, rule) in matching_rules(sheet, element) {
        for declaration in &rule.declarations {
            if let Err(reason) = style.apply(&declaration.name, &declaration.value, parent) {
                warnings.push(format!("<{}> {reason}", element.name));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::parse as parse_html;

    /// T14 in miniature: unsupported selectors drop the whole rule, not just that selector.
    #[test]
    fn unsupported_selectors_drop_the_rule() {
        let mut warnings = Vec::new();
        let sheet = parse("p, #x { color: #f00 }", &mut warnings);
        assert!(sheet.rules.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    /// §3.3: a class beats a tag, and both are captured by the score.
    #[test]
    fn specificity_orders_class_above_tag() {
        let class = Selector::parse(".k").expect("class selector");
        let tag = Selector::parse("p").expect("tag selector");
        assert!(class.score() > tag.score());
        let compound = Selector::parse("p.k").expect("compound selector");
        assert!(compound.score() > class.score());
    }

    /// §2.3: `class="a b"` matches `.a` and `.b` independently.
    #[test]
    fn class_selectors_match_any_listed_class() {
        let document = parse_html("<div class=\"a b\">x</div>");
        let div = document.root.descendants("div")[0];
        assert!(Selector::parse(".b").expect("selector").matches(div));
        assert!(Selector::parse("div.b").expect("selector").matches(div));
        assert!(!Selector::parse(".c").expect("selector").matches(div));
    }

    /// Comments and `!important` must survive parsing without confusing the scanner.
    #[test]
    fn comments_and_important_are_tolerated() {
        let mut warnings = Vec::new();
        let sheet = parse("/* hi */ p { color: #f00 !important; }", &mut warnings);
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].declarations[0].name, "color");
        assert!(warnings.is_empty());
    }
}
