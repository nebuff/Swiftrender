//! The computed style (`SPEC.md` §3.6–§3.9).
//!
//! Every supported property lives on [`ComputedStyle`] with an explicit initial value, so "what
//! does an unstyled `<div>` look like?" has exactly one answer in the codebase. Values are applied
//! by [`ComputedStyle::apply`], which returns a warning instead of failing: a declaration outside
//! the subset must be visible under `--verbose` but never fatal (`SPEC.md` §3.8, §9.3).

use super::value::{
    bad_value, parse_border_style, parse_display, parse_edges, parse_length, parse_line_height,
    parse_optional_length, BorderStyle, Color, Display, Edges, LineHeight, TextAlign,
};

/// Every supported property, at its computed value.
///
/// Lengths are `f64` pixels; `SPEC.md` §6.7 is the only place that rounds, and it rounds once.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    /// `display` (§3.6).
    pub display: Display,
    /// `width`, where `None` means `auto`.
    pub width: Option<f64>,
    /// `height`, where `None` means `auto`.
    pub height: Option<f64>,
    /// `margin`; `auto` collapses to `0` because M1 has no centring (`SPEC.md` §6.2).
    pub margin: Edges<f64>,
    /// `padding`.
    pub padding: Edges<f64>,
    /// `border-width`; ignored when `border_style` is [`BorderStyle::None`].
    pub border_width: Edges<f64>,
    /// `border-style`.
    pub border_style: BorderStyle,
    /// `border-color`, where `None` means `currentColor` (`SPEC.md` §7.4).
    pub border_color: Option<Color>,
    /// `background-color`, where `None` means `transparent`.
    pub background: Option<Color>,
    /// `color`, inherited.
    pub color: Color,
    /// `font-size` in whole pixels, clamped to `[1, 256]` (§3.9).
    pub font_size: i32,
    /// `text-align`, inherited.
    pub text_align: TextAlign,
    /// `line-height`, inherited as written.
    pub line_height: LineHeight,
}

impl ComputedStyle {
    /// The initial computed style (`SPEC.md` §4.1).
    pub fn initial() -> ComputedStyle {
        ComputedStyle {
            display: Display::Inline,
            width: None,
            height: None,
            margin: Edges::all(0.0),
            padding: Edges::all(0.0),
            border_width: Edges::all(0.0),
            border_style: BorderStyle::None,
            border_color: None,
            background: None,
            color: Color::BLACK,
            font_size: 16,
            text_align: TextAlign::Left,
            line_height: LineHeight::Normal,
        }
    }

    /// The starting point for an element: initial values, then the inherited properties copied
    /// from `parent` (`SPEC.md` §3.7).
    pub fn inherited_from(parent: Option<&ComputedStyle>) -> ComputedStyle {
        let mut style = ComputedStyle::initial();
        if let Some(parent) = parent {
            style.color = parent.color;
            style.font_size = parent.font_size;
            style.text_align = parent.text_align;
            style.line_height = parent.line_height;
        }
        style
    }

    /// The border widths that actually occupy space, i.e. zero unless the style is solid.
    pub fn effective_border(&self) -> Edges<f64> {
        match self.border_style {
            BorderStyle::Solid => self.border_width,
            BorderStyle::None => Edges::all(0.0),
        }
    }

    /// Apply one declaration, returning the reason it was dropped when it was.
    ///
    /// `parent` is needed because `inherit` must reach a value the cascade would not otherwise
    /// carry (`SPEC.md` §3.8 accepts `inherit` and `initial` for every property).
    pub fn apply(
        &mut self,
        name: &str,
        value: &str,
        parent: Option<&ComputedStyle>,
    ) -> Result<(), String> {
        let name = name.trim().to_ascii_lowercase();
        // `!important` is parsed and discarded (`SPEC.md` §3.5): it must not reorder anything.
        let value = value.trim().trim_end_matches("!important").trim();
        if value.eq_ignore_ascii_case("initial") {
            self.reset_property(&name);
            return Ok(());
        }
        if value.eq_ignore_ascii_case("inherit") {
            self.inherit_property(&name, parent);
            return Ok(());
        }
        match name.as_str() {
            "display" => self.display = parse_display(value).ok_or_else(bad_value(&name, value))?,
            "width" => self.width = parse_optional_length(value)?,
            "height" => self.height = parse_optional_length(value)?,
            "margin" => self.margin = parse_edges(value)?,
            "padding" => self.padding = parse_edges(value)?,
            "margin-top" => self.margin.top = parse_length(value)?,
            "margin-right" => self.margin.right = parse_length(value)?,
            "margin-bottom" => self.margin.bottom = parse_length(value)?,
            "margin-left" => self.margin.left = parse_length(value)?,
            "padding-top" => self.padding.top = parse_length(value)?,
            "padding-right" => self.padding.right = parse_length(value)?,
            "padding-bottom" => self.padding.bottom = parse_length(value)?,
            "padding-left" => self.padding.left = parse_length(value)?,
            "border" => self.apply_border(value)?,
            "border-width" => self.border_width = parse_edges(value)?,
            "border-style" => self.border_style = parse_border_style(value)?,
            "border-color" => {
                self.border_color = Some(Color::parse(value).ok_or_else(bad_value(&name, value))?)
            }
            "background-color" => {
                self.background = if value.eq_ignore_ascii_case("transparent") {
                    None
                } else {
                    Some(Color::parse(value).ok_or_else(bad_value(&name, value))?)
                }
            }
            "color" => self.color = Color::parse(value).ok_or_else(bad_value(&name, value))?,
            // Parsed and ignored: exactly one built-in font exists (`SPEC.md` §3.8).
            "font-family" => {}
            "font-size" => {
                let px = parse_length(value)?;
                self.font_size = px.round().clamp(1.0, 256.0) as i32;
            }
            "text-align" => {
                self.text_align = match value.to_ascii_lowercase().as_str() {
                    "left" => TextAlign::Left,
                    "center" => TextAlign::Center,
                    "right" => TextAlign::Right,
                    _ => return Err(bad_value(&name, value)()),
                }
            }
            "line-height" => self.line_height = parse_line_height(value)?,
            other => return Err(format!("unsupported property {other:?}")),
        }
        Ok(())
    }

