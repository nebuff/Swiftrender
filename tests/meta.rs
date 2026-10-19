//! Meta-tests: invariants about the test suite itself, not about the renderer.
//!
//! `TESTS.md` is the contract, and two of its rules are about bookkeeping rather than
//! rendering: a test may only be deleted if its clause was removed from `SPEC.md` first, and an
//! `#[ignore]`d test may only disappear when the phase named in its attribute lands. Checking
//! those by hand with `grep` is a habit; this file turns the habit into a check that fails in CI.
//!
//! These are deliberately **not** numbered spec tests: the numbered list is the contract with
//! `SPEC.md`, and meta-tests would muddy the count.

mod common;

use std::path::{Path, PathBuf};

use common::run_cli;

/// Number of numbered tests defined by `TESTS.md` (T59 at the Phase 1 gate, T60 at the Phase 2 gate
/// when T58 split, T61–T63 at the Phase 2.75 gate for the golden harness).
const EXPECTED_SPEC_TESTS: usize = 63;

/// How many numbered tests are `#[ignore]`d at this phase.
///
/// * Phase 1: **49**.
/// * Phase 2: **26** — the 23 tests that assert only box geometry stop being ignored; see the
///   budget note below for why the number is 26 and not the 19 that was guessed at the gate.
/// * Phase 2.75: **29** — 26 pixel-gated plus T61–T63, the golden regression tests, which are
///   ignored because no golden has been *approved* yet, not because paint is missing.
/// * Phase 3 stop point: **3** — all 26 pixel-gated tests are green and the ignored list is
///   exactly T61–T63, waiting for the human to approve the first render (`ROADMAP.md`).
/// * Phase 3 done: **0**, once those three goldens are signed and the `#[ignore]` attributes go.
///
/// Bumping this constant is a deliberate, reviewable act: it can only go down as work lands, and
/// it can only go up if a new test is added, never because a test started failing.
///
/// # Phase 2 budget, derived rather than estimated
///
/// Of the 49 currently-ignored tests, **26 assert pixel output** and therefore cannot pass before
/// Phase 3, whatever Phase 2 accomplishes:
///
/// ```text
/// T3  T9  T16 T21 T23 T28 T32 T33 T34 T35 T36 T37 T38 T59     (layout+paint cases)
/// T41 T42 T43 T44 T45 T46 T47                                 (paint-only)
/// T48 T49 T50 T51 T60                                         (artifact and perf cases)
/// T61 T62 T63                                                 (golden harness)
/// ```
///
/// That leaves **23** tests that Phase 2 can flip: T1, T2, T4, T10–T15, T17–T20, T22, T24–T27,
/// T29–T31, T39, T40. Phase 2's exit is therefore `passed == 33, ignored == 26, failed == 0`, not
/// the `40 / 19` assumed when the gate was set. The count is derived by scanning the test bodies
/// for pixel helpers (`raster`, `has_ink`, `ppm_bytes`, `png_bytes`, `svg_text`), so it can be
/// re-derived at any time instead of remembered.
///
/// Then T58 was split (review, Phase 2 gate): its layout half — the tree dump, the box count and
/// the timing — runs now as T58, and its pixel half became **T60**, which is ignored. Splitting a
/// test **re-partitions** the list rather than shrinking it: T58 leaves the ignored set and T60
/// joins it, so `ignored` stays 26 while `passed` goes 33 → 34 and the list grows to 60. The
/// review's `34 / 25 / 0` for this gate is off by one for exactly that reason — the same mistake
/// as the `40 / 19` estimate, one level down. Phase 2's exit is therefore `34 / 26 / 0`, and the
/// constants here say so.
///
/// **Phase 2.75** added T61–T63, the golden harness. They are a *different kind* of ignore: the
/// code exists, but the expected image is a contract a human has not signed yet (`ROADMAP.md`),
/// so nothing about them waits on an implementation. They are counted separately from the block
/// above — `passed == 34, ignored == 29, failed == 0` — and they are the only entries in the list
/// that will be un-ignored by *approving an artifact* rather than by writing code.
///
/// **Phase 3 stop point** is therefore `passed == 60, ignored == 3, failed == 0`. The 26
/// pixel-gated tests leave the ignored set; the three goldens stay until the human looks at the
/// PNG. The gate number is `60 / 3 / 0`, not `63 / 0 / 0`, until that signature exists.
///
/// Splitting a mixed test to move the number was considered and rejected: T6 was split because its
/// clause §2.3 is fully satisfiable at the DOM layer, but for T3, T9, T32, T33 and friends the
/// painted line *is* the clause ("a forced break paints two lines", "class beats tag"), and a
/// half-test asserting only `p.height == 40` would assert less than the spec promises.
const EXPECTED_IGNORED: usize = 3;

/// One numbered test found in the `spec_*.rs` files.
struct SpecTest {
    /// The number in the function name (`t06_…` → `6`).
    number: usize,
    /// The full function name, for failure messages.
    name: String,
    /// Whether an `#[ignore]` attribute precedes it.
    ignored: bool,
    /// The ignore attribute text, when present.
    ignore_attribute: String,
}

/// Every `spec_*.rs` file, sorted so failures are reported in a stable order.
fn spec_files() -> Vec<PathBuf> {
    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&tests_dir)
        .expect("tests directory is readable")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("spec_") && name.ends_with(".rs"))
        })
        .collect();
    files.sort();
    files
}

