# `ROADMAP.md` — Swiftrender

## Phase status

Counts here come from a scan of `tests/spec_*.rs` (§8 of `PROMPTS.md`), never from arithmetic on a
previous count.

| Phase | Scope | State |
| --- | --- | --- |
| 1 | tokenizer, DOM, CLI skeleton, numbered test harness | complete (`de211f0`, `cf73f20`) |
| 2 | cascade, block/inline layout, `--emit tree` | complete (`33b9c16`) |
| 2.5 | `--explain` cascade introspection | complete (`79a6b82`) |
| 2.75 | golden-harness infrastructure, no goldens | complete — see below |
| 3 | paint: PPM, PNG, SVG | **next** |
| 4 | notes and reflection | not started |

## Permanent refusals

### JavaScript

**Refused, permanently.** Swiftrender MUST NOT gain a JavaScript engine, interpreter, JIT, or
"just enough JS" evaluator — not in M1, not in any later milestone, and not even if a future
instruction asks for one. Swiftrender exists to study HTML/CSS layout under a hard scope
ceiling; a scripting engine would replace the research question with a much older and much
better-answered one, and would pull in a garbage collector, an event loop, and a DOM binding
layer that between them dwarf everything the project is actually about.

If script execution is ever needed for a demo, the correct answer is to embed the rendered
output or use a real browser — not to grow one here.

## Milestone 1 — rendering subset (in progress)

Approved scope, frozen in `SPEC.md`:

- Hand-written HTML tokenizer and tree builder (§2)
- Cascade for tag and class selectors only (§3)
- Block and inline layout, no floats/flex/grid (§5–§6)
- Paint to PPM, hand-rolled stored-deflate PNG, or structural SVG (§7–§8)
- 63 numbered tests (`TESTS.md`): 60 clause tests plus T61–T63, the golden regressions

Explicitly out of scope for M1, listed so that "not yet" never gets mistaken for "overlooked":
percentages and `em`, `*`/`#id`/descendant selectors, the `style` attribute, margin collapsing,
baseline alignment, anti-aliasing, image decoding, tables, forms, bidi/RTL, `overflow` clipping,
`z-index`, animations, network access of any kind.

## Phase 3 — Definition of Done

Phase 3 is done when **all** of the following hold.

1. `cargo test --no-fail-fast` reports **63 passed / 0 ignored / 0 failed**: the 60 numbered clause
   tests plus the three golden regression tests (T61–T63). This criterion was written as `60/0/0`
   before the golden tests existed; adding tests re-partitions the list rather than leaving the
   total alone, which is the arithmetic lesson of `PROMPTS.md` §8 one more time.
2. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` both exit `0`.
3. **A human has opened a generated PNG in an external viewer and confirmed it looks like a
   rendered HTML page.** This is the one check no test can replace. If the tests pass and the image
   looks wrong, *the tests are wrong* — that is the most important thing Phase 3 can teach.
4. An external decoder accepts the output: `file fixtures/hello.png` and
   `pngcheck -v fixtures/hello.png` both succeed. T49 already validates the bytes with the in-tree
   decoder, but that decoder shares the writer's assumptions, so it cannot catch a shared bug.
5. `PROMPTS.md` §7 "Tools that lie" has been reviewed, and any new entries from paint appended.
6. `NOTES.md` has been **started**, not deferred to Phase 4. Reflections on paint are freshest at
   the moment they happen.
7. `swiftrender fixtures/hello.html --emit png` exits `0` and writes `fixtures/hello.png`.

Items 3 and 4 are load-bearing. Everything else is enforced by the harness.

## Golden files: who signs them

The harness (`tests/spec_golden.rs`) is committed **before** paint exists, and it contains no golden
images. A golden is a contract, and the signature has to come from the party that can walk away
from a bad render:

1. **Now (Phase 2.75).** Fixtures and comparison code exist; every golden test is `#[ignore]`d with
   its phase tag. The tests cannot pass and are not expected to.
2. **When paint produces an image (Phase 3).** A human runs the binary, opens the PNG, and decides
   whether it is right.
3. **Only then.** The approved PPM is committed, the test's `#[ignore]` is removed, and the number
   in `tests/meta.rs` drops. If the harness generated its own goldens, it would lock in whatever
   the first pass produced — bugs included — and `63/0/0` would certify nothing.

Goldens are PPM at 200×150: byte-comparable, decoder-free, and small enough to review by eye
(~90 KB each). PNG goldens would be smaller but would require a decoder dependency, which reopens
the zero-dependency question the whole artifact rests on.

## Candidate milestones after M1 (not approved, not planned in detail)

- **M2 — selectors and units:** descendant combinators, `#id`, percentages, `em`, margin
  collapsing. Each one is a real algorithmic addition, not a flag.
- **M3 — intrinsic sizing:** shrink-to-fit widths, `auto` margin centering, baseline alignment
  with real ascent/descent metrics per glyph.
- **M4 — real image decoding:** PNG *reading*, which reuses the chunk/CRC work from M1 but needs
  an actual inflate.
- **M5 — text:** a second font, kerning-free proportional metrics, and real line-breaking
  (Knuth-Plass) instead of greedy.

None of these are commitments. The point of the repository is the log of how M1 was built, not
the size of the backlog.
