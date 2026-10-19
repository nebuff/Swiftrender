//! Painting to pixels and serializing artifacts (`SPEC.md` §7–§8).
//!
//! **Phase 3** for the renderer itself; the two checksum/compression primitives below landed in
//! Phase 1 by explicit decision at the Phase 1 gate, so that `TESTS.md` T49's PNG verification
//! rests on already-proven arithmetic rather than on code written in the same breath.
//!
//! Planned surface:
//!
//! * `canvas::Canvas` — opaque RGB buffer, initialized white (§7.2), with `fill_rect` and
//!   `blit_glyph` (1-bit mask, no anti-aliasing, §7.6).
//! * `paint::paint(&BoxTree, &mut Canvas)` — background, border, children, glyphs, in the
//!   order fixed by §7.1.
//! * `ppm::encode(&Canvas) -> Vec<u8>` — `P6` binary (§8.2).
//! * `png::encode(&Canvas) -> Vec<u8>` — colour type 2, filter 0, one IDAT built from
//!   [`deflate::zlib_stored`], with [`checksum::crc32`] per chunk (§8.3). Hand-rolled: no crate
//!   may be added for this without human approval (`QUESTIONS.md` Q3).
//! * `svg::encode(&BoxTree) -> String` — structural dump (§8.4).
//! * `font::GLYPH_8X16` — the built-in bitmap font (§6.6).

pub mod checksum;
pub mod deflate;

/// One RGB pixel, `0..=255` per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

/// Opaque white, the canvas background (`SPEC.md` §7.2).
pub const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};
