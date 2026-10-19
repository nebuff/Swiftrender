//! CSS subset: parsing, selectors and the cascade (`SPEC.md` §3).
//!
//! The subset is deliberately small and every deviation is documented:
//!
//! * selectors are `tag`, `.class`, `tag.class` and comma lists (§3.2),
//! * the `style` attribute is ignored entirely (§3.4a, `QUESTIONS.md` Q1),
//! * `!important` is parsed and ignored (§3.5),
//! * only pixel lengths exist; percentages and `em` are rejected per declaration (§3.8).
//!
//! Layout consumes the result as a [`style::ComputedStyle`] per element; nothing else in the
//! engine needs to know that CSS exists.
//!
//! * [`value`] — leaf value types and their parsers,
//! * [`style`] — the computed style and declaration application,
//! * [`parse`] — the stylesheet scanner,
//! * [`cascade`] — origin sheets and per-element resolution,
//! * [`ua`] — the normative UA sheet,
//! * [`explain`] — the `--explain` report (§9.4), a debugging aid and not part of the contract.

pub mod cascade;
pub mod explain;
pub mod parse;
pub mod style;
pub mod ua;
pub mod value;

pub use cascade::Css;
pub use parse::{
    matching_rules, parse, parse_declarations, Declaration, Rule, Selector, Stylesheet,
};
pub use style::ComputedStyle;
pub use value::{BorderStyle, Color, Display, Edges, LineHeight, TextAlign};
