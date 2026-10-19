//! Shared helpers for the numbered Milestone 1 tests (`TESTS.md`).
//!
//! Nothing here is a test itself; every `spec_*.rs` file includes it with `mod common;`.
//! Helpers come in three tiers so that a phase can only be tested at the layer that exists:
//!
//! * **parse tier** — DOM inspection, available from Phase 1.
//! * **layout tier** — [`boxes`] and [`BoxInfo`], available from Phase 2.
//! * **paint tier** — [`raster`] and the CLI runner, available from Phase 3.
//!
//! The Phase 2 and Phase 3 helpers panic with an explicit message until their phase lands;
//! the tests that call them are `#[ignore]`d with the phase named in the attribute.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use swiftrender::html::{self, Child, Document, Node};

// ---------------------------------------------------------------------------
// Parse tier
// ---------------------------------------------------------------------------

/// Parse `html` into a [`Document`].
pub fn parse(html: &str) -> Document {
    html::parse(html)
}

/// The `--emit tree` dump of `html` at the default 800×600 canvas.
pub fn tree(html: &str) -> String {
    html::dump::dump(&parse(html), 800, 600)
}

/// Every descendant element named `name`, in document order.
pub fn elements<'a>(doc: &'a Document, name: &str) -> Vec<&'a Node> {
    doc.root.descendants(name)
}

/// Names of all elements in the document, in document order.
pub fn element_names(doc: &Document) -> Vec<String> {
    fn walk(node: &Node, out: &mut Vec<String>) {
        for child in &node.children {
            if let Child::Element(inner) = child {
                out.push(inner.name.clone());
                walk(inner, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&doc.root, &mut out);
    out
}

/// All text in the document, concatenated depth-first.
pub fn all_text(doc: &Document) -> String {
    doc.root.text()
}

/// The immediate children of the `body` element, as a compact description
/// (`"element:div"` or `"text:hello"`), so ordering assertions read clearly.
pub fn body_children(doc: &Document) -> Vec<String> {
    doc.body()
        .children
        .iter()
        .map(|child| match child {
            Child::Element(node) => format!("element:{}", node.name),
            Child::Text(text) => format!("text:{}", text.trim()),
        })
        .collect()
}

/// Whether any text node in the document contains `needle`.
pub fn has_text(doc: &Document, needle: &str) -> bool {
    all_text(doc).contains(needle)
}

/// Whether the document contains any non-empty text node at all.
pub fn has_any_text(doc: &Document) -> bool {
    has_any_text_in(&doc.root)
}

/// Recursive helper for [`has_any_text`].
fn has_any_text_in(node: &Node) -> bool {
    node.children.iter().any(|child| match child {
        Child::Element(inner) => has_any_text_in(inner),
        Child::Text(text) => !text.is_empty(),
    })
}

/// Every `.rs` file under `src/`, recursively, sorted for determinism.
pub fn src_rust_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_rust(&manifest_dir().join("src"), &mut out);
    out.sort();
    out
}

/// Recursive helper for [`src_rust_files`].
fn collect_rust(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable source directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_rust(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Workspace root, from Cargo's manifest directory.
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path of `src/` relative to the workspace root, for failure messages.
pub fn relative(path: &Path) -> String {
    path.strip_prefix(manifest_dir())
        .unwrap_or(path)
        .display()
        .to_string()
}

// ---------------------------------------------------------------------------
// Layout tier (Phase 2)
// ---------------------------------------------------------------------------

/// One laid-out box, as the Phase 2 box tree will report it (`SPEC.md` §6.7: integers only).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoxInfo {
    /// Tag name, `#anon` for an anonymous block (`SPEC.md` §5.3), or `#text` for a text run.
    pub tag: String,
    /// Left edge of the box in pixels.
    pub x: i32,
    /// Top edge of the box in pixels.
    pub y: i32,
    /// Box width in pixels.
    pub width: i32,
    /// Box height in pixels.
    pub height: i32,
    /// Margin edges as `[top, right, bottom, left]`.
    pub margin: [i32; 4],
    /// Text content of the box, if any.
    pub text: Option<String>,
    /// Measured width of [`BoxInfo::text`] in pixels (`SPEC.md` §6.6).
    pub text_width: i32,
    /// Computed font size in pixels.
    pub font_size: i32,
    /// Computed text colour.
    pub color: (u8, u8, u8),
}

/// Lay `html` out and return every box in pre-order.
///
/// **Contract (Phase 2):** this must run the real binary with `--emit tree` and parse the
/// `SPEC.md` §8.1 lines, rather than reaching into the library. Layout is the layer where an
/// in-process assertion is easiest and least meaningful: if `--emit tree` prints a box at
/// `(8,24,784,20)` while some internal struct agrees, the artifact a human would read is still
/// the one that has to be right. Tests that go through the binary already do this for PNG, SVG
/// and PPM (`png_bytes`, `svg_text`, `ppm_bytes`), and layout should behave the same way.
///
/// `BoxInfo` fields are exact `i32` values because §6.7 snaps geometry once, before the tree is
/// printed: there is no tolerance to apply and no float to compare.
///
/// Panics until Phase 2 implements layout; every caller is `#[ignore]`d until then.
pub fn boxes(_html: &str) -> Vec<BoxInfo> {
    unimplemented!("layout arrives in phase 2 (SPEC.md §5-§6)")
}

/// Find the first box whose tag equals `tag`.
pub fn find<'a>(boxes: &'a [BoxInfo], tag: &str) -> &'a BoxInfo {
    boxes
        .iter()
        .find(|item| item.tag == tag)
        .unwrap_or_else(|| panic!("no box with tag {tag:?} in {boxes:#?}"))
}

/// All boxes whose tag equals `tag`.
pub fn filter<'a>(boxes: &'a [BoxInfo], tag: &str) -> Vec<&'a BoxInfo> {
    boxes.iter().filter(|item| item.tag == tag).collect()
}

