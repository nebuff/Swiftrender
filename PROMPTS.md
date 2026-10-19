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
