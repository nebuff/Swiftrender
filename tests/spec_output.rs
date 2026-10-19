//! `TESTS.md` group G — output formats, CLI and the code-quality checks (T48–T54).
//!
//! T52–T54 are already meaningful in Phase 1 and run: they assert the process contract and the
//! two `/src` rules. T48–T51 need an artifact to exist, so they wait for Phase 3.

mod common;

use common::*;

/// T48 · PPM header bytes — In: `--emit ppm --size 800x600` — Out: the file begins exactly
/// `P6\n800 600\n255\n` and is `33 + 800*600*3` bytes.
///
/// Why: §8.2 fixes the byte layout, so a viewer can read it without guessing.
#[test]
#[ignore = "phase 3: needs paint"]
fn t48_ppm_header_and_length_are_exact() {
    let ppm = ppm_bytes("<p>x</p>");
    let header = b"P6\n800 600\n255\n";
    assert_eq!(&ppm[..header.len()], header);
    assert_eq!(ppm.len(), header.len() + 800 * 600 * 3);
}

/// T49 · PNG correctness and PPM equivalence — In: the T41 fixture — Out: a PNG whose chunks
/// all pass CRC-32, whose IHDR says 8-bit truecolour, and whose raster equals the PPM payload
/// byte for byte.
///
/// Why: §8.3 — this is the test that proves the hand-rolled deflate/CRC/Adler code is correct.
#[test]
#[ignore = "phase 3: needs paint"]
fn t49_png_is_well_formed_and_matches_ppm() {
    const FIXTURE: &str = "<style>div{background:#123456;height:20px}</style><div></div>";
    let png = png_bytes(FIXTURE);
    assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);

    let chunks = png_chunks(&png);
    let ihdr = &chunks[0];
    assert_eq!(ihdr.name, *b"IHDR");
    assert_eq!(&ihdr.data[..8], &[0, 0, 3, 0x20, 0, 0, 2, 0x58], "800x600");
    assert_eq!(ihdr.data[8], 8, "bit depth");
    assert_eq!(ihdr.data[9], 2, "colour type: truecolour");
    assert_eq!(ihdr.data[12], 0, "no interlace");

    // Every chunk CRC must match what we recompute (§8.3).
    for chunk in &chunks {
        let expected = crc32(&crc32_input(&chunk.name, &chunk.data));
        assert_eq!(chunk.crc, expected, "bad CRC on {:?}", chunk.name);
    }

    // Decoding our own IDAT must reproduce the PPM payload exactly.
    let ppm = ppm_bytes(FIXTURE);
    let header = b"P6\n800 600\n255\n";
    assert_eq!(decode_png_rgb(&png), ppm[header.len()..].to_vec());
}

/// T50 · SVG structure and escaping — In: `div{background:#123456}` with `<p>a&amp;b</p>` —
/// Out: an `<svg>` of the right size, a rect with the right fill, and escaped text.
///
/// Why: §8.4 SVG is a structural dump, so the test asserts structure and escaping, not pixels.
#[test]
#[ignore = "phase 3: needs paint"]
fn t50_svg_is_structural_and_escaped() {
    const FIXTURE: &str = "<style>div{background:#123456;height:20px}</style>\
                           <div></div><p>a&amp;b</p>";
    let svg = svg_text(FIXTURE);
    assert!(svg.contains("width=\"800\""), "{svg}");
    assert!(svg.contains("height=\"600\""), "{svg}");
    assert!(svg.contains("fill=\"#123456\""), "{svg}");
    assert!(svg.contains("a&amp;b"), "text must be XML-escaped: {svg}");
    assert!(svg.trim_end().ends_with("</svg>"), "{svg}");
}

/// T51 · determinism — In: the T39 fixture rendered twice with identical arguments — Out:
/// byte-identical artifacts.
///
/// Why: §8.5 a research artifact must be reproducible; no timestamps or paths may leak in.
#[test]
#[ignore = "phase 3: needs paint"]
fn t51_output_is_byte_identical_across_runs() {
    const FIXTURE: &str = "<div>text<p>para</p>tail</div>";
    assert_eq!(
        png_bytes(FIXTURE),
        png_bytes(FIXTURE),
        "PNG output must be deterministic"
    );
    assert_eq!(
        svg_text(FIXTURE),
        svg_text(FIXTURE),
        "SVG output must be deterministic"
    );
}

