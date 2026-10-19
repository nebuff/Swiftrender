//! Meta-tests: invariants about the test suite itself, not about the renderer.
//!
//! `TESTS.md` is the contract, and two of its rules are about bookkeeping rather than
//! rendering: a test may only be deleted if its clause was removed from `SPEC.md` first, and an
//! `#[ignore]`d test may only disappear when the phase named in its attribute lands. Checking
//! those by hand with `grep` is a habit; this file turns the habit into a check that fails in CI.
//!
//! These are deliberately **not** numbered spec tests: the numbered list is the contract with
//! `SPEC.md`, and meta-tests would muddy the count.

use std::path::{Path, PathBuf};

/// Number of numbered tests defined by `TESTS.md` (T59 was added at the Phase 1 gate).
const EXPECTED_SPEC_TESTS: usize = 59;

/// How many numbered tests are `#[ignore]`d at this phase.
///
/// * Phase 1: **49** (T1–T4, T9–T51, T58, T59 wait on layout and paint).
/// * Phase 2 target: **19** — the cascade and layout tests begin running.
/// * Phase 3 target: **0**.
///
/// Bumping this constant is a deliberate, reviewable act: it can only go down as work lands, and
/// it can only go up if a new test is added, never because a test started failing.
const EXPECTED_IGNORED: usize = 49;

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

/// T·meta1 — the numbered list is exactly `T1..=T59`, with no gaps and no duplicates.
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
