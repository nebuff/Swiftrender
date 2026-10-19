# `TESTS.md` — Milestone 1 test list

**Status: APPROVED (Phase 0 gate cleared).** This list is the contract: a test may only be
deleted if its clause was removed from `SPEC.md` first.

**Conventions.** Canvas 800×600 unless stated. Default body content box is therefore
`x=8, y=8, w=784`. Default font 16px ⇒ advance 8, cell 8×16, line box 20. All values below are
**snapped integers** (§6.7). Harness: `tests/spec_*.rs` with helpers `parse(html) -> Document`,
`tree(html) -> String`, `boxes(html) -> Vec<String>`, `pixels(html, w, h) -> Raster`, and
`png_bytes(html)`; each case is one `#[test] fn tNN_…` whose failure message prints the full box
tree. Rationale text is stored as the test's doc comment.

Phase status note: tests whose clause needs layout or paint are marked `#[ignore]` with the
phase that will un-ignore them; the input, expected value, and rationale are complete either
way, so removing the attribute is the only change Phase 2/3 makes to them.

### A. Tokenizer & tree (T1–T8)

**T1 · implicit html/body** — In: `<div></div>` — Out: tree `html > body > div`,
`div` at `(8,8,784,0)`, `html h=16`. Why: §2.7 wrapper synthesis and §6.4 auto height including
the body's bottom margin.

**T2 · single paragraph geometry** — In: `<p>hi</p>` — Out: `html h=68`,
`body (8,8,784,52)`, `p (8,24,784,20)`, text run width 16. Why: pins UA margins, line box
height, and auto-height chain in one case.

**T3 · void br starts a new line** — In: `<p>a<br>b</p>` — Out: `p h=40`; ink present in both
the y≈26 and y≈46 bands. Why: §5.5 forced break without a closing tag.

**T4 · unknown element is inline** — In: `<foo>x</foo>` — Out: `body > foo(inline)` at
`(8,8,8,20)`, `body h=20`, no panic, no warning. Why: §2.11 forward compatibility for HTML we do
not model.

*Expectation corrected at the Phase 2 gate:* the original said `body > #anon > foo`. §5.3 wraps
inline runs only where inline and block children **mix**, so an all-inline body needs no wrapper,
and an inline box's rectangle covers the line box it occupies (20px), not the 16px glyph cell.

**T5 · entity decoding** — In: `<p>&amp;&lt;&#65;</p>` — Out: text `"&<A"`, run width 24. Why:
§2.4 numeric and named entities change measured width.

*Layer note (added at the Phase 1 gate, reworded at review):* the text half is §2.4 and runs from
Phase 1. The `24` follows from the §6.6 advance formula, independently pinned by T2, T27 and T40
(Phase 2/3): if any of those fail, the whole layout layer is broken and this assertion is moot.
At Phase 1 that grounding is *specified but not yet running*, which is why this is recorded as a
derivation with its dependencies named rather than as a passing assertion.

**T6 · multiple classes** — In: `<style>.b{color:#00ff00}</style><div class="a b">x</div>` —
Out: text pixels `#00ff00`. Why: §2.3/§3.2 class attribute splits on whitespace and any single
class matches.

*Layer note (added at the Phase 1 gate):* this fixture spans two clauses in two phases. §2.3 is
verified at the DOM layer by T6 itself, which runs from Phase 1. The §3.2/§7.6 half — the pixel
assertion quoted above — is verified by **T59**, because a test cannot assert pixels in a phase
that has no paintable output. T6 was split at the Phase 1 gate; the split was initially recorded
only as a code comment, which is not a tracked assertion, and T59 exists to close that gap.

**T7 · comment and doctype are ignored** — In: `<!-- c --><!DOCTYPE html><div></div>` — Out:
exactly one `div` box, no text nodes. Why: §2.1 non-element tokens must not create boxes.

**T8 · nesting cap** — In: 520 nested `<div>`s — Out: no panic, tree depth ≤ 512, one stderr
warning under `--verbose`. Why: §2.10 recursion guard must be observable, not silent.

