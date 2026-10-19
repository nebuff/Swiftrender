//! Golden-image regression tests (`SPEC.md` §7) — T61–T63.
//!
//! A golden is a **contract signed by a human**, so this harness never writes one. `ROADMAP.md`
//! records the sequence: the harness lands before paint exists, the human approves the first
//! render by looking at it, and only then is the approved PPM committed and `#[ignore]` removed.
//! A harness that could bless its own output would lock in whatever the first pass produced —
//! bugs included — and a green suite would then certify nothing.
//!
//! In this phase the fixtures exist and the goldens deliberately do not, so all three tests are
//! `#[ignore]`d with the phase named (checked by `tests/meta.rs`).
//!
//! The comparison reads goldens at runtime rather than through `include_bytes!`: a compile-time
//! include would make this phase uncompilable, because the files cannot exist yet. When the
//! goldens are committed the runtime read is still the right call — it lets a missing golden give
//! a diagnostic instead of a build error.

mod common;

use std::path::PathBuf;

use common::*;

/// Canvas the goldens are approved at: small enough to review by eye, large enough to show
/// margins, padding, borders and text.
const CANVAS: &str = "200x150";

/// Where the approved golden for `name` lives.
fn golden_path(name: &str) -> PathBuf {
    manifest_dir()
        .join("fixtures")
        .join(format!("{name}.expected.ppm"))
}

/// Render `name`.html to PPM bytes through the real binary at the golden canvas size.
fn render(name: &str) -> Vec<u8> {
    let fixture = manifest_dir().join("fixtures").join(format!("{name}.html"));
    let html = std::fs::read_to_string(&fixture)
        .unwrap_or_else(|error| panic!("fixture {} is unreadable: {error}", fixture.display()));
    let output = run_cli(&html, &["--emit", "ppm", "--size", CANVAS]);
    assert!(
        output.status.success(),
        "rendering {name} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

/// Compare `name`'s render against its approved golden, or fail with the first differing pixels.
fn compare_with_golden(name: &str) {
    let path = golden_path(name);
    let expected = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "golden {} is missing ({error}).\n\
             Goldens are committed only after a human has opened the render and approved it; \
             this test is #[ignore]d until then, so reaching this line means it was un-ignored \
             early (ROADMAP.md, \"Golden files: who signs them\").",
            path.display()
        )
    });

    let actual = render(name);
    if let Some(difference) = diff_pixels(&expected, &actual) {
        panic!("{name} does not match its approved golden: {difference}");
    }
}

/// A parsed `P6` PPM: dimensions plus row-major RGB bytes.
struct Ppm {
    /// Width in pixels.
    width: u32,
    /// Height in pixels.
    height: u32,
    /// RGB triples, row-major.
    pixels: Vec<u8>,
}

/// Parse a binary PPM, tolerating `#` comments in the header.
fn parse_ppm(bytes: &[u8]) -> Ppm {
    let mut at = 0;
    let magic = ppm_token(bytes, &mut at).expect("PPM magic");
    assert_eq!(magic, b"P6", "goldens are binary PPM");
    let width: u32 = number(ppm_token(bytes, &mut at).expect("PPM width"));
    let height: u32 = number(ppm_token(bytes, &mut at).expect("PPM height"));
    let maxval: u32 = number(ppm_token(bytes, &mut at).expect("PPM maxval"));
    assert_eq!(maxval, 255, "8-bit PPM");
    // Exactly one whitespace byte separates the header from the raster.
    at += 1;
    let pixels = bytes
        .get(at..)
        .unwrap_or_else(|| {
            panic!(
                "PPM is truncated: header ends at {at}, file is {}",
                bytes.len()
            )
        })
        .to_vec();
    assert_eq!(
        pixels.len(),
        (width * height * 3) as usize,
        "{width}x{height} PPM must hold {} bytes of raster",
        width * height * 3
    );
    Ppm {
        width,
        height,
        pixels,
    }
}

/// The next header token, skipping whitespace and `#` comments.
fn ppm_token<'a>(bytes: &'a [u8], at: &mut usize) -> Option<&'a [u8]> {
    loop {
        while *at < bytes.len() && bytes[*at].is_ascii_whitespace() {
            *at += 1;
        }
        if *at < bytes.len() && bytes[*at] == b'#' {
            while *at < bytes.len() && bytes[*at] != b'\n' {
                *at += 1;
            }
        } else {
            break;
        }
    }
    let start = *at;
    while *at < bytes.len() && !bytes[*at].is_ascii_whitespace() {
        *at += 1;
    }
    (*at > start).then(|| &bytes[start..*at])
}

