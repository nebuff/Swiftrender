# `PROMPTS.md` — orchestration log

The research output of this repository is not only the renderer; it is also the record of how a
single AI agent was directed by one human. This file logs every human instruction that changed
the trajectory of the work, in order, with a note on what it actually changed. Nothing here is
paraphrased into a flattering summary — if an instruction was vague and the agent guessed, that
is recorded too.

---

## 1. Kickoff (Phase 0)

**Human:** the project brief for *Swiftrender*, establishing the hard constraints (std-only,
no crates without approval, no JavaScript ever, ≤400 lines per file, doc comments everywhere),
the five phases, and the rule that Phase 0 ends with prose only: `SPEC.md` and `TESTS.md`
content in the reply, **no files created until the human says "approved."**

**Effect:** the agent produced an 11-section `SPEC.md` with RFC-2119 keywords and clause IDs,
and a 58-case `TESTS.md` in which every case names its input HTML/CSS, its expected box tree or
pixel assertion, and a one-line rationale — then stopped without touching the filesystem. The
"no files until approved" rule is what forced the spec to be argued in prose, where the human
could reject it cheaply; that is the single most valuable constraint in the brief so far.

**Notable:** the brief did not specify margin collapsing, baseline alignment, bitmap-font
scaling, or `<img>` sizing. The agent chose the simplest defensible option for each and flagged
all four as visible deviations rather than burying them.

## 2. Phase 0 review — approved with notes

**Human:** approved the spec and tests, with two editorial corrections and answers to the three
questions the agent had raised.

- **Q1 resolved:** the `style` attribute stays ignored (now a real clause, §3.4a, rather than a
  dangling cross-reference the agent had invented).
- **Q2 resolved:** percentages and `em` stay unsupported; the `width:50%` drop is to be
  documented behaviour, not a silent no-op.
- **Q3 resolved:** the PNG encoder stays hand-rolled. `flate2` is explicitly **not** approved,
  to preserve the std-only story.
- **Note 1:** the §3.4a ghost reference had to become a real clause.
- **Note 2:** test T24's input was ambiguous — it contained a self-correcting aside that a later
  reader could not resolve. Rewritten to a single unambiguous fixture.

**Effect:** files persisted in the approved order (licenses, README, SPEC, TESTS, QUESTIONS,
ROADMAP, PROMPTS, then crate), with the human's README framing pasted verbatim rather than
rewritten by the agent. Phase 1 begins with the tokenizer and tree builder.

**Process note worth keeping:** both human corrections were *editorial*, not architectural —
the spec's structure survived review. That is evidence that the clause-ID + citation discipline
did its job, and it is the main reason to keep that discipline in Phases 2–4.

## 3. Phase 1 persisted, then reviewed at the gate

**Agent, during persistence:** corrected the T43 fixture. T43 asserted that a red parent remains
visible at `y=38` around a 10px blue child, but under `SPEC.md` §6.4 the parent's auto height
would have been exactly the child's 10px — there was never any parent paint at `y=38` to observe.
The fixture now gives the parent `height:40px`. T43 could not have run yet (it is a Phase 3 test),
so this was found by reading, not by gaming a failure. The correction and its reasoning are
recorded in `TESTS.md` next to T43.

**Human, reviewing Phase 1:** flagged that T6 appeared in the passing set despite asserting
pixels, added the `/src` primitives request, and set a five-item checklist before commit. The
T6 flag was **correct as a process defect and wrong as an accusation of weakening** — the agent
had split T6 at the DOM layer (asserting that `class="a b"` yields two classes and that the
`<style>` text is collected) and deferred the pixel half to a code comment. No assertion was
removed or made vacuous; there was never a running pixel assertion. But a deferral recorded only
as a comment is not a tracked assertion, and `TESTS.md` still promised pixels, so the split was
invisible to anyone reading the contract.

**Fixes applied at the gate:**

