//! Hand-written HTML tokenizer and tree builder (`SPEC.md` §2).
//!
//! This module intentionally implements a small, *documented* subset of HTML rather than the
//! full HTML5 parsing algorithm: no insertion modes, no foster parenting, no tables
//! (`SPEC.md` §2.12). Everything it does do is deterministic, so the same bytes always
//! produce the same tree.
//!
//! Layers:
//!
//! * [`entity`] — character references (`&amp;`, `&#65;`).
//! * [`tokenizer`] — tags, attributes and text runs.
//! * [`tree`] — tree construction, implicit `<html>`/`<body>`, recovery, depth cap.
//! * [`dump`] — the human-readable tree used by `--emit tree`.

pub mod dump;
pub mod entity;
pub mod tokenizer;
pub mod tree;

pub use entity::decode_entities;
pub use tokenizer::{tokenize, Attr, Token};
pub use tree::{parse, Child, Document, Node};
