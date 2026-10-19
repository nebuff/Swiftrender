//! `TESTS.md` group D — block layout (T25–T30).
//!
//! Whole group waits for Phase 2. These are the cases most likely to regress, because they
//! pin the deliberate simplifications (no margin collapsing, no shrink-to-fit).

mod common;

use common::*;

/// T25 · sibling stacking — In: two empty `<div>`s each `height:20px` — Out: the second is at
/// `y=28`.
///
/// Why: §6.3 plain sequential flow with zero margins.
#[test]
fn t25_siblings_stack_in_flow_order() {
    const FIXTURE: &str = "<style>div{height:20px}</style><div></div><div></div>";
    let lines = boxes(FIXTURE);
    let divs = filter(&lines, "div");
    assert_eq!(divs.len(), 2, "{lines:#?}");
    assert_eq!((divs[0].y, divs[1].y), (8, 28));
}

/// T26 · margins do not collapse — In: two `<p>`s — Out: `p1 (8,24,…,20)`, `p2 (8,76,…,20)`,
/// gap 32.
///
/// Why: §6.3 the explicit anti-CSS simplification that is easiest to regress by accident.
#[test]
fn t26_vertical_margins_do_not_collapse() {
    let lines = boxes("<p>a</p><p>b</p>");
    let ps = filter(&lines, "p");
    assert_eq!(ps.len(), 2, "{lines:#?}");
    assert_eq!((ps[0].x, ps[0].y, ps[0].height), (8, 24, 20));
    assert_eq!((ps[1].x, ps[1].y, ps[1].height), (8, 76, 20));
    assert_eq!(
        ps[1].y - (ps[0].y + ps[0].height),
        32,
        "16 + 16, not collapsed"
    );
}

/// T27 · h1 metrics — In: `<h1>x</h1>` — Out: `font-size 32`, advance 16, line box 40, box
/// `h=40` at `y=24`, one glyph cell 16×32.
///
/// Why: §4 UA font size plus the §6.6 advance formula at a non-default size.
#[test]
fn t27_h1_uses_the_u32_metrics() {
    let lines = boxes("<h1>x</h1>");
    let h1 = find(&lines, "h1");
    assert_eq!((h1.y, h1.height), (24, 40), "{lines:#?}");
    let text = find(&lines, "#text");
    assert_eq!(text.font_size, 32);
    assert_eq!(text.text_width, 16, "advance = max(1, round(32 / 2))");
}

/// T28 · list indentation and markers — In: `<ul><li>a</li><li>b</li></ul>` — Out: `ul`
/// content x=48; `li` boxes 744 wide at y=24 and y=44; bullets ink at `(32,26)` and `(32,46)`.
///
/// Why: §4 UA list padding plus §6.8 marker placement relative to the li content box.
#[test]
#[ignore = "phase 3: needs paint"]
fn t28_list_indentation_and_markers() {
    let lines = boxes("<ul><li>a</li><li>b</li></ul>");
    let items = filter(&lines, "li");
    assert_eq!(items.len(), 2, "{lines:#?}");
    assert_eq!((items[0].x, items[0].width, items[0].y), (48, 744, 24));
    assert_eq!(items[1].y, 44);

    let image = raster("<ul><li>a</li><li>b</li></ul>");
    assert!(
        image.has_ink(32, 26, 8, 16),
        "first bullet is drawn left of the content box"
    );
    assert!(image.has_ink(32, 46, 8, 16), "second bullet likewise");
}

/// T29 · fixed-height parent keeps its height — In: `div{height:100px}` around one 20px line
/// — Out: `div h=100`, text top unchanged.
///
/// Why: §6.4 explicit height wins; auto height would have been 52.
#[test]
fn t29_explicit_height_is_not_shrunk_by_content() {
    const FIXTURE: &str = "<style>div{height:100px}</style><div><p>a</p></div>";
    let lines = boxes(FIXTURE);
    assert_eq!(find(&lines, "div").height, 100, "{lines:#?}");
    assert_eq!(find(&lines, "p").y, 24);
}

/// T30 · nested auto width chain — In: `div{padding:5px}` inside `body` containing an inner
/// `<div>` — Out: outer content width 774, inner width 774 at `x=13`.
///
/// Why: §6.2 the containing block for the inner div is the outer div's content box.
#[test]
fn t30_auto_width_recurses_through_padding() {
    const FIXTURE: &str = "<style>div{padding:5px}</style><div><div></div></div>";
    let lines = boxes(FIXTURE);
    let divs = filter(&lines, "div");
    assert_eq!(divs.len(), 2, "{lines:#?}");
    assert_eq!((divs[0].x, divs[0].width), (8, 784));
    assert_eq!((divs[1].x, divs[1].width), (13, 774));
}
