# `SPEC.md` — Swiftrender, Milestone 1

**Status: APPROVED (Phase 0 gate cleared).** This document is normative: code either
satisfies a clause or it is a bug. Tests in `TESTS.md` cite clause IDs; a change to a
clause requires a change to the citing tests in the same commit.

Keywords (MUST / MUST NOT / SHOULD / MAY) are used per RFC 2119.

## 1. Scope and hard limits

- **§1.1** Crate MUST be `swiftrender`, edition 2021, binary `swiftrender`.
- **§1.2** `std` only. **Zero external crates.** Deflate/zlib/Adler-32/CRC-32 for PNG MUST be
  written in-tree (stored blocks only, see §8.3).
- **§1.3** No JavaScript engine, ever. MUST NOT be added, even if requested; the refusal MUST
  remain recorded in `ROADMAP.md`.
- **§1.4** No network. `<link>`, `<script src>`, `<img src>` MUST be ignored without any I/O.
- **§1.5** Input is exactly one HTML file from `argv`, ≤ 8 MiB. No stdin mode.
- **§1.6** Output is exactly one artifact: `tree`, `ppm`, `png`, or `svg` (§9).

## 2. Tokenizer and tree construction (hand-written, §1.2)

- **§2.1** MUST recognize: start tag, end tag, text, comment `<!-- -->`, doctype
  `<!DOCTYPE …>`, and processing instructions (discarded).
- **§2.2** Tag and attribute names MUST be ASCII-case-insensitive and lowercased.
- **§2.3** Attribute values MAY be double-quoted, single-quoted, or unquoted; boolean
  attributes MUST be accepted.
- **§2.4** MUST decode `&amp; &lt; &gt; &quot; &apos; &#DDD; &#xHHH;`. An unrecognized
  `&name;` MUST stay literal.
- **§2.5** Void elements MUST be `br, img, link, meta` (plus `hr, input, area, base, col,
  embed, source, track, wbr` tolerated as void for robustness).
- **§2.6** Recovery MUST be deterministic: an end tag with no open match is ignored; on
  `</parent>` all still-open descendants are implicitly closed; at EOF all open elements are
  closed.
- **§2.7** Implicit structure: if `<html>` is absent it MUST be synthesized; flow content
  directly under `<html>` MUST be wrapped in a synthesized `<body>`; a missing `<body>` MUST
  be synthesized.
- **§2.8** `<head>` and `<title>` MUST NOT generate boxes. The `<title>` text MUST be kept
  only for the `--emit tree` header line.
- **§2.9** Every `<style>` element, wherever it appears, MUST contribute its text to the
  author stylesheet, in document order.
- **§2.10** Nesting deeper than 512 elements MUST drop the deeper elements and emit one
  stderr warning; MUST NOT panic or stack-overflow.
- **§2.11** Unknown element names MUST become generic `display: inline` elements (no error).
- **§2.12** MUST NOT implement tables, foster parenting, the HTML5 insertion-mode algorithm,
  or `<template>` contents.

## 3. CSS subset

- **§3.1** Grammar: `selector-list { declarations }`; declarations `prop: value` separated by
  `;`; `/* */` comments; at-rules (`@media`, `@import`, …) MUST be dropped whole.
- **§3.2** Selectors MUST support exactly: `tag`, `.class`, `tag.class`, and comma-separated
  lists. Anything else (`*`, `#id`, `a b`, `a>b`, `:hover`, `[attr]`) MUST drop the entire
  rule and warn under `--verbose`.
- **§3.3** Specificity MUST be the pair *(class count, tag count)*; ties resolved by document
  order (later wins).
- **§3.4** Origins: UA sheet, then author sheet; author MUST win at equal specificity.
- **§3.4a** The `style` attribute MUST be ignored entirely (not parsed, not cascaded). This is
  a declared M1 deviation from CSS; see `QUESTIONS.md` Q1.
- **§3.5** `!important` MUST be parsed and ignored (it does not reorder anything).
- **§3.6** Supported properties, and no others: `display`, `width`, `height`, `margin`
  (+ `-top/-right/-bottom/-left`), `padding` (+ sides), `border`, `border-width`,
  `border-style`, `border-color` (+ sides), `background-color`, `color`, `font-size`,
  `font-family`, `text-align`, `line-height`.
