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

use std::path::{Path, PathBuf};

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
    let expected = match read_ppm(&path) {
        Ok(image) => image,
        Err(reason) => panic!(
            "{reason}\n\
             Goldens are committed only after a human has opened the render and approved it; \
             this test is #[ignore]d until then, so reaching this line means it was un-ignored \
             early (ROADMAP.md, \"Golden files: who signs them\")."
        ),
    };

    let actual = parse_ppm(&render(name))
        .unwrap_or_else(|reason| panic!("{name} did not produce a valid PPM: {reason}"));
    match compare_images(&expected, &actual) {
        Ok(Comparison::Same) => {}
        Ok(Comparison::Different(difference)) => {
            panic!("{name} does not match its approved golden: {difference}")
        }
        // A canvas or format mismatch is not "a difference to report" — it is a comparison that
        // cannot be trusted, and the caller has to say so rather than treat it as a pass.
        Err(reason) => panic!("{name} could not be compared: {reason}"),
    }
}

/// The outcome of comparing two images.
#[derive(Debug)]
enum Comparison {
    /// Every pixel matches.
    Same,
    /// The images differ; the string names the first difference.
    Different(String),
}

/// A parsed `P6` PPM: dimensions plus row-major RGB bytes.
#[derive(Debug)]
struct Ppm {
    /// Width in pixels.
    width: u32,
    /// Height in pixels.
    height: u32,
    /// RGB triples, row-major.
    pixels: Vec<u8>,
}