/// Parse the numbered tests out of every spec file.
fn spec_tests() -> Vec<SpecTest> {
    let mut tests = Vec::new();
    for path in spec_files() {
        let text = std::fs::read_to_string(&path).expect("spec file is readable");
        let mut ignore_attribute = String::new();
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("#[ignore") {
                ignore_attribute = rest.to_string();
                continue;
            }
            if let Some(rest) = line.strip_prefix("fn t") {
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                if let Ok(number) = digits.parse::<usize>() {
                    tests.push(SpecTest {
                        number,
                        name: format!("{}.rs::t{}", file_stem(&path), rest),
                        ignored: !ignore_attribute.is_empty(),
                        ignore_attribute: ignore_attribute.clone(),
                    });
                }
            }
            if line.starts_with("fn ") {
                ignore_attribute.clear();
            }
        }
    }
    tests
}

/// File name without its extension, for readable failure messages.
fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("spec")
        .to_string()
}

/// T·meta1 — the numbered list is exactly `T1..=T63`, with no gaps and no duplicates.
///
/// Why: a gap means a clause lost its test; a duplicate means two tests claim one number and
/// the coverage check in `TESTS.md` is lying.
#[test]
fn meta_numbering_is_complete_and_unique() {
    let tests = spec_tests();
    let mut numbers: Vec<usize> = tests.iter().map(|test| test.number).collect();
    numbers.sort_unstable();
    let deduped: Vec<usize> = {
        let mut copy = numbers.clone();
        copy.dedup();
        copy
    };
    assert_eq!(numbers, deduped, "a test number appears twice");
    let expected: Vec<usize> = (1..=EXPECTED_SPEC_TESTS).collect();
    assert_eq!(
        numbers, expected,
        "the numbered test list must be exactly t01..=t{EXPECTED_SPEC_TESTS}"
    );
}

/// T·meta2 — the number of ignored tests matches the phase budget.
///
/// Why: this is the check that makes "never delete a test to make it pass" mechanical. Deleting a
/// test, or silently un-ignoring one, changes this count.
#[test]
fn meta_ignored_count_matches_the_phase() {
    let tests = spec_tests();
    let ignored: Vec<&SpecTest> = tests.iter().filter(|test| test.ignored).collect();
    assert_eq!(
        ignored.len(),
        EXPECTED_IGNORED,
        "ignored count changed; if work landed, lower EXPECTED_IGNORED deliberately. Currently ignored: {:#?}",
        ignored.iter().map(|test| &test.name).collect::<Vec<_>>()
    );
}

/// T·meta4 — `--explain` runs, and reports structure rather than exact wording.
///
/// Why: `--explain` is a debugging aid outside the artifact contract (`SPEC.md` §9.4), scoped at
/// the Phase 2.5 gate to exactly one meta-test. Asserting its text would freeze a format that is
/// expected to grow, so this checks only that it succeeds, says something, and contains the
/// markers that make a report legible. Everything around those markers is free to change.
#[test]
fn meta_explain_runs_and_reports_structure() {
    let fixture = "<style>p.note { color: #123456; width: 50% }</style><p class=\"note\">x</p>";

    let output = run_cli(fixture, &["--explain", "p.note"]);
    assert!(
        output.status.success(),
        "--explain must exit 0 on a matched selector"
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(!text.trim().is_empty(), "--explain must say something");
    assert!(text.contains("element "), "no element header in:\n{text}");
    assert!(
        text.contains("matched rule"),
        "no matched rules in:\n{text}"
    );
    assert!(
        text.contains("dropped"),
        "the unsupported `width: 50%` must be reported as dropped:\n{text}"
    );

    // A selector that matches nothing must say so: an empty report reads as success and is the
    // one failure mode a debugging aid cannot afford.
    let absent = run_cli(fixture, &["--explain", ".absent"]);
    assert!(
        absent.status.success(),
        "a non-matching selector is not an error"
    );
    assert!(
        String::from_utf8_lossy(&absent.stdout).contains("no element matches"),
        "silence is not an answer:\n{}",
        String::from_utf8_lossy(&absent.stdout)
    );
}

/// T·meta5 — `--explain` succeeds on every fixture, not just the one the meta-test uses.
///
/// Why: the report replays the cascade and `debug_assert`s the replay against the style the
/// engine computed, so every fixture explained is a fixture whose cascade ordering is
/// cross-checked. Exit status only — the text is a debugging aid (`SPEC.md` §9.4), and asserting
/// on it would freeze a format that is expected to grow.
#[test]
fn meta_explain_runs_on_every_fixture() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    for (name, selector) in [
        ("hello", ".center"),
        ("list", ".note"),
        ("boxes", ".center"),
    ] {
        let path = fixtures.join(format!("{name}.html"));
        let html = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("fixture {}: {error}", path.display()));
        let output = run_cli(&html, &["--explain", selector]);
        assert!(
            output.status.success(),
            "--explain {selector} failed on {name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// T·meta3 — every `#[ignore]` names the phase that will remove it.
///
/// Why: an untagged ignore is indistinguishable from a test someone quietly gave up on.
#[test]
fn meta_every_ignore_names_its_phase() {
    for test in spec_tests().iter().filter(|test| test.ignored) {
        assert!(
            test.ignore_attribute.contains("phase "),
            "{} is ignored without naming its phase: #[ignore{}",
            test.name,
            test.ignore_attribute
        );
    }
}
