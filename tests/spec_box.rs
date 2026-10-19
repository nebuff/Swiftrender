//! `TESTS.md` group C — box model (T18–T24).
//!
//! Whole group waits for Phase 2: every case is a geometry or paint assertion about the
//! content/border/margin boxes of the supported properties.

mod common;

use common::*;

/// T18 · auto width fills the containing block — In: `<div></div>` — Out: `w=784`.
///
/// Why: §6.2 baseline behaviour that every later width test is measured against.
#[test]
#[ignore = "phase 2: needs block layout"]
fn t18_auto_width_fills_the_containing_block() {
    assert_eq!(find(&boxes("<div></div>"), "div").width, 784);
}

/// T19 · explicit width — In: `div{width:100px}` — Out: `(8,8,100,0)`.
///
/// Why: §6.2 an explicit width must not stretch to the containing block.
#[test]
#[ignore = "phase 2: needs block layout"]
fn t19_explicit_width_is_honoured() {
    let lines = boxes("<style>div{width:100px}</style><div></div>");
    let div = find(&lines, "div");
    assert_eq!((div.x, div.y, div.width, div.height), (8, 8, 100, 0));
}

/// T20 · padding narrows the content box — In: `div{padding:10px}` — Out: box `w=784`,
/// content width 764, content x = 18.
///
/// Why: §6.2/§6.3 padding is subtracted from the containing block, it does not grow the box.
#[test]
#[ignore = "phase 2: needs block layout"]
fn t20_padding_narrows_the_content_box() {
    const FIXTURE: &str = "<style>div{padding:10px}</style><div><p>x</p></div>";
    let lines = boxes(FIXTURE);
    let div = find(&lines, "div");
    assert_eq!((div.x, div.y, div.width), (8, 8, 784), "{lines:#?}");
    let p = find(&lines, "p");
    assert_eq!(p.x, 18, "content box starts 10px inside the border box");
    assert_eq!(p.width, 764);
}

/// T21 · border geometry and paint — In: `div{border:2px solid #ff0000}` — Out: box `w=784`,
/// content `x=10` and 780 wide; `(8,8)` and `(9,8)` red, `(10,10)` white.
///
/// Why: §7.4 the border occupies the band between the padding box and the border box.
#[test]
#[ignore = "phase 3: needs paint"]
fn t21_border_occupies_the_border_band() {
    const FIXTURE: &str = "<style>div{border:2px solid #ff0000;height:20px}</style><div></div>";
    let lines = boxes(FIXTURE);
    let div = find(&lines, "div");
    assert_eq!((div.width, div.height), (784, 20), "{lines:#?}");

    let image = raster(FIXTURE);
    assert_eq!(image.rgb(8, 8), (0xff, 0x00, 0x00));
    assert_eq!(image.rgb(9, 8), (0xff, 0x00, 0x00));
    assert_eq!(
        image.rgb(10, 10),
        (0xff, 0xff, 0xff),
        "inside the border is empty"
    );
}

/// T22 · asymmetric margins — In: `div{margin:5px 10px}` — Out: `(18,13,764,…)`.
///
/// Why: §6.3 vertical vs horizontal margin bookkeeping.
#[test]
#[ignore = "phase 2: needs block layout"]
fn t22_asymmetric_margins_offset_both_axes() {
    let lines = boxes("<style>div{margin:5px 10px}</style><div></div>");
    let div = find(&lines, "div");
    assert_eq!((div.x, div.y, div.width), (18, 13, 764), "{lines:#?}");
    assert_eq!(div.margin, [5, 10, 5, 10]);
}

/// T23 · display none removes the subtree — In: `.g{display:none}` on
/// `<div class=g>gone</div><p>z</p>` — Out: no `div` box, `p` top `= 24`, only `z` ink.
///
/// Why: §5.1 no box, no text, no paint.
#[test]
#[ignore = "phase 2: needs box generation"]
fn t23_display_none_removes_the_subtree() {
    const FIXTURE: &str = "<style>.g{display:none}</style><div class=g>gone</div><p>z</p>";
    let lines = boxes(FIXTURE);
    assert!(
        filter(&lines, "div").is_empty(),
        "no div box may exist: {lines:#?}"
    );
    assert_eq!(find(&lines, "p").y, 24, "the paragraph moves up to the top");
    assert_eq!(
        raster(FIXTURE).count((0x00, 0x00, 0x00)),
        raster("<p>z</p>").count((0x00, 0x00, 0x00))
    );
}

/// T24 · explicit height with overflow — In: `<style>div{height:10px}</style>`
/// `<div><p>a</p></div>` — Out: `div h=10`; the `p` top is 24, i.e. below the div's bottom
/// edge (y=18), and its ink is painted there.
///
/// Why: §6.4/§7.7 overflow is visible, never clipped.
#[test]
#[ignore = "phase 2: needs block layout"]
fn t24_explicit_height_overflows_visibly() {
    const FIXTURE: &str = "<style>div{height:10px}</style><div><p>a</p></div>";
    let lines = boxes(FIXTURE);
    let div = find(&lines, "div");
    assert_eq!((div.y, div.height), (8, 10), "{lines:#?}");
    let p = find(&lines, "p");
    assert_eq!(p.y, 24, "p is pushed down by its 16px margin");
    assert!(
        p.y + p.height > div.y + div.height,
        "the child overflows the parent"
    );
}
