//! Character-reference decoding (`SPEC.md` §2.4).
//!
//! Supported forms are the five XML-ish named references plus decimal and hexadecimal
//! numeric references. Anything else — including a bare `&` and an unknown name such as
//! `&bogus;` — MUST survive verbatim, because dropping characters silently would change
//! measured text width in a way no test could explain (`TESTS.md` T57).

/// Longest entity name we are willing to look at before giving up on finding a `;`.
const MAX_NAME_LEN: usize = 32;

/// Decode character references in `input`, leaving unrecognized ones untouched.
///
/// # Examples
///
/// ```
/// use swiftrender::html::decode_entities;
/// assert_eq!(decode_entities("a &amp; b"), "a & b");
/// assert_eq!(decode_entities("&#65;&#x42;"), "AB");
/// assert_eq!(decode_entities("&bogus;"), "&bogus;");
/// ```
pub fn decode_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            // Copy one whole UTF-8 scalar at a time so multi-byte text is preserved.
            let ch = input[i..].chars().next().expect("in-bounds char");
            out.push(ch);
            i += ch.len_utf8();
            continue;
        }
        match decode_one(&input[i..]) {
            Some((decoded, consumed)) => {
                out.push(decoded);
                i += consumed;
            }
            None => {
                out.push('&');
                i += 1;
            }
        }
    }
    out
}

/// Try to decode a single character reference starting at the `&`.
///
/// Returns the decoded scalar and how many bytes of input it consumed, or `None` when the
/// reference is not recognized (in which case the caller emits a literal `&`).
fn decode_one(rest: &str) -> Option<(char, usize)> {
    let bytes = rest.as_bytes();
    debug_assert_eq!(bytes.first(), Some(&b'&'));
    let semi = rest
        .char_indices()
        .skip(1)
        .take(MAX_NAME_LEN)
        .find(|(_, c)| *c == ';')
        .map(|(idx, _)| idx)?;
    let name = &rest[1..semi];
    let decoded = if let Some(num) = name.strip_prefix('#') {
        let code = if let Some(hex) = num.strip_prefix(['x', 'X']) {
            u32::from_str_radix(hex, 16).ok()?
        } else {
            num.parse::<u32>().ok()?
        };
        // Out-of-range scalars map to U+FFFD rather than failing: an invalid reference is a
        // content bug, not a reason to abort the parse.
        char::from_u32(code).unwrap_or('\u{FFFD}')
    } else {
        match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => return None,
        }
    };
    Some((decoded, semi + 1))
}

#[cfg(test)]
mod tests {
    use super::decode_entities;

    #[test]
    fn named_and_numeric_references_decode() {
        assert_eq!(decode_entities("&lt;div&gt;"), "<div>");
        assert_eq!(decode_entities("&#65;&#x41;"), "AA");
        assert_eq!(decode_entities("&quot;&apos;"), "\"'");
    }

    #[test]
    fn unknown_references_stay_literal() {
        assert_eq!(decode_entities("&bogus;"), "&bogus;");
        assert_eq!(decode_entities("a & b"), "a & b");
        assert_eq!(decode_entities("&amp"), "&amp");
    }

    #[test]
    fn non_ascii_text_survives() {
        assert_eq!(decode_entities("héllo — ✓"), "héllo — ✓");
        assert_eq!(decode_entities("&é;"), "&é;");
    }
}
