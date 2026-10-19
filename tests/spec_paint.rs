//! `TESTS.md` group F — painting (T41–T47).
//!
//! Whole group waits for Phase 3. The assertions are deliberately about *pixels from the real
//! artifact* (canvas, then PPM bytes in T46) rather than about an internal scene graph: paint
//! is the layer where an in-process assertion can be true while the file on disk is wrong.

mod common;

use common::*;

/// T41 · background fill — In: `div{background:#123456;height:20px}` — Out: `(8,8)` is
/// `#123456`; `(791,599)` is still white.
///
/// Why: §7.1/§7.3 the background covers the padding+border box and nothing else.
#[test]
#[ignore = "phase 3: needs paint"]
fn t41_background_fills_the_box() {
    const FIXTURE: &str = "<style>div{background:#123456;height:20px}</style><div></div>";
    let image = raster(FIXTURE);
    assert_eq!(image.rgb(8, 8), (0x12, 0x34, 0x56));
    assert_eq!(image.rgb(791, 599), (0xff, 0xff, 0xff));
}

/// T42 · border defaults to `currentColor` — In: `div{color:#ff0000;border:1px solid}` — Out:
/// red border pixels.
///
/// Why: §7.4 `border-color`'s initial value comes from the element's own colour.
#[test]
#[ignore = "phase 3: needs paint"]
fn t42_border_color_defaults_to_current_color() {
    const FIXTURE: &str =
        "<style>div{color:#ff0000;border:1px solid;height:20px}</style><div></div>";
    let image = raster(FIXTURE);
    assert_eq!(image.rgb(8, 8), (0xff, 0x00, 0x00), "top border");
    assert_eq!(
        image.rgb(791, 27),
        (0xff, 0x00, 0x00),
        "bottom-right border"
    );
    assert_eq!(
        image.rgb(40, 20),
        (0xff, 0xff, 0xff),
        "the interior stays empty"
    );
}

/// T43 · child paints over parent — In: a red parent with a blue nested div — Out: `(8,8)` is
/// blue and `(8,38)` is red, i.e. the parent's paint is visible around the child.
///
/// Why: §7.1 opaque colours composite in tree order.
#[test]
#[ignore = "phase 3: needs paint"]
fn t43_children_paint_over_their_parent() {
    const FIXTURE: &str = "<style>div{background:#ff0000;height:40px} \
                           div div{background:#0000ff;height:10px}</style><div><div></div></div>";
    let image = raster(FIXTURE);
    assert_eq!(
        image.rgb(8, 8),
        (0x00, 0x00, 0xff),
        "the child covers the parent"
    );
    assert_eq!(
        image.rgb(8, 38),
        (0xff, 0x00, 0x00),
        "the parent shows below the child"
    );
}

/// T44 · glyph ink is inside its cell — In: `<p>H</p>` — Out: at least one black pixel in
/// `(8,26,8,16)`, and no ink at `y=25`.
///
/// Why: §6.5 the cell is centred in the line box; §6.6 fixes its size at 8×16.
#[test]
#[ignore = "phase 3: needs paint"]
fn t44_glyph_ink_stays_inside_the_cell() {
    let image = raster("<p>H</p>");
    assert!(image.has_ink(8, 26, 8, 16), "the glyph cell contains ink");
    assert!(
        !image.has_ink(8, 25, 8, 1),
        "the row above the cell is empty"
    );
}

/// T45 · text colour applies to ink only — In: `p{color:#ff0000}` with `<p>H</p>` — Out: every
/// non-white pixel is exactly `#ff0000`.
///
/// Why: §7.6 1-bit masks, so there must be no intermediate greys from anti-aliasing.
#[test]
#[ignore = "phase 3: needs paint"]
fn t45_text_colour_paints_ink_without_anti_aliasing() {
    let image = raster("<style>p{color:#ff0000}</style><p>H</p>");
    for colour in image.colors() {
        assert!(
            colour == (255, 255, 255) || colour == (255, 0, 0),
            "unexpected colour {colour:?} — anti-aliasing is not allowed"
        );
    }
    assert!(
        image.count((255, 0, 0)) > 0,
        "the glyph itself must be painted"
    );
}

/// T46 · empty document is white — In: `<html></html>` — Out: an 800×600 PPM whose payload is
/// entirely `0xff`.
///
/// Why: §7.2 the canvas starts opaque white, and the bytes on disk must match.
#[test]
#[ignore = "phase 3: needs paint"]
fn t46_empty_document_is_a_white_canvas() {
    let ppm = ppm_bytes("<html></html>");
    let header = b"P6\n800 600\n255\n";
    assert_eq!(&ppm[..header.len()], header);
    assert_eq!(ppm.len(), header.len() + 800 * 600 * 3);
    assert!(
        ppm[header.len()..].iter().all(|byte| *byte == 0xff),
        "an empty document must be entirely white"
    );
}
/// T47 · marker ink present — In: `<ul><li>a</li></ul>` — Out: ink in `(32,26,8,16)` and none
/// at `(40,26)`.
///
/// Why: §6.8 the bullet is 8×16 and sits 16px left of the content box, not inside it.
#[test]
#[ignore = "phase 3: needs paint"]
fn t47_list_marker_is_painted_outside_the_content_box() {
    let image = raster("<ul><li>a</li></ul>");
    assert!(
        image.has_ink(32, 26, 8, 16),
        "the bullet is painted at x=32"
    );
    assert!(
        !image.has_ink(40, 26, 1, 16),
        "the bullet does not reach the content box"
    );
}

/// T59 · class selector reaches the raster — In: `<style>.b{color:#00ff00}</style>`
/// `<div class="a b">x</div>` — Out: the `div`'s computed colour is `#00ff00` and every
/// non-white pixel is `#00ff00`.
///
/// Why: §3.2/§7.6 this is the pixel half of T6. T6 itself runs from Phase 1 but can only assert
/// the DOM layer (§2.3); the pixel assertion was originally deferred in a comment, which would
/// have left the clause unreachable from the suite. Added at the Phase 1 gate.
#[test]
#[ignore = "phase 3: needs cascade (phase 2) and paint (phase 3)"]
fn t59_class_selector_reaches_the_raster() {
    const FIXTURE: &str = "<style>.b{color:#00ff00}</style><div class=\"a b\">x</div>";

    // Cascade half (Phase 2).
    let lines = boxes(FIXTURE);
    assert_eq!(
        find(&lines, "#text").color,
        (0x00, 0xff, 0x00),
        "{lines:#?}"
    );

    // Paint half (Phase 3).
    let image = raster(FIXTURE);
    for colour in image.colors() {
        assert!(
            colour == (255, 255, 255) || colour == (0, 255, 0),
            "unexpected colour {colour:?} — only the matched class may paint"
        );
    }
    assert!(
        image.count((0x00, 0xff, 0x00)) > 0,
        "the matched class must be painted"
    );
}
