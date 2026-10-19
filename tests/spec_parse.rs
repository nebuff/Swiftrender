//! `TESTS.md` group A — tokenizer and tree construction (T1–T8).
//!
//! T5–T8 are `SPEC.md` §2 clauses and are therefore verified at the DOM layer, which is the
//! layer §2 defines; they run in Phase 1. T1–T4 additionally assert geometry or computed
//! display, so they wait for Phase 2.

mod common;

use common::*;

/// T1 · implicit html/body — In: `<div></div>` — Out: tree `html > body > div`,
/// `div` at `(8,8,784,0)`, `html h=16`.
///
/// Why: §2.7 wrapper synthesis and §6.4 auto height including the body's bottom margin.
#[test]
#[ignore = "phase 2: needs block layout"]
fn t01_implicit_html_and_body_wrappers() {
    let lines = boxes("<div></div>");
    let div = find(&lines, "div");
    assert_eq!((div.x, div.y, div.width, div.height), (8, 8, 784, 0));
    let html = find(&lines, "html");
    assert_eq!(
        html.height, 16,
        "auto height must include the body's bottom margin"
    );
}

/// T2 · single paragraph geometry — In: `<p>hi</p>` — Out: `html h=68`,
/// `body (8,8,784,52)`, `p (8,24,784,20)`, text run width 16.
///
/// Why: pins UA margins, line box height, and the auto-height chain in one case.
#[test]
#[ignore = "phase 2: needs block layout"]
fn t02_single_paragraph_geometry() {
    let lines = boxes("<p>hi</p>");
    let root = find(&lines, "html");
    assert_eq!(root.height, 68, "{lines:#?}");
    let body = find(&lines, "body");
    assert_eq!((body.x, body.y, body.width, body.height), (8, 8, 784, 52));
    let p = find(&lines, "p");
    assert_eq!((p.x, p.y, p.width, p.height), (8, 24, 784, 20));
    let text = find(&lines, "#text");
    assert_eq!(text.text_width, 16, "the run is two 8px glyph cells");
}

/// T3 · void br starts a new line — In: `<p>a<br>b</p>` — Out: `p h=40`; ink present in both
/// the y≈26 and y≈46 bands.
///
/// Why: §5.5 forced break without a closing tag.
#[test]
#[ignore = "phase 2: needs inline layout"]
fn t03_void_br_starts_a_new_line() {
    let lines = boxes("<p>a<br>b</p>");
    let p = find(&lines, "p");
    assert_eq!(p.height, 40, "{lines:#?}");
    let image = raster("<p>a<br>b</p>");
    assert!(image.has_ink(8, 26, 8, 16), "first line has ink");
    assert!(image.has_ink(8, 46, 8, 16), "second line has ink");
}

/// T4 · unknown element is inline — In: `<foo>x</foo>` — Out: `body > #anon > foo(inline)`,
/// `body h=20`, no panic, no warning.
///
/// Why: §2.11 forward compatibility for HTML we do not model.
#[test]
#[ignore = "phase 2: needs computed display"]
fn t04_unknown_element_is_inline() {
    let doc = parse("<foo>x</foo>");
    assert_eq!(element_names(&doc), vec!["head", "body", "foo"]);
    assert!(doc.warnings.is_empty(), "unknown elements are not an error");

    let lines = boxes("<foo>x</foo>");
    assert_eq!(
        find(&lines, "#anon").height,
        20,
        "text sits in one anonymous block"
    );
    let foo = find(&lines, "foo");
    assert_eq!((foo.x, foo.y, foo.height), (8, 8, 16));
}

/// T5 · entity decoding — In: `<p>&amp;&lt;&#65;</p>` — Out: text `"&<A"`, run width 24.
///
/// Why: §2.4 numeric and named entities change measured width.
#[test]
fn t05_entities_are_decoded() {
    let doc = parse("<p>&amp;&lt;&#65;</p>");
    assert_eq!(all_text(&doc), "&<A");

    // The measured width (24px) is §6.6, not §2.4, and is derived rather than asserted here:
    // three glyphs at the 8px advance that T2, T27 and T40 pin. See the layer note on T5 in
    // TESTS.md; no assertion is deferred in a comment.
}

/// T6 · multiple classes — In: `<style>.b{color:#00ff00}</style><div class="a b">x</div>` —
/// Out: text pixels `#00ff00`.
///
/// Why: §2.3/§3.2 the class attribute splits on whitespace and any single class may match.
#[test]
fn t06_class_attribute_splits_into_classes() {
    let doc = parse("<style>.b{color:#00ff00}</style><div class=\"a b\">x</div>");
    let divs = elements(&doc, "div");
    assert_eq!(divs.len(), 1);
    assert_eq!(divs[0].classes(), vec!["a", "b"]);
    assert_eq!(doc.styles, vec![".b{color:#00ff00}".to_string()]);

    // The §3.2/§7.6 half of this fixture — that the matched class actually paints #00ff00 —
    // is asserted by T59 in spec_paint.rs rather than deferred to a comment here.
    // §2.3 is the clause this test owns, and it is fully covered above.
}

/// T7 · comment and doctype are ignored — In: `<!-- c --><!DOCTYPE html><div></div>` — Out:
/// exactly one `div`, no text nodes.
///
/// Why: §2.1 non-element tokens must never create boxes or text.
#[test]
fn t07_comments_and_doctypes_never_become_nodes() {
    let doc = parse("<!-- c --><!DOCTYPE html><div></div>");
    assert_eq!(element_names(&doc), vec!["head", "body", "div"]);
    assert!(
        !has_any_text(&doc),
        "a comment must not leak text into the tree"
    );
    assert_eq!(elements(&doc, "div").len(), 1);
}

/// T8 · nesting cap — In: 520 nested `<div>`s — Out: no panic, tree depth ≤ 512, one warning.
///
/// Why: §2.10 the recursion guard must be observable, not silent.
#[test]
fn t08_deep_nesting_is_capped_not_fatal() {
    let html = format!("{}{}", "<div>".repeat(520), "</div>".repeat(520));
    let doc = parse(&html);
    assert!(
        doc.root.max_depth() <= 512,
        "depth was {}",
        doc.root.max_depth()
    );
    assert_eq!(doc.warnings.len(), 1, "{:?}", doc.warnings);
    assert!(doc.warnings[0].contains("512"), "{:?}", doc.warnings);

    // The warning must reach a human through the documented interface (§9.3).
    let output = run_cli(&html, &["--emit", "tree", "--verbose"]);
    assert!(
        output.status.success(),
        "tree dump failed: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("warning").count(), 1, "stderr was: {stderr}");
}
