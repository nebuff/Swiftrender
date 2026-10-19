# Swiftrender

**Swiftrender is a research prototype exploring what a single AI agent,
directed by one human, can build.** It is **not** a browser. It renders
a deliberately tiny subset of HTML/CSS, has no JavaScript, and makes no
speed claims. "Swift" is an aspiration, not a benchmark.

## Why this exists

2026 produced two instructive data points for AI-led software engineering:

- **FastRender** (Cursor) showed that *hundreds* of agents can produce a
  browser-shaped artifact — and that a flat multi-agent structure
  collapses into lock contention and task avoidance unless it is
  replaced by a strict Planner-Worker-Judge hierarchy.
- **one-agent-one-browser** showed that *one* strong agent under tight
  human direction can produce a small, legible renderer in days.

Swiftrender is a deliberate bet on the second model. The repo is the
artifact; the orchestration log (`PROMPTS.md`, `NOTES.md`) is the
research output.

## What it does (Milestone 1)

- Parses a small HTML subset (see `SPEC.md` §2)
- Cascades tag and class selectors (see `SPEC.md` §3)
- Block and inline layout, no floats/flex/grid (see `SPEC.md` §6)
- Paints to PPM, PNG (hand-rolled stored-deflate), or SVG (§8)
- 63 numbered tests, each citing its SPEC clause (`TESTS.md`)

## What it explicitly does not do

JavaScript (permanently), network, external fonts, percentages, `em`,
selectors beyond `tag`/`.class`, margin collapsing, baseline alignment,
anti-aliasing, image decoding, tables, forms, bidi. See `ROADMAP.md`.

## Build & run

```sh
cargo run -- fixtures/hello.html --emit png --out hello.png
cargo test
```

## License

Dual-licensed under MIT or Apache-2.0, at your option.