/// Parse an ASCII header number.
fn number(token: &[u8]) -> u32 {
    std::str::from_utf8(token)
        .expect("PPM header is ASCII")
        .parse()
        .expect("PPM header number")
}

/// Describe the first difference between two PPM images, or `None` when they match.
///
/// The message names pixels, not byte offsets: "pixel (12, 40) is #ffffff, expected #dddddd" is
/// something a human can act on, and the count says whether the problem is one pixel or the whole
/// page.
fn diff_pixels(expected: &[u8], actual: &[u8]) -> Option<String> {
    let expected = parse_ppm(expected);
    let actual = parse_ppm(actual);
    if (expected.width, expected.height) != (actual.width, actual.height) {
        return Some(format!(
            "canvas is {}x{}, golden is {}x{}",
            actual.width, actual.height, expected.width, expected.height
        ));
    }

    let mut differing = 0usize;
    let mut first = None;
    for (index, (want, got)) in expected
        .pixels
        .chunks_exact(3)
        .zip(actual.pixels.chunks_exact(3))
        .enumerate()
    {
        if want != got {
            differing += 1;
            first.get_or_insert((index, want, got));
        }
    }

    let (index, want, got) = first?;
    let x = index as u32 % expected.width;
    let y = index as u32 / expected.width;
    Some(format!(
        "{differing} of {} pixels differ; first at ({x}, {y}): golden #{:02x}{:02x}{:02x}, actual #{:02x}{:02x}{:02x} (canvas {}x{})",
        expected.width * expected.height,
        want[0],
        want[1],
        want[2],
        got[0],
        got[1],
        got[2],
        expected.width,
        expected.height
    ))
}

/// Encode a P6 PPM from RGB triples, for testing the comparer without a renderer.
fn encode_ppm(width: u32, height: u32, pixels: &[(u8, u8, u8)]) -> Vec<u8> {
    assert_eq!(pixels.len(), (width * height) as usize);
    let mut bytes = format!("P6\n{width} {height}\n255\n").into_bytes();
    for (r, g, b) in pixels {
        bytes.extend_from_slice(&[*r, *g, *b]);
    }
    bytes
}

/// The comparer is the one piece of this harness whose failure mode is **silent**: a diff that
/// always answered "equal" would make every golden test pass on a wrong render. So it is tested
/// against synthetic images now, rather than first being exercised by the goldens it judges.
#[test]
fn diff_reports_the_first_differing_pixel() {
    let white = (255, 255, 255);
    let golden = encode_ppm(2, 1, &[white, (0, 0, 0)]);
    let rendered = encode_ppm(2, 1, &[white, (1, 2, 3)]);

    assert_eq!(
        diff_pixels(&golden, &golden),
        None,
        "identical images match"
    );

    let message = diff_pixels(&golden, &rendered).expect("a changed pixel must be reported");
    assert!(message.contains("1 of 2 pixels differ"), "{message}");
    assert!(
        message.contains("(1, 0)"),
        "the coordinate must be named: {message}"
    );
    assert!(
        message.contains("#000000") && message.contains("#010203"),
        "both colours must be reported: {message}"
    );
}

/// A different canvas is a different image, and must not be reported as "equal but shorter".
#[test]
fn diff_reports_a_canvas_mismatch() {
    let small = encode_ppm(1, 1, &[(0, 0, 0)]);
    let large = encode_ppm(2, 1, &[(0, 0, 0), (0, 0, 0)]);
    let message = diff_pixels(&small, &large).expect("a canvas change must be reported");
    assert!(
        message.contains("2x1") && message.contains("1x1"),
        "{message}"
    );
}

/// T61 · golden — hello.html. In: the full-feature fixture at 200×150 — Out: pixels identical to
/// the human-approved golden. Why: a regression here means paint changed for the most-used page.
#[test]
#[ignore = "phase 3: golden not approved yet (fixtures/hello.expected.ppm)"]
fn t61_golden_hello() {
    compare_with_golden("hello");
}

/// T62 · golden — list.html. In: nested `ul`/`li` with padding — Out: pixels identical to the
/// approved golden. Why: list indentation and per-item text placement are two layers' worth of
/// arithmetic that no single-value test pins end to end.
#[test]
#[ignore = "phase 3: golden not approved yet (fixtures/list.expected.ppm)"]
fn t62_golden_list() {
    compare_with_golden("list");
}

/// T63 · golden — boxes.html. In: padding, borders, an explicit width and centred text — Out:
/// pixels identical to the approved golden. Why: the box model is where "it lays out" and "it
/// looks right" can disagree, and only an image comparison can see the difference.
#[test]
#[ignore = "phase 3: golden not approved yet (fixtures/boxes.expected.ppm)"]
fn t63_golden_boxes() {
    compare_with_golden("boxes");
}
