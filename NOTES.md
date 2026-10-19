# `NOTES.md` — Swiftrender

Research notes, started during Phase 3 rather than deferred to Phase 4. `PROMPTS.md` is the
chronological record of instructions; this file is the synthesis of what the paint phase taught.
The paint implementation is `b0a3c26`; the gate record is `31363a3`.

## 1. All tests green, image visibly wrong: the auto-height bug

Phase 2 computed a block's `auto` height as the distance from its content top to the bottom margin
edge of its last child, then stored that number as the **border-box** height. That is the right
content height and the wrong border-box height: padding and border were being eaten out of the
content instead of added around it.

The geometry tests missed it because each one isolated a different factor. T2 and T26 had no
padding or border. T20 had padding but asserted only `x` and `width`. T21 and T29 had borders or an
explicit height. No numbered test combined *auto height + padding + border + a text line*, so the
composition was never exercised. All the individual clauses were satisfied.

The first full-feature render made it obvious. In `fixtures/hello.html`, the `.note` panel had
`padding: 8px`, `border: 1px solid`, a 20px line box, and auto height. It should have been 38px
tall; the PNG showed a 20px grey band with the text painting below it. The fix in
`src/layout/block.rs` is one line:

```text
auto border-box height = content height + padding.vertical() + border.vertical()
```

The explicit-height path is unchanged, because §6.2a already defines explicit `width`/`height` as
border-box dimensions. A unit test, `auto_height_includes_padding_and_border`, now pins the
composition.

The lesson is not "write more tests". It is that **a green suite can certify the clauses while
missing their composition**. Pixel output is the integration test for the layout model, and the
human eyeball check is load-bearing precisely because no numbered clause owns "does this look like
a page?"

## 2. Ten approved fixtures were wrong

Across the Phase 2 and Phase 3 gates, ten fixtures had to be corrected. None was corrected to make
a failure disappear; each contradicted the normative spec or arithmetic already pinned elsewhere.

**Phase 2 — four fixtures.** T31 asked 100 consecutive `a` glyphs to wrap while T32 required an
unbreakable run to overflow; the two clauses are mutually exclusive. T4 expected an anonymous block
around all-inline content, which §5.3 does not require. T11 measured the centring offset from the
page origin instead of the content-box origin. T12 asserted that a non-inherited margin becomes
zero, contradicting the UA sheet's `p { margin: 16px 0 }`.

**Phase 3 — six fixtures.** T35, T41, T43, T49 and T50 used the `background` shorthand, which
§3.6 does not support; they now use `background-color`. T43 also used the unsupported `div div`
descendant selector, which §3.2 drops whole; the child is now `.child`, which keeps the
child-over-parent paint assertion and moves the specificity question into the cascade. T37 and T38
sampled `y=8`, above the paragraph's border box; the placeholders are at `(16,24)` and `(8,29)`.
T50's prose note had double-escaped `a&amp;amp;b`; the contract is one escape, `a&b` → `a&amp;b`.
T49 shares T41's fixture and is counted with it. T43 had also been corrected in Phase 1 for a
missing parent height, so that fixture was touched twice; the count of ten counts it once.

**Provenance, checked rather than assumed.** The original `background:` and `div div` lines were
added in `de211f0`, the first commit that persisted `TESTS.md` (the initial commit `a6ecddb` did
not contain the file), and they stayed unchanged through `6a03732`. The Phase 3 commit is the only
commit that changes them. So the fixtures were **wrong as approved**, not altered later to match
the engine. That distinction is the whole reason the protocol forbids deleting a test to make it
pass.

What this says about Phase 0 is uncomfortable but useful. The tests were written before any
implementation existed, so they were a design document, not an oracle. They encoded guesses about
unsupported CSS syntax, selector scope, and coordinates — guesses that were plausible in prose and
wrong against the spec. The numbered list is still the right contract, but a contract needs an
executable check and a gate that re-derives each expectation. Inline `*Expectation corrected at
the … gate*` notes are the compromise that makes the correction auditable; the alternative,
silently editing a fixture until it passes, is indistinguishable from weakening the suite.

The corrections also have a shape. The T31 fix made the test stronger by pinning the exactly-full
line-width boundary. The T43 fix preserved the child-over-parent assertion while removing two
incidental CSS dependencies. The T37/T38 fixes named the derivation from the already-pinned
paragraph position. That is the standard a correction has to meet: keep or strengthen the clause,
and show the arithmetic.

## 3. Tools that lie, named

Four tool failures have now cost real debugging time. They share a rule: **a primitive that cannot
fail cannot be trusted.** If a tool has no way to report "I could not do this", its success looks
exactly like a pass.

1. **`--no-fail-fast`.** `cargo test` stopped at the first failing test binary, so the agent
   reported three failures when there were thirty-one. The summary was true for the subset that
   ran and false for the suite. Fix: run `cargo test --no-fail-fast`, preserve the command's exit
   status, and derive counts from a scan of the test files rather than from a remembered number.

2. **`grep`-as-cargo.** A `grep` hit was treated as an authoritative answer, and the first file
   split cut at a marker that appeared earlier in the file than assumed; half the module moved into
   the wrong place. Fix: split by parsed item boundaries with an assertion that each marker
   appears exactly once, and let `tests/meta.rs` produce the numbered count. `grep` can find a
   candidate; it cannot certify a count or a boundary.

3. **`write_file`-as-edit.** A write tool reported "Created file successfully" and destroyed
   `ROADMAP.md`, because "created" and "replaced" are the same sentence to the caller. Fix:
   `git ls-files --error-unmatch <path>` first, read the current content before writing, check
   `git status --short` for ` M` versus `??`, and inspect `git diff --stat` before committing. A
   deletion count that cannot be explained is a bug, not a refactor.

4. **The comparer's silent match.** A comparison layer that can only answer "same" or "different"
   with no error case makes every golden pass on a wrong render. The first diff message also
   divided by a hard-coded 200px width instead of the image's own pixel count, so a 2×2 test
   reported "1 of 200". Fix: `compare_ppm` returns `Result<Comparison, String>`; a canvas mismatch
   is an error rather than a silent zip to the shorter image; `parse_ppm` rejects bad headers and
   truncated rasters; synthetic images pin same, different, mismatch, and parser rejection. The
   PPM parser was hardened in the same direction.

The pattern is cumulative: T6's deferred pixel assertion became a tracked test (T59); the
comparer's permissive match became a `Result`; the PPM parser's permissive header became a
`Result`; and the Phase 3 gate added the provenance check for fixture corrections. Each change
makes a silent success harder.

### Validation note: `pngcheck`

Phase 3 DoD item 4 was satisfied by `file` (identifies the output as PNG, 800×600, 8-bit RGB,
non-interlaced) and an independent Pillow decode (reads it as 800×600 RGB). `pngcheck -v` was not
available in the build environment. If the project is installed elsewhere, running `pngcheck` on
`hello.png` should be treated as the definitive external validation; the in-tree decoder shares
the writer's assumptions and cannot catch a shared bug.

## Open items

- The three goldens (`fixtures/{hello,list,boxes}.expected.ppm`) still require a human render and
  signature; T61–T63 remain `#[ignore]`d until then. Only the human should generate them.
- Phase 3.5 is scheduled after that gate: extract the shared paint visitor for raster and SVG,
  behavior-neutral, with the signed goldens as the regression check.
- Phase 4 is the broader reflection; this file is the Phase 3 material it will draw on.
