//! Swiftrender — a small, `std`-only HTML/CSS rendering prototype.
//!
//! This crate is a research artifact, not a browser: it renders a deliberately tiny
//! subset of HTML and CSS, has no JavaScript engine, performs no network I/O, and
//! depends on no external crates (`SPEC.md` §1).
//!
//! # Pipeline
//!
//! ```text
//! HTML text ──tokenize──▶ tokens ──build──▶ DOM ──style──▶ box tree ──layout──▶ boxes ──paint──▶ raster
//!             (html)                     (html)    (css)                 (layout)          (paint)
//! ```
//!
//! # Phase status
//!
//! * Phase 1: [`html`] tokenizer and tree builder.
//! * Phase 2: [`css`] cascade and [`layout`] for the supported subset.
//! * Phase 3: [`paint`] to PPM, PNG and SVG. The golden regression tests stay ignored until a
//!   human approves the first render (`ROADMAP.md`, "Golden files: who signs them").
//!
//! Every module carries a `//!` header and every public item a doc comment, as required
//! by `SPEC.md` §10.2.

pub mod cli;
pub mod css;
pub mod html;
pub mod layout;
pub mod paint;

/// Default canvas size in CSS pixels, used when `--size` is absent (`SPEC.md` §7.2).
pub const DEFAULT_SIZE: (u32, u32) = (800, 600);

/// Longest permitted recursion depth while building the DOM (`SPEC.md` §2.10).
pub const MAX_DEPTH: usize = 512;

/// Largest permitted input document, in bytes (`SPEC.md` §1.5).
pub const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;
