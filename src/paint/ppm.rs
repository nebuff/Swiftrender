//! Binary PPM (`P6`) encoding (`SPEC.md` §8.2).
//!
//! The format is fixed to one decimal header and raw RGB triples, with no comments, so a viewer
//! can read the file without guessing and a test can compare its header byte for byte.

use super::canvas::Canvas;

/// Encode `canvas` as a `P6` PPM.
pub fn encode(canvas: &Canvas) -> Vec<u8> {
    let header = format!("P6\n{} {}\n255\n", canvas.width, canvas.height);
    let mut out = Vec::with_capacity(header.len() + canvas.as_bytes().len());
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(canvas.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::Rgb;

    /// §8.2: the header is exact and the payload starts immediately after it.
    #[test]
    fn header_is_pinned() {
        let mut canvas = Canvas::new(2, 1);
        canvas.clear(Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(
            encode(&canvas),
            vec![
                b'P', b'6', b'\n', b'2', b' ', b'1', b'\n', b'2', b'5', b'5', b'\n', 1, 2, 3, 1, 2,
                3,
            ]
        );
    }
}