* **T59 added** to `TESTS.md` and `tests/spec_paint.rs`: the pixel half of T6 as a real, tracked,
  `#[ignore]`d test. The numbered list is now **59**, not 58, and the count in `README.md` was
  updated to match — the one edit made to the human's README text, flagged here because that file
  is otherwise verbatim human framing.
* **T6 documents its own layer** and points at T59 rather than describing a future extension.
* **`tests/meta.rs` added** (three meta-tests, not numbered spec tests): the numbered list must be
  exactly `t01..=t59` with no gaps or duplicates, the ignored count must equal a named constant
  (49 now, 19 targeted after Phase 2, 0 after Phase 3), and every `#[ignore]` must name its phase.
  This is what makes "never delete a test to make it pass" mechanical instead of a promise.
* **`src/paint/checksum.rs` and `src/paint/deflate.rs` landed early** — CRC-32, Adler-32 and the
  stored-block zlib writer, with published check values (`crc32("123456789") == 0xCBF43926`,
  `adler32("Wikipedia") == 0x11E60398`), an exact byte framing for `"hello"`, and a
  70,000-byte multi-block roundtrip. Requested by the human so that T49's hand-written PNG
  verifier is not the first code ever to run when Phase 3 starts.

**Process note worth keeping:** the one item the agent self-reported honestly (T49's untested
verifier) turned into work, and the one item it did not self-report (T6) was caught by a human
reading the passing list against the phase boundary. The passing list is the artifact to audit,
not the summary around it.

## 4. Phase 1 accepted; three items fixed before Phase 2

**Human:** accepted `de211f0`, asked for the tokenizer fuzz loop, asked for the T5/T57 wording to
be precise about *when* their grounding tests run, and asked to see the second idea the agent had
said it was holding back. Also set a Phase 2 exit criterion of `passed == 40, ignored == 19`.

**Deferral policy (now explicit, governs future phases):** an assertion that depends on two
independently-wrong-able layers (cascade + paint) must be a **tracked test**; an assertion that is
a strict arithmetic consequence of a rule already pinned elsewhere may be recorded as a
**derivation with the pinning tests cited**.

**Work landed:**

* `tests/fuzz_parse.rs` — an in-tree LCG fuzz loop, no crate, fixed seeds, `catch_unwind` so a panic
  is reported with the input that caused it instead of aborting. 16,000 short documents across 8
  seeds plus 50 runs at 64 KiB each, asserting four invariants: the root is always `html`, node
  count stays `<= input + 8` (the anti-quadratic check), total text stays `<= input + 1`, and depth
  never exceeds the §2.10 cap. Two hours of work, no panic found — which is itself the result.
* T5 and T57's layer notes now say their widths *follow from the §6.6 formula independently pinned
  by T2, T27 and T40 (Phase 2/3)*, and state plainly that at Phase 1 that grounding is specified
  but not running. The earlier wording — "derived from tested rules" — overstated it.
* `tests/common/mod.rs` now carries the reason `boxes()` must go through the real binary, marked
  as reviewed and as needing a better argument in a commit message to change.

**The second idea, surfaced as requested:** a CI workflow (GitHub Actions) running `cargo fmt
--check`, `cargo clippy -- -D warnings` and `cargo test` on push. It was withheld from the previous
message only because the phase boundary was the wrong moment to spend attention on it; nothing
about it is confidential.

**Phase 2 exit criterion, corrected by measurement:** the gate's `40 / 19` assumed roughly 30 of the
49 ignored tests would flip. Scanning the test bodies for pixel helpers shows **26 assert pixels**
and cannot pass before Phase 3, leaving **23** that Phase 2 can flip. The honest exit is therefore
`passed == 33, ignored == 26, failed == 0`, recorded with its derivation in `tests/meta.rs`.
Splitting the 12 mixed tests to force a friendlier number was considered and rejected: T6 was
split because its clause §2.3 is fully satisfied at the DOM layer, whereas for T3/T9/T32 the
painted line *is* the clause.