- **§3.7** Inherited: `color`, `font-size`, `font-family`, `text-align`, `line-height`. All
  other properties are **not** inherited.
- **§3.8** Value grammar:
  - Length: `<number>px` (decimals allowed, negatives allowed for margins) or bare `0`.
    `em`, `rem`, `%`, `vw`, `vh`, `pt` MUST be rejected (declaration dropped, warning).
  - `auto` valid for `width`, `height`, `margin-*`.
  - `display` ∈ `block | inline | none`; `text-align` ∈ `left | center | right`.
  - `line-height`: `normal` (= **1.25 × font-size**), a unitless number (multiplier), or
    `Npx`.
  - Color: `#rgb`, `#rrggbb`, `transparent`, and the named set `black white red green blue
    yellow fuchsia magenta cyan aqua gray grey silver maroon olive navy purple teal lime
    orange`.
  - `border-style` ∈ `solid | none`. Any other style (`dashed`, `double`, …) MUST drop the
    declaration.
  - `font-family` MUST be parsed and ignored (one built-in font).
  - `inherit` and `initial` MUST be accepted for every property.
- **§3.9** `font-size` MUST be rounded to the nearest integer and clamped to `[1, 256]`. Bad
  or missing values fall back to the inherited value.

## 4. UA stylesheet (normative, byte-exact behaviour)

```
html, body, div, p, ul, li, h1, h2, h3 { display: block }
span, a, br, img                     { display: inline }
body   { margin: 8px }
p      { margin: 16px 0 }
h1     { font-size: 32px; margin: 16px 0 }
h2     { font-size: 24px; margin: 16px 0 }
h3     { font-size: 20px; margin: 16px 0 }
ul     { margin: 16px 0; padding-left: 40px }
a      { color: #0000ee }
```

- **§4.1** Initial values: `color #000000`, `background-color transparent`, `font-size 16px`,
  `line-height normal`, `text-align left`, all margins/padding `0`, `border none`,
  `width/height auto`.
- **§4.2** `li` MUST paint a bullet marker (§6.8). This is engine behaviour, not a CSS
  property.
- **§4.3** The UA sheet MUST be overridable by author rules (`p { margin: 0 }` wins).

## 5. Box generation

- **§5.1** `display: none` MUST suppress the element, its descendants, and its text from
  layout and paint.
- **§5.2** Each block-level child of a block container is a block box in normal flow.
- **§5.3** Contiguous runs of inline-level children (text, inline elements, `<img>`, `<br>`)
  inside a block container MUST be wrapped in **anonymous block boxes**, so every block
  container's children are either all block or all inline.
- **§5.4** A block container whose children are inline establishes an inline formatting
  context (§6.5).
- **§5.5** `<br>` produces a forced line break, zero width, no background.
- **§5.6** `<img>` is an **atomic inline** box: size = `width`/`height` if specified, else
  150 × 50.

## 6. Layout

- **§6.1** 1 CSS px = 1 image px. Origin top-left, LTR, block direction top-to-bottom. No
  writing modes.
- **§6.2** Block content width: `auto` → `max(0, cb_content_w − ml − mr − bl − br − pl − pr)`;
  `Npx` → `N`; `auto` margins → `0` (no centering).
- **§6.3** Block position: content top = parent content top + accumulated flow offset +
  `margin-top`. **Vertical margins MUST NOT collapse.** Horizontal offset = parent content
  left + `margin-left` + `border-left` + `padding-left`.
- **§6.4** Block height: `auto` → distance from content top to the last in-flow child's bottom
  **margin** edge (empty → `0`); explicit `height` overrides and children may overflow
  visibly.
- **§6.5** Inline layout (greedy, no floats):
  - Item heights: text item = `line-height` px; atomic inline = its outer height. Line box
    height `L` = max item height (min 1).
  - Items are placed left to right. A break opportunity exists after a collapsed space run, at
    `<br>`, and before/after an atomic inline. A word is never split.
  - Whitespace: runs of space/tab/LF/CR/FF collapse to one space; leading and trailing spaces
    of each line and of each block's inline content vanish and consume no width.
  - Every item MUST be vertically centred in its line box:
    `top = line_top + round((L − item_h) / 2)`. No baseline alignment in M1.
  - `text-align` shifts each line by `0` / `round((W − used)/2)` / `W − used`, clamped to
    ≥ 0.
  - An unbreakable word wider than `W` MUST overflow to the right without error or clipping.
