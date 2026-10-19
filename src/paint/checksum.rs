//! CRC-32 and Adler-32 checksums (`SPEC.md` §8.3).
//!
//! These are the two checksums a PNG needs: CRC-32 over each chunk's type and data, and
//! Adler-32 over the uncompressed bytes inside the zlib stream.
//!
//! They landed in Phase 1 rather than Phase 3 by explicit decision at the Phase 1 gate. The
//! reasoning was that `TESTS.md` T49 verifies the PNG encoder by recomputing these checksums by
//! hand, so if our own primitives were wrong, the failure would surface during Phase 3 as a
//! *renderer* bug hunt when it was really arithmetic. Both functions are pure, have published
//! check values, and cost about thirty lines to pin — cheap insurance, no external crates.

/// Reflected CRC-32 polynomial used by PNG, gzip and zlib (`0xEDB88320`).
const CRC32_POLYNOMIAL: u32 = 0xEDB8_8320;

/// Largest prime smaller than 2^16, required by Adler-32.
const ADLER32_MODULUS: u32 = 65_521;

/// Lookup table for [`crc32`], built at compile time so there is no lazy-init machinery.
const CRC32_TABLE: [u32; 256] = build_crc32_table();

/// Build the CRC-32 byte table in a `const` context.
const fn build_crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut index = 0;
    while index < 256 {
        let mut crc = index as u32;
        let mut bit = 0;
        while bit < 8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (CRC32_POLYNOMIAL & mask);
            bit += 1;
        }
        table[index] = crc;
        index += 1;
    }
    table
}

/// CRC-32 (IEEE 802.3): reflected polynomial, initial value `0xFFFFFFFF`, final complement.
///
/// # Examples
///
/// ```
/// use swiftrender::paint::checksum::crc32;
/// assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
/// ```
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        let slot = ((crc ^ u32::from(*byte)) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32_TABLE[slot];
    }
    !crc
}

/// Adler-32, as used by zlib: two running sums modulo 65521, high half is the sum of sums.
///
/// # Examples
///
/// ```
/// use swiftrender::paint::checksum::adler32;
/// assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
/// ```
pub fn adler32(bytes: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for byte in bytes {
        a = (a + u32::from(*byte)) % ADLER32_MODULUS;
        b = (b + a) % ADLER32_MODULUS;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::{adler32, crc32};

    /// The published check value for CRC-32/ISO-HDLC: `"123456789"` → `0xCBF43926`.
    #[test]
    fn crc32_matches_the_published_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    /// CRC-32 of the empty input is zero, which also pins the initial/final complement pair.
    #[test]
    fn crc32_of_nothing_is_zero() {
        assert_eq!(crc32(b""), 0);
    }

    /// The worked example from the zlib documentation: `"Wikipedia"` → `0x11E60398`.
    #[test]
    fn adler32_matches_the_zlib_documentation_example() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    /// Adler-32 of nothing is 1, not 0 — the initial value of the first sum.
    #[test]
    fn adler32_of_nothing_is_one() {
        assert_eq!(adler32(b""), 1);
    }

    /// A single byte exercises the table index and the rotate path once.
    #[test]
    fn crc32_of_a_single_zero_byte_is_pinned() {
        // Computed independently with a bit-by-bit reference implementation of the same
        // polynomial; it guards against a table-build regression, not against the polynomial
        // itself (the `123456789` vector above does that).
        let mut crc = 0xFFFF_FFFFu32;
        crc ^= 0;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
        assert_eq!(crc32(&[0u8]), !crc);
    }
}
