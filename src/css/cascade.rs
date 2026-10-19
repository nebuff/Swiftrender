//! The cascade (`SPEC.md` §3.3, §3.4, §3.7).
//!
//! Two origin sheets, authority in this order:
//!
//! 1. the UA sheet from [`super::ua`],
//! 2. the author sheet collected from every `<style>` element, in document order.
//!
//! Within a sheet, precedence is specificity and then source order, which
//! [`super::parse::apply_sheet`] implements by sorting matched declarations *ascending* and
//! applying them in that order — the last one applied is the winner.

use super::parse::{apply_sheet, parse, parse_author, Stylesheet};
use super::style::ComputedStyle;
use super::ua::UA_STYLESHEET;
use crate::html::{Document, Node};

/// Everything the cascade needs, parsed once per document.
#[derive(Debug, Clone, Default)]
pub struct Css {
    /// The user-agent sheet.
    pub ua: Stylesheet,
    /// The author sheet, concatenated from every `<style>` element in document order (§2.9).
    pub author: Stylesheet,
}

impl Css {
    /// Build both origin sheets for `document`.
    ///
    /// Warnings go to `warnings` so `--verbose` can report dropped rules and at-rules (§9.3).
    pub fn from_document(document: &Document, warnings: &mut Vec<String>) -> Css {
        let ua = parse(UA_STYLESHEET, warnings);
        let author = parse_author(&document.styles, warnings);
        Css { ua, author }
    }

    /// Compute the style for one element, given its parent's computed style.
    ///
    /// Inheritance happens first (`§3.7`: five properties), then the UA sheet, then the author
    /// sheet, so an author rule always beats a UA rule of equal specificity (§3.4).
    pub fn style_for(
        &self,
        element: &Node,
        parent: Option<&ComputedStyle>,
        warnings: &mut Vec<String>,
    ) -> ComputedStyle {
        let mut style = ComputedStyle::inherited_from(parent);
        apply_sheet(&self.ua, element, &mut style, parent, warnings);
        apply_sheet(&self.author, element, &mut style, parent, warnings);
        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::parse as parse_html;

    /// Build a document plus its sheets, returning both.
    fn setup(html: &str) -> (Document, Css, Vec<String>) {
        let document = parse_html(html);
        let mut warnings = Vec::new();
        let css = Css::from_document(&document, &mut warnings);
        (document, css, warnings)
    }

    /// §4.3: an author rule with the same specificity as a UA rule wins.
    #[test]
    fn author_overrides_ua_at_equal_specificity() {
        let (document, css, mut warnings) = setup("<style>p { margin: 0 }</style><p>x</p>");
        let paragraph = document.root.descendants("p")[0];
        let style = css.style_for(paragraph, None, &mut warnings);
        assert_eq!(style.margin.top, 0.0);
        assert_eq!(style.display, crate::css::Display::Block);
    }

    /// §3.3: a class beats a tag regardless of source order.
    #[test]
    fn class_beats_tag_regardless_of_order() {
        let (document, css, mut warnings) = setup(
            "<style>.k { color: #00ff00 } div { color: #ff0000 }</style><div class=\"k\">x</div>",
        );
        let div = document.root.descendants("div")[0];
        let style = css.style_for(div, None, &mut warnings);
        assert_eq!(style.color.r, 0x00);
        assert_eq!(style.color.g, 0xff);
    }

    /// §3.7: colour and text-align inherit; margin does not.
    #[test]
    fn only_the_documented_properties_inherit() {
        let (document, css, mut warnings) =
            setup("<style>div { color: #0000ff; text-align: center; margin: 10px }</style><div><p>x</p></div>");
        let div = document.root.descendants("div")[0];
        let parent_style = css.style_for(div, None, &mut warnings);
        let paragraph = document.root.descendants("p")[0];
        let style = css.style_for(paragraph, Some(&parent_style), &mut warnings);
        assert_eq!(style.color.b, 0xff, "colour inherits");
        assert_eq!(style.text_align, crate::css::TextAlign::Center);
        assert_eq!(
            style.margin.top, 16.0,
            "the UA margin applies, not the parent's 10px"
        );
    }

    /// §3.8: an unsupported value is dropped with a warning, and the rest of the rule survives.
    #[test]
    fn unsupported_values_are_dropped_individually() {
        let (document, css, mut warnings) =
            setup("<style>div { width: 50%; height: 2em; color: #123456 }</style><div></div>");
        let div = document.root.descendants("div")[0];
        let style = css.style_for(div, None, &mut warnings);
        assert_eq!(style.width, None, "width falls back to auto");
        assert_eq!(style.height, None);
        assert_eq!(
            style.color.to_hex(),
            "#123456",
            "the supported declaration survives"
        );
        assert_eq!(warnings.len(), 2, "one warning per dropped declaration");
    }
}
