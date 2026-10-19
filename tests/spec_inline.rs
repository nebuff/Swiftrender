//! `TESTS.md` group E — inline formatting (T31–T40).
//!
//! Whole group waits for Phase 2 (or Phase 3 where paint is asserted). These cases pin the
//! line-breaking and whitespace rules of `SPEC.md` §6.5, which are the ones a reader is most
//! likely to disagree with — which is exactly why they are numbered.

mod common;

use common::*;

/// T31 · greedy wrapping — In: `<p>` followed by `"ab "`×60 — Out: two line fragments, widths
/// 784 and 640, `p h=40`.
///
/// Why: §6.5 breaks exactly when the next item would exceed the content width.
///
/// *Fixture corrected at the Phase 2 gate:* the original fixture was 100 consecutive `a` glyphs,
/// which contains no break opportunity and therefore contradicts T32 (an unbreakable run must
/// overflow on one line). See the note on T31 in `TESTS.md`.
#[test]
fn t31_lines_break_greedily() {
    let fixture = format!("<p>{}</p>", "ab ".repeat(60));
    let lines = boxes(&fixture);
    assert_eq!(find(&lines, "p").height, 40, "{lines:#?}");
    let runs = filter(&lines, "#text");
    assert_eq!(runs.len(), 2, "two line fragments: {lines:#?}");
    assert_eq!(
        runs[0].text_width, 784,
        "33 words at 16px plus 32 spaces at 8px"
    );
    assert_eq!(runs[1].text_width, 640, "27 words plus 26 spaces");
}

/// T32 · unbreakable word overflows — In: `<p>` followed by 200 `a` glyphs — Out: one line,
/// `p h=20`, ink past x=792, no error.
///
/// Why: §6.5 words are never split, and overflowing is not an error.
#[test]
#[ignore = "phase 2: needs inline layout"]
fn t32_unbreakable_words_overflow() {
    let fixture = format!("<p>{}</p>", "a".repeat(200));
    let lines = boxes(&fixture);
    assert_eq!(find(&lines, "p").height, 20, "one line only: {lines:#?}");
    assert!(
        raster(&fixture).has_ink(796, 26, 3, 8),
        "ink continues past the content box"
    );
}

/// T33 · center alignment — In: `p{text-align:center}` with `<p>abc</p>` — Out: first ink at
/// `x=388`.
///
/// Why: §6.5 the offset rounds once: `8 + round((784-24)/2)`.
#[test]
#[ignore = "phase 2: needs inline layout"]
fn t33_center_alignment_rounds_once() {
    const FIXTURE: &str = "<style>p{text-align:center}</style><p>abc</p>";
    assert_eq!(find(&boxes(FIXTURE), "#text").x, 388);
    assert!(
        raster(FIXTURE).has_ink(388, 26, 24, 8),
        "ink starts at the centred offset"
    );
}

/// T34 · right alignment — In: `p{text-align:right}` with `<p>abc</p>` — Out: first ink at
/// `x=768`, last ink at `x=791`.
///
/// Why: §6.5 the right offset is `W - used`.
#[test]
#[ignore = "phase 2: needs inline layout"]
fn t34_right_alignment_flushes_to_the_edge() {
    const FIXTURE: &str = "<style>p{text-align:right}</style><p>abc</p>";
    let lines = boxes(FIXTURE);
    assert_eq!(find(&lines, "#text").x, 768, "{lines:#?}");
    assert!(
        !raster(FIXTURE).has_ink(792, 0, 8, 600),
        "nothing is painted past the content box"
    );
}

/// T35 · inline fragment background — In: `.s{background:#00ff00}` on
/// `<p><span class=s>ab</span></p>` — Out: a green rect exactly `(8,24,16,20)`, white at
/// `(24,24)`.
///
/// Why: §7.1 an inline box paints the whole line box height, not just the glyph cell.
#[test]
#[ignore = "phase 3: needs paint"]
fn t35_inline_fragments_paint_the_line_box() {
    const FIXTURE: &str = "<style>.s{background:#00ff00}</style><p><span class=s>ab</span></p>";
    let image = raster(FIXTURE);
    assert_eq!(image.rgb(8, 24), (0x00, 0xff, 0x00));
    assert_eq!(
        image.rgb(23, 43),
        (0x00, 0xff, 0x00),
        "the fragment is 20px tall"
    );
    assert_eq!(
        image.rgb(24, 24),
        (0xff, 0xff, 0xff),
        "the background stops at the fragment"
    );
}

