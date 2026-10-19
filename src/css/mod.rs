//! CSS subset: parsing, selectors and the cascade (`SPEC.md` §3).
//!
//! **Phase 2.** Nothing here is implemented yet; this placeholder exists so the module
//! layout is reviewable in Phase 1 and so the Phase 2 contract has a home.
//!
//! Planned surface (all public items will carry doc comments, `SPEC.md` §10.2):
//!
//! * `Stylesheet::parse(text) -> Stylesheet` — a rule list, with unsupported rules dropped
//!   and reported through [`crate::html::Document::warnings`]-style diagnostics.
//! * `Selector` — a simple selector: optional tag plus optional single class (§3.2).
//! * `StyleMap` — computed values for the supported properties (§3.6), with inheritance
//!   applied for exactly the five inherited properties (§3.7).
//! * `ua::STYLESHEET` — the normative user-agent sheet from §4, compiled in as a string.

/// Declaration-level placeholder for the Phase 2 cascade.
///
/// Present only so that the module surface is visible during Phase 1 review.
pub const SUPPORTED_PROPERTIES: &[&str] = &[
    "display",
    "width",
    "height",
    "margin",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "padding",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "border",
    "border-width",
    "border-style",
    "border-color",
    "background-color",
    "color",
    "font-size",
    "font-family",
    "text-align",
    "line-height",
];

/// Properties inherited by default (`SPEC.md` §3.7).
pub const INHERITED_PROPERTIES: &[&str] = &[
    "color",
    "font-size",
    "font-family",
    "text-align",
    "line-height",
];