/// T52 · exit codes — In: no arguments / a missing file / `--emit bogus` — Out: `2`, `1`, `2`,
/// and no output file written on failure.
///
/// Why: §9.2 CI needs to distinguish a usage error from an I/O failure from success.
#[test]
fn t52_exit_codes_follow_the_contract() {
    let usage = run_cli_raw(&[]);
    assert_eq!(usage.status.code(), Some(2), "no input is a usage error");

    let missing = run_cli_raw(&["definitely-not-a-file.html"]);
    assert_eq!(
        missing.status.code(),
        Some(1),
        "an unreadable input is an I/O failure"
    );

    let bad_emit = run_cli("<p>x</p>", &["--emit", "bogus"]);
    assert_eq!(
        bad_emit.status.code(),
        Some(2),
        "an unknown --emit is a usage error"
    );

    let out = scratch_path("ppm");
    let failed = run_cli(
        "<p>x</p>",
        &["--emit", "bogus", "--out", &out.to_string_lossy()],
    );
    assert_eq!(failed.status.code(), Some(2));
    assert!(!out.exists(), "a failed run must not write an output file");

    let ok = run_cli("<p>x</p>", &["--emit", "tree"]);
    assert_eq!(ok.status.code(), Some(0), "a supported run succeeds");
    assert!(!ok.stdout.is_empty(), "tree output goes to stdout");
}

/// T53 · file length cap — In: every `.rs` file under `src/` — Out: at most 400 lines each.
///
/// Why: §10.1 the size limit is machine-checked, not a convention nobody enforces.
#[test]
fn t53_no_source_file_exceeds_400_lines() {
    let files = src_rust_files();
    assert!(
        !files.is_empty(),
        "the scan must actually find source files"
    );
    for path in files {
        let text = std::fs::read_to_string(&path).expect("source file is readable");
        let lines = text.lines().count();
        assert!(
            lines <= 400,
            "{} has {lines} lines (limit 400, SPEC.md §10.1)",
            relative(&path)
        );
    }
}

/// T54 · module headers — In: every `.rs` file under `src/` — Out: the first line starts with
/// `//!`.
///
/// Why: §10.2 a module without a header cannot explain its slice of the spec.
#[test]
fn t54_every_module_starts_with_a_header() {
    for path in src_rust_files() {
        let text = std::fs::read_to_string(&path).expect("source file is readable");
        let first = text.lines().next().unwrap_or_default();
        assert!(
            first.starts_with("//!"),
            "{} does not start with a module header (SPEC.md §10.2)",
            relative(&path)
        );
    }
}

// ---------------------------------------------------------------------------
// Minimal PNG reader, written for T49 only
// ---------------------------------------------------------------------------

/// One PNG chunk.
struct Chunk {
    name: [u8; 4],
    data: Vec<u8>,
    crc: u32,
}

/// Walk the chunk list of a PNG, in file order.
fn png_chunks(png: &[u8]) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut offset = 8;
    while offset + 12 <= png.len() {
        let length = u32::from_be_bytes([
            png[offset],
            png[offset + 1],
            png[offset + 2],
            png[offset + 3],
        ]) as usize;
        let start = offset + 4;
        let mut name = [0u8; 4];
        name.copy_from_slice(&png[start..start + 4]);
        let data = png[start + 4..start + 4 + length].to_vec();
        let crc_at = start + 4 + length;
        let crc = u32::from_be_bytes([
            png[crc_at],
            png[crc_at + 1],
            png[crc_at + 2],
            png[crc_at + 3],
        ]);
        chunks.push(Chunk { name, data, crc });
        offset = crc_at + 4;
    }
    chunks
}

/// CRC-32 (IEEE) over a chunk's type and data.
fn crc32_input(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut input = name.to_vec();
    input.extend_from_slice(data);
    input
}

/// The CRC-32 used by PNG chunks.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

/// Decode a stored-deflate truecolour PNG back into RGB bytes.
fn decode_png_rgb(png: &[u8]) -> Vec<u8> {
    let mut zlib = Vec::new();
    for chunk in png_chunks(png) {
        if &chunk.name == b"IDAT" {
            zlib.extend_from_slice(&chunk.data);
        }
    }
    let stored = inflate_stored(&zlib);
    let width = 800usize;
    let mut rgb = Vec::with_capacity(stored.len());
    for row in stored.chunks_exact(width * 3 + 1) {
        assert_eq!(row[0], 0, "every scanline must use filter 0");
        rgb.extend_from_slice(&row[1..]);
    }
    rgb
}

/// Inflate a zlib stream consisting of stored (type 0) blocks.
fn inflate_stored(zlib: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut offset = 2; // skip the two-byte zlib header
    while offset + 5 <= zlib.len() {
        let final_block = zlib[offset] & 1 == 1;
        let length = u16::from_le_bytes([zlib[offset + 1], zlib[offset + 2]]) as usize;
        let start = offset + 5;
        out.extend_from_slice(&zlib[start..start + length]);
        offset = start + length;
        if final_block {
            break;
        }
    }
    out
}
