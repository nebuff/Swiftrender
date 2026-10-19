//! `TESTS.md` group H — robustness (T55–T58).
//!
//! T55–T57 are recovery clauses (§2.4, §2.6, §2.7) and are therefore verified at the DOM
//! layer, which is the layer those clauses define; they run in Phase 1. T58 needs the full
//! pipeline to have something to time.

mod common;

use common::*;

/// T55 · mismatched end tags — In: `<div><p>x</div>y` — Out: a deterministic tree with `y`
/// after the div, no panic.
///
/// Why: §2.6 real-world HTML is broken, and a renderer that crashes on it is useless.
#[test]
fn t55_mismatched_end_tags_are_recovered() {
    let doc = parse("<div><p>x</div>y");
    assert_eq!(elements(&doc, "div").len(), 1);
    assert_eq!(
        elements(&doc, "p").len(),
        1,
        "p is closed implicitly by </div>"
    );
    assert!(
        has_text(&doc, "y"),
        "trailing text survives: {:?}",
        all_text(&doc)
    );
    assert_eq!(body_children(&doc), vec!["element:div", "text:y"]);
}

/// T56 · text before `<html>` — In: `hello<p>x` — Out: the text becomes the first child of the
/// synthesized body, followed by the paragraph.
///
/// Why: §2.7 document order must survive the implicit wrapper synthesis.
#[test]
fn t56_leading_text_lands_in_the_synthesized_body() {
    let doc = parse("hello<p>x");
    assert_eq!(doc.root.name, "html");
    assert_eq!(body_children(&doc), vec!["text:hello", "element:p"]);
}

/// T57 · unknown entity stays literal — In: `<p>&bogus;</p>` — Out: the text `"&bogus;"`.
///
/// Why: §2.4 dropping characters silently would change measured width with no explanation.
#[test]
fn t57_unknown_entities_stay_literal() {
    let doc = parse("<p>&bogus;</p>");
    assert_eq!(all_text(&doc), "&bogus;");

    // The measured width (56px) is §6.6 applied to the 8px advance that T2, T27 and T40 pin;
    // see the layer note on T57 in TESTS.md. Nothing is deferred to a comment.
}

/// T58 · bulk input guard — In: 5,000 sibling `<div>`s — Out: a release run finishes in under
/// two seconds and produces byte-identical output twice.
///
/// Why: an accidental O(n²) in layout is invisible at three boxes and obvious at five thousand.
#[test]
#[ignore = "phase 3: needs paint to time the full pipeline"]
fn t58_bulk_input_stays_linear() {
    let fixture = "<div></div>".repeat(5_000);
    let started = std::time::Instant::now();
    let first = png_bytes(&fixture);
    let elapsed = started.elapsed();
    assert!(
        elapsed.as_secs_f64() < 2.0,
        "rendering 5,000 divs took {elapsed:?}"
    );
    assert_eq!(first, png_bytes(&fixture), "output must be deterministic");
}