/// T36 · consecutive breaks — In: `<p>a<br><br>b</p>` — Out: `p h=60`, ink on lines 1 and 3,
/// line 2 blank.
///
/// Why: §5.5 an empty line box still occupies its height.
#[test]
#[ignore = "phase 2: needs inline layout"]
fn t36_consecutive_breaks_make_empty_lines() {
    let lines = boxes("<p>a<br><br>b</p>");
    assert_eq!(find(&lines, "p").height, 60, "{lines:#?}");
    let image = raster("<p>a<br><br>b</p>");
    assert!(image.has_ink(8, 26, 8, 8), "line 1 has ink");
    assert!(!image.has_ink(8, 46, 8, 8), "line 2 is blank");
    assert!(image.has_ink(8, 66, 8, 8), "line 3 has ink");
}

/// T37 · image placeholder default — In: `<p>x<img></p>` — Out: an img box `150×50`, `p h=50`,
/// img at `x=16`.
///
/// Why: §5.6 intrinsic size and §6.5 the atomic inline drives the line height.
#[test]
#[ignore = "phase 3: needs paint"]
fn t37_image_uses_its_intrinsic_size() {
    let lines = boxes("<p>x<img></p>");
    let image_box = find(&lines, "img");
    assert_eq!(
        (image_box.x, image_box.width, image_box.height),
        (16, 150, 50),
        "{lines:#?}"
    );
    assert_eq!(find(&lines, "p").height, 50);
    assert_eq!(raster("<p>x<img></p>").rgb(16, 8), (0xc0, 0xc0, 0xc0));
}

/// T38 · image explicit size — In: `img{width:20px;height:10px}` with `<p><img></p>` — Out:
/// img `20×10`, `p h=20`, placeholder at `(8,8)`.
///
/// Why: §5.6 CSS size overrides the intrinsic size; §6.5 `line-height` still wins the line box.
#[test]
#[ignore = "phase 3: needs paint"]
fn t38_image_css_size_overrides_intrinsic() {
    const FIXTURE: &str = "<style>img{width:20px;height:10px}</style><p><img></p>";
    let lines = boxes(FIXTURE);
    let image_box = find(&lines, "img");
    assert_eq!((image_box.width, image_box.height), (20, 10), "{lines:#?}");
    assert_eq!(find(&lines, "p").height, 20);
    assert_eq!(raster(FIXTURE).rgb(8, 8), (0xc0, 0xc0, 0xc0));
}

/// T39 · anonymous block boxes — In: `<div>text<p>para</p>tail</div>` — Out: three children
/// (`#anon`, `p`, `#anon`) with `div h=92` and document order preserved.
///
/// Why: §5.3 the hardest structural rule in M1: every block container's children are either
/// all block or all inline.
#[test]
fn t39_mixed_content_creates_anonymous_blocks() {
    let lines = boxes("<div>text<p>para</p>tail</div>");
    let div = find(&lines, "div");
    assert_eq!(div.height, 92, "{lines:#?}");
    let children: Vec<&str> = lines
        .iter()
        .filter(|item| item.tag != "html" && item.tag != "body" && item.tag != "#text")
        .map(|item| item.tag.as_str())
        .collect();
    assert_eq!(children, vec!["div", "#anon", "p", "#anon"]);
    assert_eq!(
        filter(&lines, "#anon")[0].y,
        8,
        "the first run stays above the paragraph"
    );
    assert_eq!(
        find(&lines, "p").y,
        44,
        "the paragraph sits between the two runs"
    );
}

/// T40 · whitespace collapsing — In: `<p>  a   b  </p>` — Out: the run is `"a b"`, 24px wide,
/// with first ink at `x=8`.
///
/// Why: §6.5 leading and trailing spaces must not shift alignment.
#[test]
fn t40_whitespace_collapses_and_trims() {
    let lines = boxes("<p>  a   b  </p>");
    let text = find(&lines, "#text");
    assert_eq!(text.text.as_deref(), Some("a b"), "{lines:#?}");
    assert_eq!(text.text_width, 24);
    assert_eq!(text.x, 8);
}