// ---------------------------------------------------------------------------
// Paint tier (Phase 3)
// ---------------------------------------------------------------------------

/// An RGB raster produced by the renderer (`SPEC.md` §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major RGB triples.
    pub pixels: Vec<u8>,
}

impl Raster {
    /// The RGB triple at `(x, y)`.
    pub fn rgb(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let index = ((y * self.width + x) * 3) as usize;
        (
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
        )
    }

    /// How many pixels equal `color`.
    pub fn count(&self, color: (u8, u8, u8)) -> usize {
        self.pixels
            .chunks_exact(3)
            .filter(|pixel| (pixel[0], pixel[1], pixel[2]) == color)
            .count()
    }

    /// Whether any pixel in `x, y, w, h` differs from white.
    pub fn has_ink(&self, x: u32, y: u32, w: u32, h: u32) -> bool {
        (y..y + h).any(|row| (x..x + w).any(|column| self.rgb(column, row) != (255, 255, 255)))
    }

    /// The set of distinct colours present, for "no anti-aliasing greys" assertions.
    pub fn colors(&self) -> BTreeSet<(u8, u8, u8)> {
        self.pixels
            .chunks_exact(3)
            .map(|pixel| (pixel[0], pixel[1], pixel[2]))
            .collect()
    }
}

/// Paint `html` at the default canvas size.
///
/// Panics until Phase 3 implements paint; every caller is `#[ignore]`d until then.
pub fn raster(_html: &str) -> Raster {
    unimplemented!("paint arrives in phase 3 (SPEC.md §7)")
}

/// Encode `html` as a PPM byte vector through the real binary.
pub fn ppm_bytes(html: &str) -> Vec<u8> {
    run_cli(html, &["--emit", "ppm"]).stdout
}

/// Encode `html` as PNG bytes through the real binary.
pub fn png_bytes(html: &str) -> Vec<u8> {
    let out = scratch_path("png");
    let result = run_cli(html, &["--emit", "png", "--out", &out.to_string_lossy()]);
    assert!(
        result.status.success(),
        "png run failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(&out).expect("png written");
    let _ = std::fs::remove_file(&out);
    bytes
}

/// Encode `html` as SVG text through the real binary.
pub fn svg_text(html: &str) -> String {
    String::from_utf8(run_cli(html, &["--emit", "svg"]).stdout).expect("svg is utf-8")
}

// ---------------------------------------------------------------------------
// Process helpers
// ---------------------------------------------------------------------------

/// Monotonic counter so parallel tests never share a scratch file.
static SCRATCH: AtomicUsize = AtomicUsize::new(0);

/// A unique scratch path outside the repository, so tests never dirty the worktree.
pub fn scratch_path(extension: &str) -> PathBuf {
    let id = SCRATCH.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "swiftrender-test-{}-{id}.{extension}",
        std::process::id()
    ))
}

/// Run the compiled binary on `html` with `args`, writing `html` to a scratch input file.
pub fn run_cli(html: &str, args: &[&str]) -> Output {
    let input = scratch_path("html");
    std::fs::write(&input, html).expect("scratch input is writable");
    let output = Command::new(env!("CARGO_BIN_EXE_swiftrender"))
        .arg(&input)
        .args(args)
        .output()
        .expect("swiftrender binary is built for integration tests");
    let _ = std::fs::remove_file(&input);
    output
}

/// Run the binary with exactly these arguments (no input file written).
pub fn run_cli_raw(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swiftrender"))
        .args(args)
        .output()
        .expect("swiftrender binary is built for integration tests")
}
