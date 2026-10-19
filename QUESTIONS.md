# `QUESTIONS.md` — open uncertainties

Per the rules of engagement, at most three questions per phase. Each question records the
simplest defensible option that was implemented by default, so that a missing answer never
blocks progress. Resolved questions stay here as a decision log.

## Q1 — Is the `style` attribute in scope for Milestone 1?

*Asked in Phase 0. Implemented default: ignored entirely.*

The M1 selector list is "tag selectors and a single class selector" (§3.2). The `style`
attribute is not a selector, so supporting it would silently add a second cascade origin and a
second specificity tier to every test in group B.

**Resolution: RESOLVED — ignore the attribute.** Confirmed by the human at the Phase 0 gate.
Now normative as §3.4a. T16 asserts that `<p style="color:#ff0000">x</p>` renders black ink.

## Q2 — Percentages and `em` units?

*Asked in Phase 0. Implemented default: rejected per declaration, with a warning.*

Percentages require a containing-block-aware resolution pass for every property, including
`padding` and `line-height`, and they interact with the auto-height rule in §6.4. `em` adds a
font-size-dependent unit resolver on top. Neither is needed to demonstrate layout, and both
would multiply the test matrix.

**Resolution: RESOLVED — unsupported in M1.** Confirmed by the human at the Phase 0 gate.
§3.8 requires such declarations to be dropped individually, and T17 pins that behaviour with
`width:50%; height:2em`.

## Q3 — Hand-rolled PNG, or request approval for `flate2`?

*Asked in Phase 0. Implemented default: hand-rolled stored-deflate encoder.*

Hard constraint 1 allows crates only after explicit human approval. A PNG encoder needs
deflate, CRC-32 and Adler-32; all three fit in roughly 150 lines when the deflate stream uses
**stored** (uncompressed) blocks, at the cost of larger files.

**Resolution: RESOLVED — hand-rolled, no `flate2`.** Confirmed by the human at the Phase 0 gate,
explicitly to preserve the "std only" story that makes the repository interesting. T49 decodes
our own output and compares it byte-for-byte against the PPM payload, which is the real proof
that the hand-rolled encoder is correct.

## Phase 2 questions

*(none yet — Phase 1 has not been reviewed)*