- **§6.6** Font metrics: `advance = max(1, round(font_size / 2))`; glyph cell =
  `advance × font_size`; the 8×16 bitmap is scaled nearest-neighbour. So 16px → 8×16 cells,
  32px → 16×32.
- **§6.7** Geometry is computed in `f64` and snapped to integer pixels with **round-half-up
  exactly once**, before painting and before emitting the box tree. All test assertions are on
  snapped integers.
- **§6.8** `li` marker: an 8×16 (scaled, §6.6) bullet placed at `x = content_left − 16`,
  `y = first_line_top + round((L − font_size)/2)`.

## 7. Painting

- **§7.1** Recursive paint order: box background → box border → children in tree order (line
  boxes paint each item's background/border, then its glyphs). Later boxes paint over earlier
  ones.
- **§7.2** Canvas is **opaque white**, size from `--size` (default 800×600).
- **§7.3** `background-color: transparent` paints nothing.
- **§7.4** Borders: solid only, four rects between padding box and border box;
  `border-color` default is `currentColor`.
- **§7.5** `<img>` placeholder fills its content box with `background-color` if opaque, else
  `#c0c0c0`.
- **§7.6** Glyphs are 1-bit masks (no anti-aliasing) painted in `color`. No alpha
  compositing anywhere: every painted colour is opaque.
- **§7.7** No clipping, no `overflow`, no `z-index`, no shadows, no radii.

## 8. Output formats

- **§8.1** `tree`: deterministic text. Header
  `swiftrender <title or (untitled)> <W>x<H>`; then one line per box, indented 2 spaces per
  depth:
  `tag.cls   x=<i> y=<i> w=<i> h=<i>  margin=<t,r,b,l> padding=<t,r,b,l> border=<t,r,b,l>`.
  Anonymous blocks print as `#anon`, text runs as `"text"`.
- **§8.2** `ppm`: exactly `P6\n<W> <H>\n255\n` followed by RGB triples, no comments.
- **§8.3** `png`: PNG 1.2, bit depth 8, colour type 2 (truecolour), no interlace, filter type
  0 on every row, a single `IDAT` built from **stored** deflate blocks, correct per-chunk
  CRC-32 and zlib Adler-32. MUST be decodable by standard viewers.
- **§8.4** `svg`: `<svg xmlns="http://www.w3.org/2000/svg" width height>`; `<rect>` for
  backgrounds/borders/placeholders; `<text x y font-size fill>` for runs, XML-escaped. SVG is
  a **structural dump**, explicitly not required to be pixel-identical to the raster.
- **§8.5** Same input + same args ⇒ **byte-identical** output. No timestamps, versions, or
  absolute paths inside artifacts.

## 9. CLI

```
swiftrender <INPUT.html> [--emit tree|ppm|png|svg] [--out PATH] [--size WxH] [--verbose]
```

- **§9.1** `--emit` defaults to `png`; `--out` defaults to stdout for `tree`, else
  `<stem>.<ext>`.
- **§9.2** Exit `0` success, `2` usage error (missing arg, bad flag, bad `--size`), `1` I/O
  failure (missing/unreadable/unwritable file). Malformed HTML MUST NOT be an error.
- **§9.3** `--verbose` sends dropped-rule/dropped-declaration warnings to **stderr**.

## 10. Code quality (self-enforced)

- **§10.1** No file in `src/` may exceed **400 lines** (enforced by §T53).
- **§10.2** Every module starts with a `//!` header; every public item has a doc comment
  (module-header check is §T54).
- **§10.3** Every clause here SHOULD map to ≥ 1 numbered test in `TESTS.md`.

## 11. Non-goals for M1

JavaScript (permanently), selectors beyond `tag`/`.class`, floats/flex/grid/positioning,
percentages, `em`, external fonts, real image decoding, anti-aliasing, tables, forms,
bidi/RTL, margin collapsing, overflow clipping, `style` attribute, `z-index`, animations.