**Process note worth keeping:** the agent's own arithmetic was wrong (it said 24 paint-gated tests;
the awk scan said 26, because T9 and T16 carry raster assertions it had forgotten). The lesson is
the same one as T6, one level down: derive counts from artifacts, never from memory.

## 5. Phase 2 — cascade and layout

**Target, agreed before starting:** `passed == 33, ignored == 26, failed == 0` on the numbered list,
with the 26 being exactly the tests that assert pixels. Delivered as agreed.

**What was built:** `css/` (value types, computed style, stylesheet scanner, cascade, UA sheet) and
`layout/` (box generation with anonymous-block wrapping and `display: none` suppression, font
metrics, block flow, inline line breaking with whitespace collapsing and `text-align`, and the
§8.1 box dump). `--emit tree` now prints the box tree, and `boxes()` in the test harness reads the
real binary's output, as the Phase 1 gate required.

**Four expectations in the approved test list were wrong, and the engine was right in every case.**
Each was corrected in the test *and* in `TESTS.md`, with the reason recorded inline:

* **T31 vs T32 were mutually contradictory.** T31 asked 100 consecutive `a` glyphs to wrap into 98 +
  2, while T32 asks an unbreakable run to overflow on one line. A run of `a`s has no break
  opportunity, so no engine obeying §6.5 ("a word is never split") can satisfy both. T31 now uses
  `"ab "`×60 — 33 words measure exactly 784px, so the corrected fixture is stronger than the original:
  it pins the exactly-full boundary.
* **T4** expected `body > #anon > foo`. §5.3 only wraps where inline and block children mix, and an
  inline box covers its line box (20px), not its glyph cell (16px).
* **T11** expected `x=388`, the centring offset alone, ignoring that offsets are measured from the
  content-box origin at `x=8` — the glyph is at 396.
* **T12** expected the paragraph's margin to be zero, contradicting the UA sheet's `p { margin: 16px
  0 }`. "Not inherited" means it keeps its own margin, not that it has none.

**One deviation had to be written into the spec.** M1 uses **border-box** sizing for explicit
`width`/`height`, recorded as `SPEC.md` §6.2a; the dump's `w`/`h` are border-box dimensions, which
is what every geometry test already assumed. The *reason* was first stated wrongly here and
corrected at review: T20 on its own is model-independent (`width: auto` gives a 784px border box
either way), so naming T20 as half of a contradiction was inaccurate. The real pressure is T21 read
through a single dump with T20 — under content-box, `height: 20px` + 2px borders is 24px tall and
T21's `h == 20` fails; report content-box dims instead and T20's `x == 8` fails. No single
reporting convention satisfies both, so border-box is forced *given the tests as approved*.
Border-box is also independently the nicer model — but that is a preference, not the justification.

**A tool caught what the agent's memory did not.** After the first full run, the agent reported
"only three failing tests"; that was an artifact of `cargo` stopping at the first failing test
binary. `--no-fail-fast` revealed 31. The same run also exposed that the agent's hand-written
`boxes()` parser silently defaulted `x` to 0 — the line grammar puts the x *value* immediately after
the `  x=` separator, so tokenising the remainder dropped it, and only assertions that happened to
expect 0 could pass. Both are the same lesson as T6 and the 24-vs-26 count: **read the tool's real
output, not the summary you remember writing.**

**Process note worth keeping:** three files crossed the 400-line budget during the phase
(`css/style.rs`, `layout/inline.rs`, `tests/common/mod.rs`). They were split *before* adding more,
as instructed at the Phase 1 gate — and the first split attempt itself was wrong: it cut at a marker
that appeared earlier in the file than the agent assumed, so the "split" moved half of the module
into the wrong file. The fix was to reconstruct from the preserved parts and split by *parsed item
boundaries* rather than by line numbers remembered from an earlier `grep`.

## 6. Phase 2 review — the gate, resolved

