//! The user-agent stylesheet (`SPEC.md` §4, normative).
//!
//! This is the only place any element's default appearance is defined. Keeping it as CSS text
//! rather than Rust match arms means the UA defaults travel through exactly the same cascade as
//! author rules, which is what makes `SPEC.md` §4.3 ("the UA sheet MUST be overridable") true by
//! construction instead of by careful coding.
//!
//! Two entries deserve comment:
//!
//! * `head, title, style` are `display: none` because `SPEC.md` §2.8 says head content must not
//!   generate boxes. Making it a style rule rather than a special case in box generation keeps
//!   the box generator free of tag-name checks.
//! * `li` has no marker rule: list markers are engine behaviour (`SPEC.md` §6.8), not a CSS
//!   property, because M1 supports no `::marker` or `list-style-*`.

/// The normative UA stylesheet (`SPEC.md` §4), byte-for-byte as approved.
pub const UA_STYLESHEET: &str = "\
html, body, div, p, ul, li, h1, h2, h3 { display: block }
span, a, br, img { display: inline }
head, title, style, script, link, meta { display: none }
body { margin: 8px }
p { margin: 16px 0 }
h1 { font-size: 32px; margin: 16px 0 }
h2 { font-size: 24px; margin: 16px 0 }
h3 { font-size: 20px; margin: 16px 0 }
ul { margin: 16px 0; padding-left: 40px }
a { color: #0000ee }
";
