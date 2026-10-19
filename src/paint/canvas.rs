//! The opaque RGB canvas (`SPEC.md` §7.2).
//!
//! The canvas is initialized to white and every write is opaque: there is no alpha channel and
//! no compositing (`SPEC.md` §7.6). Painting is deliberately just clipped rectangle fills; the
//! clipping here is to the canvas bounds, not CSS overflow clipping, which M1 does not have
//! (`SPEC.md` §7.7).

use super::Rgb;

/// A row-major, eight-bit-per-channel RGB image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major RGB triples, three bytes per pixel.
    pub pixels: Vec<u8>,
}

impl Canvas {
    /// Create an opaque white canvas of `width` × `height`.
    pub fn new(width: u32, height: u32) -> Canvas {
        Canvas {
            width,
            height,
            pixels: vec![255; width as usize * height as usize * 3],
        }
    }

    /// Fill the rectangle at `(x, y)` with `color`, clipping it to the canvas.
    ///
    /// Negative or zero-sized rectangles are ignored. Clipping only stops writes outside the
    /// image; it is not the CSS `overflow` property, which M1 does not implement.
    pub fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: Rgb) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + width).min(self.width as i32);
        let y1 = (y + height).min(self.height as i32);
        if x1 <= x0 || y1 <= y0 {
            return;
        }

        let row_bytes = self.width as usize * 3;
        let left = x0 as usize * 3;
        let right = x1 as usize * 3;
        for row in y0..y1 {
            let start = row as usize * row_bytes + left;
            let end = row as usize * row_bytes + right;
            for pixel in self.pixels[start..end].chunks_exact_mut(3) {
                pixel.copy_from_slice(&[color.r, color.g, color.b]);
            }
        }
    }

    /// The RGB triple at `(x, y)`, for tests and diagnostics.
    ///
    /// # Panics
    ///
    /// Panics when the coordinate is outside the canvas.
    pub fn rgb(&self, x: u32, y: u32) -> Rgb {
        let index = (y as usize * self.width as usize + x as usize) * 3;
        Rgb {
            r: self.pixels[index],
            g: self.pixels[index + 1],
            b: self.pixels[index + 2],
        }
    }

    /// The raw RGB bytes, without a header.
    pub fn as_bytes(&self) -> &[u8] {
        &self.pixels
    }

    /// Fill the whole canvas with `color`.
    pub fn clear(&mut self, color: Rgb) {
        self.fill_rect(0, 0, self.width as i32, self.height as i32, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::WHITE;

    /// §7.2: a new canvas is entirely white.
    #[test]
    fn a_new_canvas_is_white() {
        let canvas = Canvas::new(3, 2);
        assert_eq!(canvas.rgb(0, 0), WHITE);
        assert_eq!(canvas.rgb(2, 1), WHITE);
    }

    /// The raster loop's endpoints are the whole bug surface: a 2×2 fill at (1, 1) must set
    /// exactly four pixels and leave their neighbours alone.
    #[test]
    fn fill_rect_writes_exactly_the_requested_pixels() {
        let mut canvas = Canvas::new(4, 4);
        let red = Rgb { r: 255, g: 0, b: 0 };
        canvas.fill_rect(1, 1, 2, 2, red);

        assert_eq!(canvas.rgb(0, 0), WHITE);
        assert_eq!(canvas.rgb(1, 0), WHITE);
        assert_eq!(canvas.rgb(0, 1), WHITE);
        assert_eq!(canvas.rgb(1, 1), red);
        assert_eq!(canvas.rgb(2, 2), red);
        assert_eq!(canvas.rgb(3, 3), WHITE);
        assert_eq!(canvas.rgb(3, 1), WHITE);
    }

    /// Out-of-bounds rectangles are clipped rather than panicking or wrapping.
    #[test]
    fn fill_rect_clips_to_the_canvas() {
        let mut canvas = Canvas::new(2, 2);
        let blue = Rgb { r: 0, g: 0, b: 255 };
        canvas.fill_rect(-1, -1, 3, 3, blue);
        assert_eq!(canvas.rgb(0, 0), blue);
        assert_eq!(canvas.rgb(1, 1), blue);
        canvas.fill_rect(10, 10, 5, 5, Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(canvas.rgb(1, 1), blue);
    }
}