**Check 1 — §6.2a (border-box).** The review could not derive the contradiction from T20/T21 as
written, and it was right not to: the note blamed T20, which is model-independent. The arithmetic
that does hold was added to `SPEC.md` §6.2a and to the paragraph above: content-box + a border-box
dump fails T21, content-box + a content-box dump fails T20, therefore border-box. "Forced" stands;
the *derivation* was wrong and is now shown rather than asserted. The review's alternative framing
— "proactively specified" — was declined for that reason, but its observation that border-box is
a sensible M1 default is recorded, because a future phase might wonder why we deviated at all.

**Check 2 — the four corrections.** Both files carry all four: `TESTS.md` (T31 fixture note, T4/T11/
T12 reason lines) and the test bodies in `spec_parse.rs`, `spec_cascade.rs`, `spec_inline.rs`. Four
hunks, as required. What the check could not see, because the tree was already clean by then: the
Phase 2 work was committed by the repository owner as `33b9c16` **with the message "Phase 1"**. The
code is fine; the audit trail is not. Rewriting a commit the owner made is not the agent's call, so
the mapping is recorded here instead: **`33b9c16` contains Phase 2**, and `de211f0`/`cf73f20` are
the Phase 1 artifacts. The commit message written at the gate (`Phase 2: cascade, block/inline
layout…`) describes the same contents and can be used for an amend if the owner wants one.

**Check 3 — T58 split.** Taken. T58 now asserts the layout property it exists to protect — 5,000
divs in a tree dump under 2 s (measured 0.21 s debug / 0.02 s release), byte-identical twice, and
**all 5,000 boxes present** — and T60 keeps the PNG determinism for Phase 3. The box count was added
because a quadratic layout may drop boxes silently rather than run slow, and unlike timing it does
not depend on machine load.

The review predicted this gate would become `34 / 25 / 0`. It becomes **`34 / 26 / 0`**: splitting a
test re-partitions the list instead of shrinking it, so T58 leaves the ignored set and T60 joins it,
and the numbered list grows to 60. That is the third arithmetic correction in two gates (`40 / 19`,
then `24` vs `26`, now `25` vs `26`), all with the same root cause — reasoning about counts from
memory instead of from the artifacts. `tests/meta.rs` still derives its budget from a scan.

**Scheduled for Phase 2.5:** `--explain <selector>`, a cascade debugging aid showing matched rules
in specificity order with dropped declarations and why. Scoped by the review to one meta-test only;
it is a debugging tool, not part of the artifact's contract.

## 7. Tools that lie (append-only)

Every entry is a signal that read as green while not meaning what the summary claimed. The fixes
are local; the failure mode is general, which is why they are written down.

1. **`cargo test` without `--no-fail-fast`** — cargo stops at the first failing test *binary*, so
   a run with 31 failures reported "only three" in Phase 2. Fix: always pass `--no-fail-fast`, and
   read the `test result:` lines rather than the exit code alone.
2. **Piping cargo output through a filter** — `cmd | grep …` then `echo $?` reports the *filter's*
   status. That happened in the Phase 2 gate report: "TEST=0" was `grep`, and the suite's own
   status had never been captured at all. Fix: `set -o pipefail`, or redirect to a file and check
   the command's own status.
3. **A test that asserts half a clause and defers the rest to a comment** — reads as fully green
   (T6, Phase 1; the same pattern was then found twice more in T5 and T57). Fix: if a clause has N
   assertions, the test has N assertions, and deferrals become tracked tests (the T59 pattern).

## 8. Count rule

Any number about the test suite must be produced by a scan of the files, never by arithmetic on a
previous number. Four consecutive slips make the case: `40/19` and `34/25/0` from the reviewer,
`24`-vs-`26` and `25`-vs-`26` from the agent — every one of them reasoning in prose instead of
counting artifacts. `tests/meta.rs` derives its budget by scanning `spec_*.rs`; prose must do the
same before it states a number.

## 9. Phase 2.5 — `--explain`

Built as scheduled, in one module (`src/css/explain.rs`) plus a CLI flag, with the scope the gate
set: small, off the critical path, one meta-test.

