# `ROADMAP.md` — Swiftrender

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
- 58 numbered tests, each citing its clause (`TESTS.md`)

Explicitly out of scope for M1, listed so that "not yet" never gets mistaken for "overlooked":
percentages and `em`, `*`/`#id`/descendant selectors, the `style` attribute, margin collapsing,
baseline alignment, anti-aliasing, image decoding, tables, forms, bidi/RTL, `overflow` clipping,
`z-index`, animations, network access of any kind.

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