### B. Cascade (T9–T17)

**T9 · class beats tag** — In: `div{color:#ff0000} .k{color:#00ff00}` on
`<div class=k>x</div>` — Out: green ink, zero red pixels. Why: §3.3 specificity ordering.

**T10 · document order breaks ties** — In: `p{color:#ff0000}` then `p{color:#00ff00}` — Out:
green ink. Why: §3.3 last-wins at equal specificity.

**T11 · inherited properties** — In: `div{text-align:center;color:#0000ff}` with
`<div><p>x</p></div>` — Out: `p` glyph starts at `x=396` and is blue; `p`'s own `text-align`
computed value is `center`. Why: §3.7 inheritance through an element that never declares the
property.

*Expectation corrected at the Phase 2 gate:* `388` is the centring offset alone, but offsets are
measured inside the content box, which starts at `x=8`, so the glyph lands at
`8 + round((784 − 8)/2) = 396`. T33's `388` is a different fixture with a different offset and was
already correct.

**T12 · non-inherited margin** — In: `div{margin:10px}` with `<div><p>x</p></div>` — Out: `p`
box top `= 8+10+16 = 34`, `p margin=16,0,16,0`. Why: §3.7 must not leak box-model properties to
children.

*Expectation corrected at the Phase 2 gate:* the original asserted `margin=0`, which contradicts
the UA sheet's `p { margin: 16px 0 }`. "Not inherited" means the paragraph keeps its **own**
margin — the one the UA sheet gives it — not that it has none.

**T13 · selector list** — In: `h1, p { margin: 0 }` — Out: both boxes report
`margin=0,0,0,0`. Why: §3.2 comma lists expand to independent rules.

**T14 · unsupported selectors dropped whole** — In: `#x{color:#ff0000} *{color:#ff0000}
p{color:#0000ff}` on `<p id=x>a</p>` — Out: blue ink only (not red, not purple/black). Why:
§3.2 must drop the rule, not partially apply it.

**T15 · !important ignored** — In: `p{color:#ff0000!important}` then `p{color:#00ff00}` — Out:
green ink, no stderr error. Why: §3.5 parses but does not prioritize.

**T16 · style attribute ignored** — In: `<p style="color:#ff0000">x</p>` — Out: black ink.
Why: §3.4a declared deviation (Q1).

**T17 · unsupported value dropped** — In: `div{width:50%; height:2em}` — Out: `div` width 784,
height auto, two verbose warnings. Why: §3.8 rejection must be per-declaration and non-fatal.

### C. Box model (T18–T24)

**T18 · auto width fills containing block** — In: `<div></div>` — Out: `w=784`. Why: §6.2
baseline.

**T19 · explicit width** — In: `div{width:100px}` — Out: `(8,8,100,0)`. Why: §6.2 explicit
width does not stretch.

**T20 · padding narrows content** — In: `div{padding:10px}` — Out: box `w=784`, content width
764, content x = 18; left padding filled only if a background is set. Why: §6.2/§6.3 subtract
padding from the containing block.

**T21 · border geometry and paint** — In: `div{border:2px solid #ff0000}` — Out: box `w=784`,
content `x=10`, content width 780; pixels at `(8,8)` and `(9,8)` are red, `(10,10)` is white.
Why: §7.4 border occupies the band between padding and border box.

**T22 · asymmetric margins** — In: `div{margin:5px 10px}` — Out: `(18,13,764,…)`. Why: §6.3
horizontal vs vertical margin bookkeeping.

**T23 · display none removes subtree** — In: `.g{display:none}` on
`<div class=g>gone</div><p>z</p>` — Out: no `div` box, `p` top `= 24`, only `z` ink exists.
Why: §5.1 no box and no paint.

**T24 · explicit height with overflow** — In: `<style>div{height:10px}</style>`
`<div><p>a</p></div>` — Out: `div h=10`; `p` top = 8 + 16 = 24 (margin) is outside the div's
10px box; ink appears at y ≥ 24, i.e. below the box bottom (y=18). Why: §6.4/§7.7 overflow is
visible, not clipped.

