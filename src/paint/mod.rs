//! Painting to pixels and serializing artifacts (`SPEC.md` §7–§8).
//!
//! A render is one pass over the finished box tree into an opaque white [`Canvas`], followed by
//! an encoder for the requested artifact. Nothing here changes layout; every coordinate was
//! snapped once before paint began (`SPEC.md` §6.7).
//!
//! * [`canvas`] — the RGB buffer and its clipped rectangle fill,
//! * [`raster`] — background, border, children, glyphs in the order fixed by §7.1,
//! * [`font`] — the hand-drawn 8×16 bitmap font (§6.6),
//! * [`ppm`] — `P6` binary (§8.2),
//! * [`png`] — hand-rolled truecolour PNG over stored deflate (§8.3),
//! * [`svg`] — structural SVG dump (§8.4),
//! * [`checksum`] and [`deflate`] — the Phase 1 primitives the PNG encoder depends on.

pub mod canvas;
pub mod checksum;
pub mod deflate;
pub mod font;
pub mod png;
pub mod ppm;
pub mod raster;
pub mod svg;

pub use canvas::Canvas;
pub use raster::paint_tree;

use crate::css::Color;

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

impl From<Color> for Rgb {
    fn from(color: Color) -> Rgb {
        Rgb {
            r: color.r,
            g: color.g,
            b: color.b,
        }
    }
}

/// Opaque white, the canvas background (`SPEC.md` §7.2).
pub const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};