**What it does.** `swiftrender page.html --explain p.note` walks the document in document order
carrying each element's computed style, and for every matching element prints the rules that
matched in **cascade order**, each declaration's outcome (`applied`, or `dropped` with the reason),
and the resulting computed style. An unsupported selector is a usage error (exit `2`); a selector
that matches nothing says so explicitly rather than printing an empty report, because silence
reads as success and that is the one failure mode a debugging aid cannot afford.

**One ordering, two consumers.** `parse::matching_rules` was extracted so `apply_sheet` and the
explainer share the single definition of cascade order. Re-implementing the sort inside the
explainer would have made a debugging tool that can disagree with the renderer about the thing it
explains. The report also replays the cascade into a scratch style and `debug_assert_eq!`s it
against the style `style_for` actually computed, so drift between the two is a test failure rather
than a plausible-looking lie.

**Format deliberately unpinned.** One meta-test (`meta_explain_runs_and_reports_structure`) asserts
exit status, non-empty output, the structural markers (`element `, `matched rule`, `dropped`) and
the no-match message. Nothing asserts exact wording, and no numbered test references `--explain`:
it is a tool, not an artifact, and freezing its text would make every future improvement a
documentation chore.

**Why `33b9c16` was not amended.** The gate asked for its "Phase 1" message to be corrected, or the
decision documented. It is documented: the commit is reachable from `origin/main`, so rewording it
means a force-push of shared history — destructive, and not something to do on the strength of a
review note. The reflog additionally shows a reset followed by a re-commit (`4da0c0d`, identical
tree) after the gate commit was made, so the branch has already been rewritten once locally; adding
another rewrite would make the log harder to trust, not easier. The mapping in §6 stands as the
record: `33b9c16` contains Phase 2.

## 10. Phase 2.75 — splits and the golden harness

Three commits, in the order the gate asked for: the definition of done, the file splits, then the
harness.

**The definition of done moved into the repo** (`ROADMAP.md`). A gate that exists only in a review
message is not a gate, and Phase 3's had already survived one round trip as prose. Items 3 and 4 —
a human opens the image, and an external decoder parses it — are the two no test can replace: T49
validates PNG bytes with the *in-tree* decoder, which shares the writer's assumptions and so cannot
catch a bug the two have in common.

**Both flagged files were split before paint, not during it.** `tree.rs` (391) became `dom.rs`
(the node model every later layer reads, 150) and `tree.rs` (construction, 253); `parse.rs` (372)
became `rule.rs` (selector and rule types plus the single definition of cascade order, 148) and
`parse.rs` (the scanner, 238). The longest source file is now 360. Two details worth keeping:

* The split was done by **parsed item boundaries with an assertion that each marker appears exactly
  once** — the Phase 2 lesson about a split that cut at the wrong `grep` hit, applied *before*
  repeating it rather than after.
* `parse.rs` re-exports the moved types (`pub use super::rule::{…}`) so `css::parse::Selector`
  keeps naming the same type. A split that forces every call site to change is a rename wearing a
  split's clothes; this one changes no public path.

**The golden harness is staged, not blessed.** `tests/spec_golden.rs` compares a render against
`fixtures/<name>.expected.ppm` at 200×150, and **no golden exists**. The comparison reads the file
at runtime rather than through `include_bytes!` — a compile-time include would make this commit
uncompilable, since the files cannot exist before a human approves a render. The three tests are
`#[ignore]`d with the missing file named in the reason, so un-ignoring one early fails with the
policy, not a bare file-not-found. Goldens are PPM because byte comparison needs no decoder, which
keeps the zero-dependency premise intact.

**Arithmetic, again.** The gate wrote Phase 3's exit as `60/0/0`. Adding T61–T63 makes it `63/0/0`;
`ignored` is now 29 = the 26 pixel-gated tests + 3 goldens, so Phase 2.75 sits at `34/29/0`. That is
the fifth count in this project derived from prose and the fifth that needed the scan to correct it,
which is exactly why §8 exists and why `ROADMAP.md` states the number with its composition.