### D. Block layout (T25–T30)

**T25 · sibling stacking** — In: two empty `<div>`s each `height:20px` — Out: second at
`y=28`. Why: §6.3 sequential flow with zero margins.

**T26 · margins do not collapse** — In: two `<p>`s — Out: `p1 (8,24,…,20)`,
`p2 (8,76,…,20)`, gap 32. Why: §6.3 explicit anti-CSS simplification, easy to regress.

**T27 · h1 metrics** — In: `<h1>x</h1>` — Out: `font-size 32`, advance 16, line box 40, box
`h=40` at `y=24`, one glyph cell 16×32. Why: §4 UA font-size and §6.6 advance formula at a
non-default size.

**T28 · list indentation and markers** — In: `<ul><li>a</li><li>b</li></ul>` — Out: `ul`
content x=48; `li` boxes `w=744`, `li1 y=24 h=20`, `li2 y=44`; bullets ink at `(32,26)` and
`(32,46)`. Why: §4/§6.8 marker placement relative to the li content box.

**T29 · fixed-height parent keeps its height** — In: `div{height:100px}` containing one 20px
line — Out: `div h=100`, text top unchanged. Why: §6.4 auto-vs-explicit height resolution.

**T30 · nested auto width chain** — In: `div{padding:5px}` inside `body` containing an inner
`<div>` — Out: outer content width 774, inner width 774, inner x = 13. Why: §6.2 recursion
through padding.

### E. Inline formatting (T31–T40)

**T31 · greedy wrapping** — In: `<p>` + `"ab "`×60 — Out: two line fragments, widths 784 and
640, `p h=40`. Why: §6.5 breaks exactly when the next item would exceed W.

