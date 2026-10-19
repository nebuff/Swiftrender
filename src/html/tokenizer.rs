//! HTML tokenizer (`SPEC.md` §2.1–§2.5).
//!
//! The tokenizer is a single forward pass over the bytes with no backtracking and no
//! allocation for the common case. It recognizes tags, attributes and text; comments,
//! doctypes and processing instructions are recognized and **discarded**, so they can never
//! turn into boxes (`SPEC.md` §2.1, `TESTS.md` T7).
//!
//! A `<` that does not begin markup (as in `a < b`) is ordinary text.

use super::entity::decode_entities;

/// One attribute, with `value == None` for a boolean attribute such as `<img hidden>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attr {
    /// Lowercased attribute name.
    pub name: String,
    /// Attribute value with entities already decoded; `None` when the attribute had no `=`.
    pub value: Option<String>,
}

/// A token produced by [`tokenize`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// `<name …>`, or `<name …/>` when `self_closing` is set.
    StartTag {
        /// Lowercased tag name.
        name: String,
        /// Attributes in source order.
        attrs: Vec<Attr>,
        /// Whether the tag ended with `/>`.
        self_closing: bool,
    },
    /// `</name>`.
    EndTag {
        /// Lowercased tag name.
        name: String,
    },
    /// A run of text with character references decoded.
    Text(String),
}

/// Tokenize `input` into a flat token list.
///
/// # Examples
///
/// ```
/// use swiftrender::html::{tokenize, Token};
/// let tokens = tokenize("<p class=\"a\">hi</p>");
/// assert_eq!(tokens.len(), 3);
/// assert!(matches!(&tokens[0], Token::StartTag { name, .. } if name == "p"));
/// assert_eq!(tokens[1], Token::Text("hi".to_string()));
/// ```
pub fn tokenize(input: &str) -> Vec<Token> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut text_from = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        match parse_markup(input, i) {
            Some((token, next)) => {
                push_text(&mut tokens, &input[text_from..i]);
                if let Some(token) = token {
                    tokens.push(token);
                }
                i = next;
                text_from = next;
            }
            // Not markup after all: the `<` is literal text.
            None => i += 1,
        }
    }
    push_text(&mut tokens, &input[text_from..]);
    tokens
}

/// Push a text token, decoding entities and skipping empty runs.
fn push_text(tokens: &mut Vec<Token>, raw: &str) {
    if raw.is_empty() {
        return;
    }
    tokens.push(Token::Text(decode_entities(raw)));
}

/// Parse one markup construct starting at `start` (a `<`).
///
/// Returns the token (or `None` for constructs that are dropped, such as comments) and the
/// index just past the construct. Returns `None` overall when `start` does not begin markup,
/// leaving the `<` to be treated as text.
fn parse_markup(input: &str, start: usize) -> Option<(Option<Token>, usize)> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let after = start + 1;
    if after >= len {
        return None;
    }
    match bytes[after] {
        b'!' => {
            if input[after..].starts_with("!--") {
                let end = find(input, after + 3, "-->")
                    .map(|at| at + 3)
                    .unwrap_or(len);
                Some((None, end))
            } else {
                Some((None, find_byte(bytes, after, b'>').map_or(len, |at| at + 1)))
            }
        }
        b'?' => Some((None, find_byte(bytes, after, b'>').map_or(len, |at| at + 1))),
        b'/' => {
            let mut i = after + 1;
            while i < len && is_whitespace(bytes[i]) {
                i += 1;
            }
            let name_start = i;
            while i < len && is_name_byte(bytes[i]) {
                i += 1;
            }
            let name = input[name_start..i].to_ascii_lowercase();
            let end = find_byte(bytes, i, b'>').map_or(len, |at| at + 1);
            if name.is_empty() {
                Some((None, end))
            } else {
                Some((Some(Token::EndTag { name }), end))
            }
        }
        byte if byte.is_ascii_alphabetic() => Some(parse_start_tag(input, after)),
        // `<=`, `<3`, `</ >` and friends are text.
        _ => None,
    }
}

