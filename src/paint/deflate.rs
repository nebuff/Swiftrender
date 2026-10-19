//! Stored-block zlib writer (`SPEC.md` §8.3).
//!
//! `SPEC.md` §1.2 forbids external crates, and `QUESTIONS.md` Q3 records the decision to write
//! PNG's compression layer in-tree. This module is that layer, in its simplest correct form:
//! **stored** (uncompressed) deflate blocks. The output is larger than a real encoder would
//! produce and completely valid, which is the right trade for a research artifact whose PNGs
//! are diagnostic images rather than web assets.
//!
//! Landed in Phase 1 by explicit decision at the Phase 1 gate, so that T49's PNG verification
//! has something already-proven underneath it before the renderer depends on it.

use super::checksum::adler32;

/// Largest payload a single stored deflate block may carry (2^16 − 1 bytes).
const MAX_STORED_BYTES: usize = 65_535;

/// Wrap `data` in a zlib stream built entirely from stored deflate blocks.
///
/// The stream is a two-byte zlib header, one or more stored blocks, then the Adler-32 of the
/// **uncompressed** `data` in big-endian order, as zlib requires.
///
/// # Examples
///
/// ```
/// use swiftrender::paint::deflate::zlib_stored;
/// let stream = zlib_stored(b"hello");
/// assert_eq!(&stream[..2], &[0x78, 0x01]);
/// // 2 header + 1 block byte + 2 LEN + 2 NLEN + 5 payload + 4 Adler-32
/// assert_eq!(stream.len(), 16);
/// ```
pub fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let blocks = data.len() / MAX_STORED_BYTES + 1;
    let mut out = Vec::with_capacity(data.len() + blocks * 5 + 6);

    // 0x78 = deflate, 32 KiB window, no preset dictionary. 0x01 is the "fastest" FLEVEL whose
    // FCHECK bits make the two-byte header a multiple of 31, as the format requires.
    out.push(0x78);
    out.push(0x01);

    let mut offset = 0;
    loop {
        let remaining = data.len() - offset;
        let take = remaining.min(MAX_STORED_BYTES);
        let is_last = take == remaining;

        // Block header byte: BFINAL in bit 0, BTYPE = 00 (stored) in bits 1-2.
        out.push(if is_last { 0x01 } else { 0x00 });

        // Every stored block carries LEN followed by its one's complement, both little-endian.
        let length = take as u16;
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&(!length).to_le_bytes());
        out.extend_from_slice(&data[offset..offset + take]);

        offset += take;
        if is_last {
            break;
        }
    }

    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::zlib_stored;

    /// Inflate a stream produced by [`zlib_stored`], for roundtrip tests only.
    fn inflate_stored(stream: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut offset = 2; // skip the zlib header
        loop {
            let final_block = stream[offset] & 1 == 1;
            let length = u16::from_le_bytes([stream[offset + 1], stream[offset + 2]]) as usize;
            let complement = u16::from_le_bytes([stream[offset + 3], stream[offset + 4]]);
            assert_eq!(
                !complement, length as u16,
                "NLEN must be the one's complement of LEN"
            );
            let start = offset + 5;
            out.extend_from_slice(&stream[start..start + length]);
            offset = start + length;
            if final_block {
                break;
            }
        }
        let adler = u32::from_be_bytes([
            stream[offset],
            stream[offset + 1],
            stream[offset + 2],
            stream[offset + 3],
        ]);
        assert_eq!(
            adler,
            super::super::checksum::adler32(&out),
            "trailing Adler-32"
        );
        out
    }

    /// The byte framing of a tiny payload is pinned exactly, including the hand-computed
    /// Adler-32 of `"hello"` (`0x062C0215`). This is the check that would catch a wrong zlib
    /// header, a missing NLEN complement or a little-endian slip.
    #[test]
    fn framing_of_hello_is_pinned_byte_for_byte() {
        let stream = zlib_stored(b"hello");
        assert_eq!(
            stream,
            vec![
                0x78, 0x01, // zlib header
                0x01, // final stored block
                0x05, 0x00, // LEN = 5
                0xFA, 0xFF, // NLEN = !5
                b'h', b'e', b'l', b'l', b'o', // payload
                0x06, 0x2C, 0x02, 0x15, // Adler-32 of "hello"
            ]
        );
    }

    /// The empty payload still needs one (empty) block plus its Adler-32, and that block must be
    /// marked final: `0x01` is BFINAL = 1 with BTYPE = 00, followed by LEN = 0 and NLEN = 0xFFFF.
    #[test]
    fn framing_of_nothing_is_a_single_empty_final_block() {
        assert_eq!(
            zlib_stored(b""),
            vec![0x78, 0x01, 0x01, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x01]
        );
    }

    /// Roundtrip through a decoder written to the same specification.
    #[test]
    fn small_payload_roundtrips() {
        let data: Vec<u8> = (0..=255u8).collect();
        assert_eq!(inflate_stored(&zlib_stored(&data)), data);
    }

    /// Payloads over the 64 KiB block limit must split, with BFINAL only on the last block.
    #[test]
    fn large_payload_splits_into_blocks() {
        let data: Vec<u8> = (0..70_000u32).map(|i| (i % 251) as u8).collect();
        let stream = zlib_stored(&data);
        assert_eq!(stream[2], 0x00, "the first block is not final");
        assert_eq!(inflate_stored(&stream), data);
    }
}