/// Parse a binary PPM, tolerating `#` comments in the header.
///
/// Returns a reason rather than panicking: a comparer that panics on a malformed golden is a
/// comparer whose failure mode depends on where the panic happens to be caught. The caller has to
/// handle the error, which is the point — a silent success here would certify a wrong render.
fn parse_ppm(bytes: &[u8]) -> Result<Ppm, String> {
    let mut at = 0;
    let magic = ppm_token(bytes, &mut at).ok_or("missing PPM magic")?;
    if magic != b"P6" {
        return Err(format!(
            "magic is {:?}, expected P6 (binary PPM)",
            String::from_utf8_lossy(magic)
        ));
    }
    let width = number(ppm_token(bytes, &mut at).ok_or("missing width")?)?;
    let height = number(ppm_token(bytes, &mut at).ok_or("missing height")?)?;
    let maxval = number(ppm_token(bytes, &mut at).ok_or("missing maxval")?)?;
    if maxval != 255 {
        return Err(format!("maxval is {maxval}, expected 255"));
    }
    if width == 0 || height == 0 {
        return Err(format!(
            "canvas is {width}x{height}, which cannot hold pixels"
        ));
    }
    // Exactly one whitespace byte separates the header from the raster.
    at += 1;
    let pixels = bytes
        .get(at..)
        .ok_or_else(|| format!("header ends at {at}, past the end of {}", bytes.len()))?;
    let expected_len = (width * height * 3) as usize;
    if pixels.len() != expected_len {
        return Err(format!(
            "{width}x{height} PPM holds {} raster bytes, expected {expected_len}",
            pixels.len()
        ));
    }
    Ok(Ppm {
        width,
        height,
        pixels: pixels.to_vec(),
    })
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
fn number(token: &[u8]) -> Result<u32, String> {
    std::str::from_utf8(token)
        .map_err(|_| format!("header token {token:?} is not ASCII"))?
        .parse()
        .map_err(|_| {
            format!(
                "header token {:?} is not a number",
                String::from_utf8_lossy(token)
            )
        })
}

/// Read and parse an image from disk.
///
/// The disk half of the harness is its own failure mode: a wrong path or an unreadable file must
/// name the path it tried, not surface later as a mysterious paint failure. The golden tests call
/// this, and so do the tests below — so the file-I/O path is exercised before the first golden
/// exists, rather than for the first time on the day T61 is un-ignored.
fn read_ppm(path: &Path) -> Result<Ppm, String> {
    let bytes = std::fs::read(path).map_err(|error| {
        format!(
            "golden {} is missing or unreadable: {error}",
            path.display()
        )
    })?;
    parse_ppm(&bytes)
        .map_err(|reason| format!("golden {} is not a valid PPM: {reason}", path.display()))
}

/// Compare two images pixel by pixel, already parsed.
///
/// `Err` means the inputs could not be compared at all — unparsable bytes or a different canvas —
/// and is deliberately *not* folded into [`Comparison::Different`]: a caller that only looks for
/// "same" cannot miss it, because there is no `Ok` to match on. A canvas mismatch in particular
/// must never read as a pass, and must never read as a pixel diff either: the coordinates in a
/// size-changed image are meaningless.
///
/// The `Different` message names pixels, not byte offsets: "1 of 30,000 pixels differ; first at
/// (12, 40): golden #ffffff, actual #dddddd" is something a human can act on, and the count says
/// whether the problem is one pixel or the whole page.
fn compare_ppm(expected: &[u8], actual: &[u8]) -> Result<Comparison, String> {
    let expected =
        parse_ppm(expected).map_err(|reason| format!("golden is not a valid PPM: {reason}"))?;
    let actual =
        parse_ppm(actual).map_err(|reason| format!("render is not a valid PPM: {reason}"))?;
    compare_images(&expected, &actual)
}

/// The pixel comparison itself, for callers that have already parsed both sides.
fn compare_images(expected: &Ppm, actual: &Ppm) -> Result<Comparison, String> {
    if (expected.width, expected.height) != (actual.width, actual.height) {
        return Err(format!(
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

    let Some((index, want, got)) = first else {
        return Ok(Comparison::Same);
    };
    let x = index as u32 % expected.width;
    let y = index as u32 / expected.width;
    Ok(Comparison::Different(format!(
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
    )))
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
fn compare_matches_identical_images() {
    let image = encode_ppm(2, 1, &[(255, 255, 255), (0, 0, 0)]);
    assert!(
        matches!(compare_ppm(&image, &image), Ok(Comparison::Same)),
        "identical images must match"
    );
}

/// A changed pixel must be reported, with the coordinate and both colours.
#[test]
fn compare_reports_the_first_differing_pixel() {
    let white = (255, 255, 255);
    let golden = encode_ppm(2, 1, &[white, (0, 0, 0)]);
    let rendered = encode_ppm(2, 1, &[white, (1, 2, 3)]);

    let message = match compare_ppm(&golden, &rendered) {
        Ok(Comparison::Different(message)) => message,
        other => panic!("a changed pixel must be reported, got {other:?}"),
    };
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

/// A canvas mismatch is rejected outright, not zipped to the shorter image and reported as a
/// difference — that silently passes when the render is too tall and the extra rows are blank.
#[test]
fn compare_rejects_a_canvas_mismatch() {
    let golden = encode_ppm(200, 150, &[(255, 255, 255); 200 * 150]);
    let shorter = encode_ppm(200, 149, &[(255, 255, 255); 200 * 149]);
    let error = compare_ppm(&golden, &shorter).expect_err("a canvas change must be rejected");
    assert!(
        error.contains("200x149") && error.contains("200x150"),
        "the message must name both canvases: {error}"
    );
}

/// A golden with the wrong magic is not a golden; the parser must say so rather than read the
/// bytes as a raster and report a pixel diff against nonsense.
#[test]
fn ppm_parser_rejects_bad_magic() {
    let error = parse_ppm(b"P5\n1 1\n255\n\0\0\0").expect_err("P5 is not a binary PPM");
    assert!(error.contains("P5"), "{error}");
    assert!(parse_ppm(b"not a ppm at all").is_err());
}

/// A truncated raster must be rejected, not compared up to the point where it runs out.
#[test]
fn ppm_parser_rejects_a_truncated_raster() {
    let error = parse_ppm(b"P6\n2 2\n255\n\0\0\0").expect_err("3 of 12 raster bytes is truncated");
    assert!(error.contains("expected 12"), "{error}");
}

/// The golden tests read from disk, so the disk path is tested first: a real file, written and read
/// back, compared with the same function the golden tests call. Without this, the first execution
/// of the file-I/O half would be the day a golden is un-ignored, and a path bug there would look
/// like a paint bug.
#[test]
fn ppm_roundtrip_through_disk() {
    let path =
        std::env::temp_dir().join(format!("swiftrender-roundtrip-{}.ppm", std::process::id()));
    let bytes = encode_ppm(200, 150, &[(0x12, 0x34, 0x56); 200 * 150]);
    std::fs::write(&path, &bytes).expect("scratch image is writable");

    let written = parse_ppm(&bytes).expect("encoded PPM parses");
    let parsed = read_ppm(&path).expect("disk read must succeed");
    assert!(
        matches!(compare_images(&written, &parsed), Ok(Comparison::Same)),
        "a file must compare equal to the bytes that were written to it"
    );
    let _ = std::fs::remove_file(&path);
}

/// A missing golden is an error naming the path, not a panic deep inside a byte parser.
#[test]
fn read_ppm_reports_a_missing_file() {
    let path = std::env::temp_dir().join(format!("swiftrender-absent-{}.ppm", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let error = read_ppm(&path).expect_err("a missing file must be an error");
    assert!(error.contains("missing or unreadable"), "{error}");
    assert!(error.contains(".ppm"), "the path must be named: {error}");
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
