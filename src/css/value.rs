//! CSS value types and their parsers (`SPEC.md` §3.8).
//!
//! Everything here is a leaf: no knowledge of elements, the cascade, or layout. Parsers return
//! `Result<_, String>` so the caller can turn a rejection into a warning that names the offending
//! value, which is how `--verbose` explains a dropped declaration (`SPEC.md` §9.3).
//!
//! The M1 unit rule is deliberately austere: `<number>px` and a bare `0`. Percentages, `em`, `rem`
//! and every other CSS unit are rejected per declaration rather than approximated
//! (`TESTS.md` T17).

/// The four sides of a box, in top/right/bottom/left order.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edges<T> {
    /// Top edge.
    pub top: T,
    /// Right edge.
    pub right: T,
    /// Bottom edge.
    pub bottom: T,
    /// Left edge.
    pub left: T,
}

impl<T: Copy> Edges<T> {
    /// All four sides set to `value`.
    pub fn all(value: T) -> Edges<T> {
        Edges {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

impl Edges<f64> {
    /// Top plus bottom.
    pub fn vertical(&self) -> f64 {
        self.top + self.bottom
    }

    /// Left plus right.
    pub fn horizontal(&self) -> f64 {
        self.left + self.right
    }
}

/// The `display` property (`SPEC.md` §3.8: block, inline or none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    /// Generates a block-level box.
    Block,
    /// Generates an inline-level box.
    Inline,
    /// Generates no box at all.
    None,
}

/// The `text-align` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    /// Lines start at the content-box left edge.
    Left,
    /// Lines are centred, rounding once (`SPEC.md` §6.5).
    Center,
    /// Lines are flushed to the content-box right edge.
    Right,
}

/// The `border-style` property. Only the two styles in `SPEC.md` §3.8 are representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    /// No border is painted and no space is reserved.
    None,
    /// A solid border of `border-width` on every side.
    Solid,
}

/// The `line-height` property, kept as written so a unitless factor inherits as a factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineHeight {
    /// `normal`, resolved as 1.25 × the element's own font-size (`SPEC.md` §3.8).
    Normal,
    /// A unitless multiplier of the element's own font-size.
    Number(f64),
    /// An absolute length in pixels.
    Px(f64),
}

impl LineHeight {
    /// Resolve against a font size.
    pub fn resolve(self, font_size: f64) -> f64 {
        match self {
            LineHeight::Normal => 1.25 * font_size,
            LineHeight::Number(factor) => factor * font_size,
            LineHeight::Px(px) => px,
        }
    }
}

/// An opaque RGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Color {
    /// Black, the initial value of `color` (`SPEC.md` §4.1).
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };

    /// Parse `#rgb`, `#rrggbb` or one of the named colours in `SPEC.md` §3.8.
    pub fn parse(text: &str) -> Option<Color> {
        let text = text.trim().to_ascii_lowercase();
        if let Some(hex) = text.strip_prefix('#') {
            let expand = |c: u8| -> u8 { c * 17 };
            return match hex.len() {
                3 => {
                    let digits = hex.as_bytes();
                    Some(Color {
                        r: expand(hex_digit(digits[0])?),
                        g: expand(hex_digit(digits[1])?),
                        b: expand(hex_digit(digits[2])?),
                    })
                }
                6 => {
                    let value = u32::from_str_radix(hex, 16).ok()?;
                    Some(Color {
                        r: ((value >> 16) & 0xFF) as u8,
                        g: ((value >> 8) & 0xFF) as u8,
                        b: (value & 0xFF) as u8,
                    })
                }
                _ => None,
            };
        }
        let (r, g, b) = match text.as_str() {
            "black" => (0, 0, 0),
            "white" => (255, 255, 255),
            "red" => (255, 0, 0),
            "green" => (0, 128, 0),
            "blue" => (0, 0, 255),
            "yellow" => (255, 255, 0),
            "fuchsia" | "magenta" => (255, 0, 255),
            "cyan" | "aqua" => (0, 255, 255),
            "gray" | "grey" => (128, 128, 128),
            "silver" => (192, 192, 192),
            "maroon" => (128, 0, 0),
            "olive" => (128, 128, 0),
            "navy" => (0, 0, 128),
            "purple" => (128, 0, 128),
            "teal" => (0, 128, 128),
            "lime" => (0, 255, 0),
            "orange" => (255, 165, 0),
            _ => return None,
        };
        Some(Color { r, g, b })
    }

    /// The `#rrggbb` form, used by the tree dump and later by SVG output.
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// Decode one hex digit.
fn hex_digit(byte: u8) -> Option<u8> {
    (byte as char).to_digit(16).map(|digit| digit as u8)
}

/// Build the closure used with `ok_or_else` for a bad value.
pub fn bad_value<'a>(name: &'a str, value: &'a str) -> impl Fn() -> String + 'a {
    move || format!("unsupported value {value:?} for {name}")
}

/// Parse a length: `<number>px` or a bare `0` (`SPEC.md` §3.8). Negative values are allowed; only
/// margins have any use for them.
pub fn parse_length(text: &str) -> Result<f64, String> {
    let text = text.trim();
    if text == "0" {
        return Ok(0.0);
    }
    if let Some(number) = text.strip_suffix("px") {
        if let Ok(px) = number.trim().parse::<f64>() {
            if px.is_finite() {
                return Ok(px);
            }
        }
    }
    Err(format!(
        "unsupported length {text:?} (only px is supported)"
    ))
}