*Fixture corrected at the Phase 2 gate:* as originally written (100 consecutive `a` glyphs) this
test contradicted **T32**, which requires an unbreakable run to overflow on a single line. A run of
100 `a`s contains no break opportunity at all, so no engine satisfying §6.5 ("a word is never
split") can produce two lines from it. The fixture now uses whitespace-separated words, and is
stronger than the original: 33 words measure `33 × 16 + 32 × 8 = 784`, exactly the content width,
so it pins the exactly-full boundary as well as the ordinary break.

**T32 · unbreakable word overflows** — In: `<p>` + `a`×200 — Out: one line, `p h=20`, ink
extends past x=792, no error. Why: §6.5 no mid-word breaks, no crash.

**T33 · center alignment** — In: `p{text-align:center}` with `<p>abc</p>` — Out: first ink x =
388. Why: §6.5 offset formula rounds once.

**T34 · right alignment** — In: `p{text-align:right}` with `<p>abc</p>` — Out: first ink x =
768, last ink ends at 791. Why: §6.5 right offset is `W − used`.

**T35 · inline fragment background** — In: `.s{background:#00ff00}` on
`<p><span class=s>ab</span></p>` — Out: green rect exactly `(8,24,16,20)`; white at `(24,24)`.
Why: §7.1 inline boxes paint the full line box height, not just the glyph cell.

**T36 · consecutive breaks** — In: `<p>a<br><br>b</p>` — Out: `p h=60`, ink in lines 1 and 3,
line 2 blank. Why: §5.5 empty line boxes still occupy height.

**T37 · image placeholder default** — In: `<p>x<img></p>` — Out: img box `150×50`, `p h=50`,
img x = `8+8=16`. Why: §5.6 intrinsic size and §6.5 line height driven by an atomic inline.

**T38 · image explicit size** — In: `img{width:20px;height:10px}` with `<p><img></p>` — Out:
img `20×10`, `p h=20` (line-height dominates), placeholder `#c0c0c0` at `(8,8)`. Why: §5.6
override plus §6.5 max-of-items rule.

**T39 · anonymous block boxes** — In: `<div>text<p>para</p>tail</div>` — Out: `div` has
exactly 3 children: `#anon(h=20)`, `p(h=20, margins 16)`, `#anon(h=20)`; `div h=92`; order
preserved. Why: §5.3 the hardest structural rule in M1.

**T40 · whitespace collapsing** — In: `<p>  a   b  </p>` — Out: text run `"a b"`, width 24,
first ink at x=8. Why: §6.5 leading/trailing spaces must not shift alignment.

### F. Painting (T41–T47)

**T41 · background fill** — In: `div{background:#123456}` — Out: pixel
`(8,8) = (0x12,0x34,0x56)`; pixel `(791,599)` still white. Why: §7.1/§7.3 background covers
content+padding+border only.

**T42 · border defaults to currentColor** — In: `div{color:#ff0000;border:1px solid}` — Out:
border pixels red, text absent. Why: §7.4 `border-color` initial value.

**T43 · child paints over parent** — In: `div{background:#ff0000;height:40px}` with nested
`div div{background:#0000ff;height:10px}` — Out: `(8,8)` blue, `(8,38)` red. Why: §7.1
tree-order compositing with opaque colours. (Fixture corrected in Phase 1: the original
fixture omitted the parent's height, and under §6.4 the parent's auto height would have been
exactly the child's 10px, leaving no parent paint to observe at y=38.)

**T44 · glyph ink is inside the cell** — In: `<p>H</p>` — Out: ≥1 black pixel in `(8,26,8,16)`,
all pixels at y=25 white. Why: §6.5 centred cell and §6.6 cell extent.

**T45 · text colour applied to ink only** — In: `p{color:#ff0000}` with `<p>H</p>` — Out:
every non-white pixel has `r=255,g=0,b=0`; no grey anti-aliasing pixels. Why: §7.6 1-bit
masks, no AA.

**T46 · empty document is white** — In: `<html></html>` — Out: 800×600 PPM whose pixel payload
is all `0xff`. Why: §7.2 canvas initialisation.

**T47 · marker ink present** — In: `<ul><li>a</li></ul>` — Out: ≥1 black pixel in
`(32,26,8,16)`, none at `(40,26)`. Why: §6.8 marker box size/offset.

**T59 · class selector reaches the raster** — In: `<style>.b{color:#00ff00}</style>`
`<div class="a b">x</div>` — Out: the `div`'s computed colour is `#00ff00` and every non-white
pixel is `#00ff00`. Why: §3.2/§7.6 the pixel half of T6, added at the Phase 1 gate so that the
deferral is a tracked assertion rather than a comment. Requires both the cascade (Phase 2) and
paint (Phase 3).

### G. Output, CLI, pipeline (T48–T54)

**T48 · PPM header bytes** — In: `--emit ppm --size 800x600` — Out: file begins exactly
`P6\n800 600\n255\n`, total length `33 + 800*600*3`. Why: §8.2 exact byte layout.

**T49 · PNG correctness and PPM equivalence** — In: T41 fixture — Out: signature
`89 50 4E 47 0D 0A 1A 0A`; `IHDR` w/h match, bit depth 8, colour type 2, interlace 0; every
chunk CRC-32 recomputed by the test matches; after stripping filter bytes, raster equals the
PPM payload byte-for-byte. Why: §8.3 proves our hand-rolled zlib/CRC/Adler is right.

**T50 · SVG structure and escaping** — In: `div{background:#123456}` with `<p>a&amp;b</p>` —
Out: `<svg … width="800" height="600"`, one `<rect … fill="#123456"/>`,
`<text …>a&amp;amp;b</text>` escaped correctly, well-formed XML. Why: §8.4 structural
contract.

**T51 · determinism** — In: run the T39 fixture twice, same args — Out: outputs byte-identical,
no timestamps/paths inside. Why: §8.5 reproducible artifacts for a research repo.

**T52 · exit codes** — In: no args / missing file / `--emit bogus` — Out: `2`, `1`, `2`;
nothing written on failure. Why: §9.2 CI-usable failure semantics.

**T53 · file length cap** — In: scan `src/**/*.rs` — Out: every file ≤ 400 lines. Why: §10.1
constraint is machine-checked, not aspirational.

**T54 · module headers** — In: scan `src/**/*.rs` — Out: each file's first line starts with
`//!`. Why: §10.2 enforced where a test can enforce it.

### H. Robustness (T55–T58, T60)

**T55 · mismatched end tags** — In: `<div><p>x</div>y` — Out: deterministic tree, `y` text
present after the div, no panic (§2.6). Why: real-world HTML is broken and we must not crash.

**T56 · text before html** — In: `hello<p>x` — Out: `#anon("hello")` then `p`, both inside
body, correct order (§2.7). Why: implicit body must capture leading text.

**T57 · unknown entity stays literal** — In: `<p>&bogus;</p>` — Out: text `"&bogus;"`, width
56. Why: §2.4 must not silently drop characters.

*Layer note (added at the Phase 1 gate, reworded at review):* as with T5, the literal-text half is
§2.4 and runs from Phase 1; `56` is `7 × 8`, following from the §6.6 advance formula independently
pinned by T2, T27 and T40 (Phase 2/3). If any of those fail, the layout layer is broken and this
assertion is moot; at Phase 1 that grounding is specified but not yet running.

**T58 · bulk input guard** — In: 5,000 sibling `<div>`s — Out: a `--emit tree` dump in under
2 s, byte-identical across runs, containing all 5,000 divs (5,002 boxes total), no recursion limit
hit. Why: early performance guard against accidentally quadratic layout.

*Split at the Phase 2 gate, on the T6 precedent.* As approved, T58 asserted pixel determinism,
which cannot run before paint exists — and so the anti-quadratic guard was absent from exactly the
layer it protects. The layout half now runs in Phase 2; the paint half is T60. The 2 s budget
keeps its original number, measured at 0.21 s debug / 0.02 s release on this fixture. The box
count is not decoration: a quadratic layout can also *drop* boxes silently, and unlike the timing
it does not depend on how loaded the machine is.

**T60 · bulk pixel determinism** — In: 5,000 sibling `<div>`s — Out: a full render in under 2 s
producing byte-identical PNGs twice. Why: deterministic layout does not imply deterministic paint
(iteration order, uninitialised buffers), so the original T58 assertion is repeated at the layer
it actually describes rather than inherited. Ignores: needs paint.

### I. Golden regression (T61–T63)

These three compare a render against an approved PPM at 200×150. They differ from every other entry
in what makes them pass: not new code, but a **human signing an image** (`ROADMAP.md`, "Golden
files: who signs them"). They are `#[ignore]`d until then, and the ignore reason names the missing
file so an early un-ignore fails with an explanation instead of a file-not-found.

**T61 · golden hello** — In: `fixtures/hello.html` — Out: pixels identical to
`fixtures/hello.expected.ppm`. Why: the most-used fixture is the one whose regressions hurt most.

**T62 · golden list** — In: `fixtures/list.html` (nested `ul`/`li`, one classed item) — Out: pixels
identical to `fixtures/list.expected.ppm`. Why: list indentation plus per-item text placement spans
two layers, and no single-value assertion pins the combination.

**T63 · golden boxes** — In: `fixtures/boxes.html` (padding, border, explicit width, centred text)
— Out: pixels identical to `fixtures/boxes.expected.ppm`. Why: the box model is where "it lays out"
and "it looks right" can disagree, and only an image comparison sees the difference.

*Why they are numbered rather than meta-tests:* they are spec-level assertions about §7 output, and
numbering puts them inside the ignore budget `tests/meta.rs` polices — a golden that quietly
stopped running would show up as a count change.

*Why the goldens are PPM:* byte-comparable without a decoder dependency, which keeps the
zero-runtime-dependency premise intact. PNG goldens would be smaller but would need a decoder in
dev-dependencies, reopening a question the artifact rests on.

**Coverage check:** parsing 8, cascade 9, box model 7, block layout 6, inline 10, paint 8,
output/CLI 7, robustness 5, golden 3 = **63 tests**, above the 40 minimum, with every SPEC clause
§1–§10 cited at least once. (T59 was added at the Phase 1 gate, T60 at the Phase 2 gate when T58
split, T61–T63 at the Phase 2.75 gate; see their entries for why.)
