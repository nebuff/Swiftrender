//! `TESTS.md` group B — the cascade (T9–T17).
//!
//! Every case needs computed styles, which arrived in Phase 2. T9 and T16 also assert pixels,
//! so they joined the running set with paint in Phase 3.

mod common;

use common::*;

/// T9 · class beats tag — In: `div{color:#ff0000} .k{color:#00ff00}` on `<div class=k>x</div>`
/// — Out: green ink, zero red pixels.
///
/// Why: §3.3 specificity ordering.
#[test]
fn t09_class_beats_tag() {
    const FIXTURE: &str = "<style>div{color:#ff0000} .k{color:#00ff00}</style><div class=k>x</div>";
    let lines = boxes(FIXTURE);
    assert_eq!(
        find(&lines, "#text").color,
        (0x00, 0xff, 0x00),
        "{lines:#?}"
    );
    assert_eq!(
        raster(FIXTURE).count((0xff, 0x00, 0x00)),
        0,
        "no red may be painted"
    );
}

/// T10 · document order breaks ties — In: `p{color:#ff0000}` then `p{color:#00ff00}` — Out:
/// green ink.
///
/// Why: §3.3 last-wins at equal specificity.
#[test]
fn t10_document_order_breaks_ties() {
    const FIXTURE: &str = "<style>p{color:#ff0000} p{color:#00ff00}</style><p>x</p>";
    assert_eq!(find(&boxes(FIXTURE), "#text").color, (0x00, 0xff, 0x00));
}
/// T11 · inherited properties — In: `div{text-align:center;color:#0000ff}` with
/// `<div><p>x</p></div>` — Out: the glyph starts at `x=396` and is blue.
///
/// Why: §3.7 inheritance through an element that never declares the property.
///
/// *Expectation corrected at the Phase 2 gate:* the original said `x=388`, which is the centring
/// offset alone. The offset is measured inside the content box, and the content box starts at
/// `x=8`, so the glyph is at `8 + round((784 − 8)/2) = 396`. T33's `388` is a different fixture
/// (three glyphs, so a different offset) and was already right.
#[test]
fn t11_inherited_properties_reach_descendants() {
    const FIXTURE: &str = "<style>div{text-align:center;color:#0000ff}</style><div><p>x</p></div>";
    let lines = boxes(FIXTURE);
    let text = find(&lines, "#text");
    assert_eq!(
        text.x, 396,
        "content-box origin + centring offset, {lines:#?}"
    );
    assert_eq!(text.color, (0x00, 0x00, 0xff));
}

/// T12 · non-inherited margin — In: `div{margin:10px}` with `<div><p>x</p></div>` — Out: the
/// `p` box top is `8+10+16 = 34` and its margin is the UA sheet's `16px 0`, not the parent's 10px.
///
/// Why: §3.7 must not leak box-model properties to children.
///
/// *Expectation corrected at the Phase 2 gate:* the original asserted `margin = 0`, which
/// contradicts the UA stylesheet's `p { margin: 16px 0 }`. What "not inherited" means is that the
/// paragraph keeps its **own** margin (16), not that it has none.
#[test]
fn t12_margin_is_not_inherited() {
    const FIXTURE: &str = "<style>div{margin:10px}</style><div><p>x</p></div>";
    let lines = boxes(FIXTURE);
    let p = find(&lines, "p");
    assert_eq!(
        p.y, 34,
        "8 (body) + 10 (div margin) + 16 (UA p margin), {lines:#?}"
    );
    assert_eq!(
        p.margin,
        [16, 0, 16, 0],
        "the UA margin, not the parent's 10px"
    );
}

/// T13 · selector list — In: `h1, p { margin: 0 }` — Out: both boxes report zero margins.
///
/// Why: §3.2 comma lists expand into independent rules.
#[test]
fn t13_selector_lists_expand() {
    const FIXTURE: &str = "<style>h1, p { margin: 0 }</style><h1>a</h1><p>b</p>";
    let lines = boxes(FIXTURE);
    for tag in ["h1", "p"] {
        assert_eq!(
            find(&lines, tag).margin,
            [0, 0, 0, 0],
            "{tag} in {lines:#?}"
        );
    }
}

/// T14 · unsupported selectors dropped whole — In: `#x{color:#ff0000} *{color:#ff0000}
/// p{color:#0000ff}` on `<p id=x>a</p>` — Out: blue ink only.
///
/// Why: §3.2 must drop the whole rule, not partially apply it.
#[test]
fn t14_unsupported_selectors_drop_the_rule() {
    const FIXTURE: &str =
        "<style>#x{color:#ff0000} *{color:#ff0000} p{color:#0000ff}</style><p id=x>a</p>";
    let lines = boxes(FIXTURE);
    assert_eq!(
        find(&lines, "#text").color,
        (0x00, 0x00, 0xff),
        "{lines:#?}"
    );
}

/// T15 · `!important` ignored — In: `p{color:#ff0000!important}` then `p{color:#00ff00}` —
/// Out: green ink, no stderr error.
///
/// Why: §3.5 parses `!important` but does not let it reorder the cascade.
#[test]
fn t15_important_is_parsed_and_ignored() {
    const FIXTURE: &str = "<style>p{color:#ff0000!important} p{color:#00ff00}</style><p>x</p>";
    assert_eq!(find(&boxes(FIXTURE), "#text").color, (0x00, 0xff, 0x00));
    let output = run_cli(FIXTURE, &["--emit", "tree", "--verbose"]);
    assert!(output.status.success(), "!important must not be an error");
}

/// T16 · `style` attribute ignored — In: `<p style="color:#ff0000">x</p>` — Out: black ink.
///
/// Why: §3.4a, the declared M1 deviation recorded in `QUESTIONS.md` Q1.
#[test]
fn t16_style_attribute_is_ignored() {
    let lines = boxes("<p style=\"color:#ff0000\">x</p>");
    assert_eq!(
        find(&lines, "#text").color,
        (0x00, 0x00, 0x00),
        "{lines:#?}"
    );
    assert_eq!(
        raster("<p style=\"color:#ff0000\">x</p>").count((0xff, 0x00, 0x00)),
        0
    );
}

/// T17 · unsupported value dropped — In: `div{width:50%; height:2em}` — Out: width 784,
/// height auto, two verbose warnings.
///
/// Why: §3.8 rejection must be per-declaration and non-fatal.
#[test]
fn t17_unsupported_units_drop_only_that_declaration() {
    const FIXTURE: &str = "<style>div{width:50%; height:2em}</style><div></div>";
    let lines = boxes(FIXTURE);
    let div = find(&lines, "div");
    assert_eq!(div.width, 784, "width falls back to auto, {lines:#?}");
    assert_eq!(div.height, 0, "height falls back to auto");

    let output = run_cli(FIXTURE, &["--emit", "tree", "--verbose"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("warning").count(),
        2,
        "one warning per dropped declaration, got: {stderr}"
    );
}
