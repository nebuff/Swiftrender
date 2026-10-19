//! Truecolour PNG encoding (`SPEC.md` §8.3).
//!
//! The format is PNG 1.2 with bit depth 8, colour type 2, no interlacing and filter type 0 on
//! every scanline. Compression is a single `IDAT` containing [`zlib_stored`], and every chunk
//! carries the CRC-32 defined by [`crc32`]. Both primitives landed in Phase 1 so this module can
//! stay a framing exercise: the arithmetic was already pinned before the renderer used it.

use super::canvas::Canvas;
use super::checksum::crc32;
use super::deflate::zlib_stored;

/// The eight-byte PNG signature.
const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Encode `canvas` as a truecolour PNG.
pub fn encode(canvas: &Canvas) -> Vec<u8> {
    let mut out = Vec::with_capacity(canvas.as_bytes().len() + 1024);
    out.extend_from_slice(&SIGNATURE);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&canvas.width.to_be_bytes());
    ihdr.extend_from_slice(&canvas.height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(2); // colour type: truecolour
    ihdr.push(0); // compression: deflate
    ihdr.push(0); // filter method: adaptive, but every row uses filter 0
    ihdr.push(0); // interlace: none
    chunk(&mut out, b"IHDR", &ihdr);

    // §8.3: filter type 0 on every row, so each scanline is one zero byte and its RGB bytes.
    let stride = canvas.width as usize * 3;
    let mut raw = Vec::with_capacity((stride + 1) * canvas.height as usize);
    for row in canvas.as_bytes().chunks_exact(stride) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let compressed = zlib_stored(&raw);
    chunk(&mut out, b"IDAT", &compressed);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// Append one PNG chunk: length, type, data, then the CRC-32 of type+data.
fn chunk(out: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(name);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&out[start..]).to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::Rgb;

    /// Walk the chunk list, checking the framing and every CRC.
    fn verify(png: &[u8]) -> Vec<(String, Vec<u8>)> {
        assert_eq!(&png[..8], &SIGNATURE);
        let mut chunks = Vec::new();
        let mut offset = 8;
        while offset < png.len() {
            let length = u32::from_be_bytes(png[offset..offset + 4].try_into().unwrap()) as usize;
            let name = String::from_utf8_lossy(&png[offset + 4..offset + 8]).to_string();
            let data = png[offset + 8..offset + 8 + length].to_vec();
            let crc_at = offset + 8 + length;
            let crc = u32::from_be_bytes(png[crc_at..crc_at + 4].try_into().unwrap());
            let mut input = png[offset + 4..offset + 8].to_vec();
            input.extend_from_slice(&data);
            assert_eq!(crc, crc32(&input), "bad CRC on {name}");
            chunks.push((name, data));
            offset = crc_at + 4;
        }
        chunks
    }

    /// The framing of a 1×1 image is pinned: IHDR, one IDAT and IEND, with the right header.
    #[test]
    fn framing_is_png_1_2_truecolour() {
        let mut canvas = Canvas::new(1, 1);
        canvas.clear(Rgb { r: 1, g: 2, b: 3 });
        let chunks = verify(&encode(&canvas));
        let names: Vec<&str> = chunks.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["IHDR", "IDAT", "IEND"]);
        assert_eq!(
            chunks[0].1,
            vec![0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0],
            "width, height, depth, colour type, compression, filter, interlace"
        );
    }

    /// A row must start with filter 0 and carry the canvas bytes in order.
    #[test]
    fn scanlines_use_filter_zero() {
        let mut canvas = Canvas::new(2, 2);
        canvas.clear(Rgb { r: 9, g: 8, b: 7 });
        let chunks = verify(&encode(&canvas));
        let idat = &chunks[1].1;
        // 2 header + 1 block header + 2 LEN + 2 NLEN + payload + 4 Adler-32.
        let payload = &idat[7..idat.len() - 4];
        assert_eq!(payload, &[0, 9, 8, 7, 9, 8, 7, 0, 9, 8, 7, 9, 8, 7]);
    }
}
