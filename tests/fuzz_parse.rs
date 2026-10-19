//! Randomized robustness test for the tokenizer and tree builder.
//!
//! Not part of the numbered list in `TESTS.md`: it asserts a *property* (never panic, never grow
//! without bound) rather than a clause, so it has no clause ID to cite and no phase tag — it runs
//! from Phase 1 and must keep running forever.
//!
//! Three deliberate choices:
//!
//! * **No crate.** The generator is a 64-bit LCG written here, in the same spirit as the
//!   hand-rolled PNG layer (`QUESTIONS.md` Q3). A fuzz *framework* would be a dependency; a fuzz
//!   *loop* is twenty lines.
//! * **Fixed seeds.** A failure reproduces exactly, and the run is byte-for-byte the same in CI
//!   as on a laptop, which is the same determinism `SPEC.md` §8.5 demands of the renderer.
//! * **`catch_unwind`, not just a call.** The point is to *observe* a panic and report the input
//!   that caused it, rather than to abort with a backtrace that names only the byte offset.

use std::panic::{catch_unwind, AssertUnwindSafe};

use swiftrender::html::{self, Child, Node};

/// Deterministic 64-bit linear congruential generator (Knuth's MMIX constants).
struct Lcg(u64);

impl Lcg {
    /// Seed the generator.
    fn new(seed: u64) -> Lcg {
        Lcg(seed)
    }

    /// Next 32-bit value, taken from the high bits where an LCG is strongest.
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }

    /// A value in `0..bound`, or `0` when `bound` is zero.
    fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            0
        } else {
            self.next_u32() % bound
        }
    }
}

/// Markup fragments a real document might contain, including ones the engine deliberately
/// mishandles: unknown tags, unterminated entities, bare angle brackets, stray quotes.
const PIECES: &[&str] = &[
    "<div>",
    "</div>",
    "<p>",
    "</p>",
    "<br>",
    "<br/>",
    "<img src=x>",
    "<span class=\"a b\">",
    "</span>",
    "<style>p{color:#f00}</style>",
    "<!-- comment -->",
    "<!-- unterminated",
    "<!DOCTYPE html>",
    "<ul>",
    "<li>",
    "</li>",
    "</ul>",
    "<h1>",
    "</h1>",
    "<a href='y'>",
    "</a>",
    "<html>",
    "<body>",
    "</body>",
    "</html>",
    "<title>t</title>",
    "<foo bar=baz>",
    "</>",
    "<?pi?>",
    "&amp;",
    "&bogus;",
    "&#65;",
    "&#x110000;",
    "&",
    "text ",
    "\n\t ",
    "<",
    ">",
    "/",
    "=",
    "\"",
    "'",
    "é",
    "✓",
];

/// Build a random HTML-ish document of roughly `length` bytes.
///
/// Output is always valid UTF-8: raw noise is generated from the printable ASCII range, and the
/// multi-byte pieces above exercise the tokenizer's char-boundary handling.
fn random_html_ish(rng: &mut Lcg, length: usize) -> String {
    let mut out = String::with_capacity(length + 8);
    while out.len() < length {
        if rng.below(6) == 0 {
            let count = rng.below(4) + 1;
            for _ in 0..count {
                // 0x20..=0x7e: printable ASCII, so noise can still look like markup.
                out.push((0x20 + rng.below(0x5f)) as u8 as char);
            }
        } else {
            out.push_str(PIECES[rng.below(PIECES.len() as u32) as usize]);
        }
    }
    out
}

/// Count every node in a subtree, elements and text runs alike.
fn count_nodes(node: &Node) -> usize {
    1 + node
        .children
        .iter()
        .map(|child| match child {
            Child::Element(inner) => count_nodes(inner),
            Child::Text(_) => 1,
        })
        .sum::<usize>()
}

/// Parse `input` and assert the invariants that must hold for *any* byte sequence.
///
/// These are the properties the rest of the engine assumes: a root that is always `html`, a
/// document whose size stays proportional to its input, and a depth that never exceeds the cap
/// in `SPEC.md` §2.10.
fn check_invariants(input: &str) {
    let tokens = html::tokenize(input);
    let document = html::parse(input);

    assert_eq!(document.root.name, "html", "the root is always html (§2.7)");
    assert_eq!(
        document.body().name,
        "body",
        "a body is always synthesized (§2.7)"
    );

    // Growth must stay linear in the input. Every element costs at least one byte and text runs
    // cannot outnumber elements by more than one, so the bound is generous but still catches
    // anything quadratic.
    let nodes = count_nodes(&document.root);
    let bound = input.len() + 8;
    assert!(
        nodes <= bound,
        "{} nodes from {} bytes of input (bound {bound})",
        nodes,
        input.len()
    );

    let text = document.root.text();
    assert!(
        text.len() <= input.len() + 1,
        "{} bytes of text from {} bytes of input",
        text.len(),
        input.len()
    );

    assert!(
        document.root.max_depth() <= swiftrender::MAX_DEPTH,
        "depth {} exceeds the cap (§2.10)",
        document.root.max_depth()
    );

    // Tokens cannot outnumber bytes, and the tree cannot invent tokens.
    assert!(tokens.len() <= input.len() + 1);
}

/// 2,000 short documents from each of 8 independent seeds — 16,000 inputs covering empty
/// documents, truncated tags, lone delimiters and every fragment in `PIECES`.
///
/// A sweep rather than one long run: one seed that happens to miss a shape of input proves
/// nothing about the next, and the sweep still finishes in about a second.
#[test]
fn fuzz_short_inputs_never_panic() {
    for seed in 0..8u64 {
        let mut rng = Lcg::new(0x00C0_FFEE ^ seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        for iteration in 0..2_000 {
            let length = rng.below(256) as usize;
            let input = random_html_ish(&mut rng, length);
            let outcome = catch_unwind(AssertUnwindSafe(|| check_invariants(&input)));
            assert!(
                outcome.is_ok(),
                "panic on seed {seed} iteration {iteration}, input {input:?}"
            );
        }
    }
}

/// 50 documents at the tokenizer's full working size, to cover the paths that only matter when
/// there is enough input for many nested elements and long text runs.
#[test]
fn fuzz_large_inputs_never_panic() {
    let mut rng = Lcg::new(0xBEEF);
    for iteration in 0..50 {
        let input = random_html_ish(&mut rng, 64 * 1024);
        let outcome = catch_unwind(AssertUnwindSafe(|| check_invariants(&input)));
        assert!(
            outcome.is_ok(),
            "panic on iteration {iteration} ({} bytes)",
            input.len()
        );
    }
}

/// The generator itself must be deterministic, or a failure report is worthless.
#[test]
fn generator_is_deterministic() {
    let first = random_html_ish(&mut Lcg::new(1), 512);
    let second = random_html_ish(&mut Lcg::new(1), 512);
    assert_eq!(first, second);
    assert_ne!(first, random_html_ish(&mut Lcg::new(2), 512));
}