    /// Apply the `border` shorthand (`SPEC.md` §3.8), which accepts width, style and colour in any
    /// order.
    fn apply_border(&mut self, value: &str) -> Result<(), String> {
        for token in value.split_whitespace() {
            if token.eq_ignore_ascii_case("solid") {
                self.border_style = BorderStyle::Solid;
            } else if token.eq_ignore_ascii_case("none") {
                self.border_style = BorderStyle::None;
            } else if let Ok(length) = parse_length(token) {
                self.border_width = Edges::all(length);
            } else if let Some(color) = Color::parse(token) {
                self.border_color = Some(color);
            } else {
                return Err(format!("unsupported border token {token:?}"));
            }
        }
        Ok(())
    }

    /// Reset one property to its initial value, for the `initial` keyword.
    fn reset_property(&mut self, name: &str) {
        let initial = ComputedStyle::initial();
        match name {
            "display" => self.display = initial.display,
            "width" => self.width = initial.width,
            "height" => self.height = initial.height,
            "margin" => self.margin = initial.margin,
            "padding" => self.padding = initial.padding,
            "border" | "border-width" => self.border_width = initial.border_width,
            "border-style" => self.border_style = initial.border_style,
            "border-color" => self.border_color = initial.border_color,
            "background-color" => self.background = initial.background,
            "color" => self.color = initial.color,
            "font-size" => self.font_size = initial.font_size,
            "text-align" => self.text_align = initial.text_align,
            "line-height" => self.line_height = initial.line_height,
            _ => {}
        }
    }

    /// Copy one property's value out of `parent`, for the `inherit` keyword.
    fn inherit_property(&mut self, name: &str, parent: Option<&ComputedStyle>) {
        let Some(parent) = parent else { return };
        match name {
            "display" => self.display = parent.display,
            "width" => self.width = parent.width,
            "height" => self.height = parent.height,
            "margin" => self.margin = parent.margin,
            "padding" => self.padding = parent.padding,
            "border" | "border-width" => self.border_width = parent.border_width,
            "border-style" => self.border_style = parent.border_style,
            "border-color" => self.border_color = parent.border_color,
            "background-color" => self.background = parent.background,
            "color" => self.color = parent.color,
            "font-size" => self.font_size = parent.font_size,
            "text-align" => self.text_align = parent.text_align,
            "line-height" => self.line_height = parent.line_height,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §3.9: font sizes are clamped and rounded to whole pixels.
    #[test]
    fn font_size_is_clamped() {
        let mut style = ComputedStyle::initial();
        assert!(style.apply("font-size", "20.6px", None).is_ok());
        assert_eq!(style.font_size, 21);
        assert!(style.apply("font-size", "0px", None).is_ok());
        assert_eq!(style.font_size, 1, "clamped to the lower bound");
        assert!(style.apply("font-size", "900px", None).is_ok());
        assert_eq!(style.font_size, 256, "clamped to the upper bound");
    }

    /// §3.5: `!important` is accepted and does not change what the declaration sets.
    #[test]
    fn important_is_stripped() {
        let mut style = ComputedStyle::initial();
        assert!(style.apply("color", "#00ff00 !important", None).is_ok());
        assert_eq!(style.color.g, 0xff);
    }

    /// §3.8: the `border` shorthand accepts its three parts in any order.
    #[test]
    fn border_shorthand_is_order_independent() {
        let mut first = ComputedStyle::initial();
        first.apply("border", "1px solid #ff0000", None).unwrap();
        let mut second = ComputedStyle::initial();
        second.apply("border", "solid red 1px", None).unwrap();
        assert_eq!(first.border_style, BorderStyle::Solid);
        assert_eq!(first.border_width.top, 1.0);
        assert_eq!(first.border_color, second.border_color);
        assert_eq!(first.effective_border().top, 1.0);
    }

    /// §3.8: an unsupported property and an unsupported value both drop, without panicking.
    #[test]
    fn unsupported_declarations_report_a_reason() {
        let mut style = ComputedStyle::initial();
        assert!(style.apply("float", "left", None).is_err());
        assert!(style.apply("padding", "10%", None).is_err());
        assert!(style.apply("text-align", "justify", None).is_err());
    }

    /// §3.8: `inherit` and `initial` move the property to the parent value and the initial value.
    #[test]
    fn inherit_and_initial_keywords_work() {
        let mut parent = ComputedStyle::initial();
        parent.color = Color { r: 1, g: 2, b: 3 };
        parent.margin = Edges::all(7.0);
        let mut style = ComputedStyle::inherited_from(Some(&parent));
        style.apply("color", "initial", Some(&parent)).unwrap();
        assert_eq!(style.color, Color::BLACK);
        style.apply("margin", "inherit", Some(&parent)).unwrap();
        assert_eq!(style.margin.top, 7.0);
    }
}