/// Parse a length that may also be `auto`.
pub fn parse_optional_length(text: &str) -> Result<Option<f64>, String> {
    if text.trim().eq_ignore_ascii_case("auto") {
        Ok(None)
    } else {
        parse_length(text).map(Some)
    }
}

/// Parse the `margin`/`padding` shorthand: one to four lengths (`SPEC.md` §3.8).
///
/// `auto` margins are accepted and collapse to `0`, because M1 has no centring (§6.2).
pub fn parse_edges(text: &str) -> Result<Edges<f64>, String> {
    let values: Vec<f64> = text
        .split_whitespace()
        .map(|token| {
            if token.eq_ignore_ascii_case("auto") {
                Ok(0.0)
            } else {
                parse_length(token)
            }
        })
        .collect::<Result<Vec<f64>, String>>()?;
    match values.as_slice() {
        [all] => Ok(Edges::all(*all)),
        [vertical, horizontal] => Ok(Edges {
            top: *vertical,
            right: *horizontal,
            bottom: *vertical,
            left: *horizontal,
        }),
        [top, horizontal, bottom] => Ok(Edges {
            top: *top,
            right: *horizontal,
            bottom: *bottom,
            left: *horizontal,
        }),
        [top, right, bottom, left] => Ok(Edges {
            top: *top,
            right: *right,
            bottom: *bottom,
            left: *left,
        }),
        _ => Err(format!("expected 1 to 4 lengths, got {text:?}")),
    }
}

/// Parse `display`.
pub fn parse_display(text: &str) -> Option<Display> {
    match text.trim().to_ascii_lowercase().as_str() {
        "block" => Some(Display::Block),
        "inline" => Some(Display::Inline),
        "none" => Some(Display::None),
        _ => None,
    }
}

/// Parse `border-style`; every other CSS style is rejected (`SPEC.md` §3.8).
pub fn parse_border_style(text: &str) -> Result<BorderStyle, String> {
    match text.trim().to_ascii_lowercase().as_str() {
        "solid" => Ok(BorderStyle::Solid),
        "none" => Ok(BorderStyle::None),
        other => Err(format!("unsupported border-style {other:?}")),
    }
}

/// Parse `line-height`.
pub fn parse_line_height(text: &str) -> Result<LineHeight, String> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("normal") {
        return Ok(LineHeight::Normal);
    }
    if let Some(px) = text.strip_suffix("px") {
        if let Ok(value) = px.trim().parse::<f64>() {
            return Ok(LineHeight::Px(value));
        }
    }
    match text.parse::<f64>() {
        Ok(factor) if factor.is_finite() && factor > 0.0 => Ok(LineHeight::Number(factor)),
        _ => Err(format!("unsupported line-height {text:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §3.8: only pixel lengths exist, and `0` needs no unit.
    #[test]
    fn lengths_are_pixels_or_zero() {
        assert_eq!(parse_length("0"), Ok(0.0));
        assert_eq!(parse_length("12px"), Ok(12.0));
        assert_eq!(parse_length("2.5px"), Ok(2.5));
        assert!(parse_length("50%").is_err());
        assert!(parse_length("2em").is_err());
        assert!(parse_length("1pt").is_err());
    }

    /// §3.8: the four shorthand forms map onto the right edges.
    #[test]
    fn edge_shorthands_expand() {
        assert_eq!(parse_edges("1px"), Ok(Edges::all(1.0)));
        assert_eq!(
            parse_edges("5px 10px"),
            Ok(Edges {
                top: 5.0,
                right: 10.0,
                bottom: 5.0,
                left: 10.0
            })
        );
        assert_eq!(
            parse_edges("1px 2px 3px"),
            Ok(Edges {
                top: 1.0,
                right: 2.0,
                bottom: 3.0,
                left: 2.0
            })
        );
        assert!(parse_edges("1px 2px 3px 4px 5px").is_err());
    }

    /// §3.8: named colours and both hex forms parse; nonsense does not.
    #[test]
    fn colours_parse_in_three_forms() {
        assert_eq!(Color::parse("#f00"), Some(Color { r: 255, g: 0, b: 0 }));
        assert_eq!(
            Color::parse("#123456"),
            Some(Color {
                r: 0x12,
                g: 0x34,
                b: 0x56
            })
        );
        assert_eq!(Color::parse("red"), Some(Color { r: 255, g: 0, b: 0 }));
        assert_eq!(Color::parse("#12345"), None);
        assert_eq!(Color::parse("chartreuse"), None);
        assert_eq!(
            Color {
                r: 0x12,
                g: 0x34,
                b: 0x56
            }
            .to_hex(),
            "#123456"
        );
    }

    /// §3.8: `line-height` accepts the three documented forms and rejects the rest.
    #[test]
    fn line_height_forms_parse() {
        assert_eq!(parse_line_height("normal"), Ok(LineHeight::Normal));
        assert_eq!(parse_line_height("1.5"), Ok(LineHeight::Number(1.5)));
        assert_eq!(parse_line_height("24px"), Ok(LineHeight::Px(24.0)));
        assert!(parse_line_height("1.5em").is_err());
        assert!(parse_line_height("0").is_err());
    }
}
