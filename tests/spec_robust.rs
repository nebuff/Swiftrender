//! `TESTS.md` group H — robustness (T55–T58, T60).
//!
//! T55–T57 are recovery clauses (§2.4, §2.6, §2.7) and are therefore verified at the DOM
//! layer, which is the layer those clauses define; they run in Phase 1. T58 was split at the
//! Phase 2 gate: the anti-quadratic property it guards is a *layout* property, so the tree half
//! runs now and the pixel-determinism half waits for paint as T60.

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

/// T58 · bulk input guard — In: 5,000 sibling `<div>`s — Out: a tree dump in under two seconds,
/// byte-identical across runs, containing all 5,000 boxes, no recursion limit hit.
///
/// Why: an accidental O(n²) in layout is invisible at three boxes and obvious at five thousand.
///
/// Split at the Phase 2 gate, on the T6 precedent: the property being guarded lives in layout,
/// so it should not wait for paint to exist. Measured on this fixture: 0.21 s in a debug build
/// and 0.02 s in release, i.e. the 2 s budget keeps ~10× headroom rather than being tight.
#[test]
fn t58_bulk_input_stays_linear() {
    let fixture = "<div></div>".repeat(5_000);
    let started = std::time::Instant::now();
    let first = run_cli(&fixture, &["--emit", "tree"]);
    let elapsed = started.elapsed();

    assert!(first.status.success(), "5,000 divs must layout");
    assert!(
        elapsed.as_secs_f64() < 2.0,
        "laying out 5,000 divs took {elapsed:?}"
    );

    // An O(n²) bug can also be *silent*: boxes get dropped instead of slowing the run down. The
    // count is the assertion that notices, and it doubles as the "no recursion limit hit" check.
    let boxes = parse_box_dump(&String::from_utf8_lossy(&first.stdout));
    assert_eq!(
        filter(&boxes, "div").len(),
        5_000,
        "every div must be boxed"
    );
    assert_eq!(boxes.len(), 5_002, "html + body + 5,000 divs, nothing else");

    let second = run_cli(&fixture, &["--emit", "tree"]);
    assert_eq!(
        first.stdout, second.stdout,
        "tree output must be deterministic"
    );
}

/// T60 · bulk pixel determinism — In: 5,000 sibling `<div>`s — Out: a full render finishes in
/// under two seconds and produces byte-identical PNGs twice.
///
/// Why: the pixel half of the original T58, split out when T58 moved to Phase 2; a rasteriser
/// can be deterministic in layout and non-deterministic in paint (iteration order, uninitialised
/// buffers), so the assertion has to be repeated at the paint layer rather than inherited.
#[test]
#[ignore = "phase 3: needs paint (split from T58 at the Phase 2 gate)"]
fn t60_bulk_render_pixels_are_deterministic() {
    let fixture = "<div></div>".repeat(5_000);
    let started = std::time::Instant::now();
    let first = png_bytes(&fixture);
    let elapsed = started.elapsed();
    assert!(
        elapsed.as_secs_f64() < 2.0,
        "rendering 5,000 divs took {elapsed:?}"
    );
    assert!(first.starts_with(b"\x89PNG\r\n\x1a\n"), "PNG signature");
    assert_eq!(first, png_bytes(&fixture), "output must be deterministic");
}