/// Parse a start tag whose name begins at `name_start`, returning it and the next index.
fn parse_start_tag(input: &str, name_start: usize) -> (Option<Token>, usize) {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = name_start;
    while i < len && is_name_byte(bytes[i]) {
        i += 1;
    }
    let name = input[name_start..i].to_ascii_lowercase();
    let mut attrs = Vec::new();
    let mut self_closing = false;

    loop {
        while i < len && is_whitespace(bytes[i]) {
            i += 1;
        }
        if i >= len {
            break;
        }
        match bytes[i] {
            b'>' => {
                i += 1;
                break;
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'>' => {
                self_closing = true;
                i += 2;
                break;
            }
            _ => {
                let (attr, next) = parse_attr(input, i);
                if let Some(attr) = attr {
                    attrs.push(attr);
                }
                i = next.max(i + 1);
            }
        }
    }

    let token = Token::StartTag {
        name,
        attrs,
        self_closing,
    };
    (Some(token), i)
}

/// Parse one attribute at `start`, returning it (`None` for a stray `/`) and the next index.
fn parse_attr(input: &str, start: usize) -> (Option<Attr>, usize) {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = start;
    let name_start = i;
    while i < len && !is_whitespace(bytes[i]) && !matches!(bytes[i], b'=' | b'>' | b'/') {
        i += 1;
    }
    let name = input[name_start..i].to_ascii_lowercase();

    while i < len && is_whitespace(bytes[i]) {
        i += 1;
    }
    let mut value = None;
    if i < len && bytes[i] == b'=' {
        i += 1;
        while i < len && is_whitespace(bytes[i]) {
            i += 1;
        }
        if i < len && (bytes[i] == b'"' || bytes[i] == b'\'') {
            let quote = bytes[i];
            i += 1;
            let value_start = i;
            while i < len && bytes[i] != quote {
                i += 1;
            }
            value = Some(decode_entities(&input[value_start..i]));
            if i < len {
                i += 1;
            }
        } else {
            let value_start = i;
            while i < len && !is_whitespace(bytes[i]) && bytes[i] != b'>' {
                i += 1;
            }
            value = Some(decode_entities(&input[value_start..i]));
        }
    }

    let attr = if name.is_empty() {
        None
    } else {
        Some(Attr { name, value })
    };
    (attr, i)
}

/// Find `needle` at or after `from`.
fn find(input: &str, from: usize, needle: &str) -> Option<usize> {
    input.get(from..)?.find(needle).map(|at| from + at)
}

/// Find a single byte at or after `from`.
fn find_byte(bytes: &[u8], from: usize, needle: u8) -> Option<usize> {
    bytes[from.min(bytes.len())..]
        .iter()
        .position(|b| *b == needle)
        .map(|at| from.min(bytes.len()) + at)
}

/// Whether `byte` may appear in a tag or attribute name.
fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
}

/// Whether `byte` is HTML whitespace for tokenizer purposes.
fn is_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_in_all_quoting_styles() {
        let tokens = tokenize(r#"<div class="a b" id='x' hidden data-y=1>"#);
        match &tokens[0] {
            Token::StartTag { attrs, .. } => {
                assert_eq!(attrs[0].value.as_deref(), Some("a b"));
                assert_eq!(attrs[1].value.as_deref(), Some("x"));
                assert_eq!(attrs[2].value, None);
                assert_eq!(attrs[3].value.as_deref(), Some("1"));
            }
            other => panic!("expected a start tag, got {other:?}"),
        }
    }

    #[test]
    fn comments_doctypes_and_stray_angle_brackets() {
        let names: Vec<String> = tokenize("<!-- x --><!DOCTYPE html>a < b<?pi?><div>")
            .into_iter()
            .filter_map(|t| match t {
                Token::StartTag { name, .. } => Some(name),
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["div".to_string()]);
        assert!(tokenize("a < b")
            .iter()
            .all(|t| matches!(t, Token::Text(_))));
    }

    #[test]
    fn self_closing_is_recorded() {
        assert!(matches!(
            tokenize("<br/>")[0],
            Token::StartTag {
                self_closing: true,
                ..
            }
        ));
    }
}
