//! Parsing the `--emit tree` box dump into `BoxInfo` values.
//!
//! Split out of `common/mod.rs` to stay inside the 400-line budget. The helpers here are how every
//! numbered layout test reads the *artifact* a human would read, rather than reaching into a
//! library struct: see the contract note on `boxes`.

use super::run_cli;

/// One laid-out box, as the Phase 2 box tree will report it (`SPEC.md` §6.7: integers only).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoxInfo {
    /// Tag name, `#anon` for an anonymous block (`SPEC.md` §5.3), or `#text` for a text run.
    pub tag: String,
    /// Class names on the element, in source order.
    pub classes: Vec<String>,
    /// Left edge of the box in pixels.
    pub x: i32,
    /// Top edge of the box in pixels.
    pub y: i32,
    /// Box width in pixels.
    pub width: i32,
    /// Box height in pixels.
    pub height: i32,
    /// Margin edges as `[top, right, bottom, left]`.
    pub margin: [i32; 4],
    /// Padding edges as `[top, right, bottom, left]`.
    pub padding: [i32; 4],
    /// Border widths as `[top, right, bottom, left]`, zero unless solid.
    pub border: [i32; 4],
    /// Text content of the box, if any.
    pub text: Option<String>,
    /// Measured width of [`BoxInfo::text`] in pixels (`SPEC.md` §6.6).
    pub text_width: i32,
    /// Computed font size in pixels.
    pub font_size: i32,
    /// Computed text colour.
    pub color: (u8, u8, u8),
}

/// Lay `html` out and return every box in pre-order.
///
/// **This runs the real binary** with `--emit tree` and parses the `SPEC.md` §8.1 lines rather than
/// reaching into the library. Layout is the layer where an in-process assertion is easiest and
/// least meaningful: if `--emit tree` prints a box at `(8,24,784,20)` while some internal struct
/// agrees, the artifact a human would read is still the one that has to be right. PNG, SVG and PPM
/// already work this way (`png_bytes`, `svg_text`, `ppm_bytes`).
///
/// `BoxInfo` fields are exact `i32` values because §6.7 snaps geometry once, before the tree is
/// printed: there is no tolerance to apply and no float to compare.
///
/// **Do not "optimize" this into a library call.** The point is that a human reading
/// `--emit tree` and a test asserting on the same bytes cannot disagree; routing the assertion
/// through an internal struct would recreate exactly the gap this helper exists to close. This
/// decision was reviewed and confirmed at the Phase 1 gate, so changing it needs a reason that
/// beats that one in a commit message.
pub fn boxes(html: &str) -> Vec<BoxInfo> {
    let output = run_cli(html, &["--emit", "tree"]);
    assert!(
        output.status.success(),
        "--emit tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).expect("the tree dump is UTF-8");
    let parsed = parse_box_dump(&text);
    assert!(!parsed.is_empty(), "the dump had no boxes:\n{text}");
    parsed
}

/// Parse a `--emit tree` dump into boxes, in the order they appear.
///
/// The line grammar is fixed by `SPEC.md` §8.1 plus the Phase 2 additions documented in
/// `layout/serialize.rs`:
///
/// ```text
/// {label}  x=… y=… w=… h=…  margin=t,r,b,l padding=t,r,b,l border=t,r,b,l  font=… color=#rrggbb
/// ```
///
/// A label is an element name with optional classes, `#anon`, or a quoted text run. Because a
/// quoted label can contain spaces, the label is taken as everything before the `  x=` marker
/// rather than as the first whitespace-separated token.
pub fn parse_box_dump(text: &str) -> Vec<BoxInfo> {
    let mut out = Vec::new();
    for line in text.lines() {
        // The split consumes the `x=` key along with the separator, so put it back before
        // tokenising: without this, `x` silently defaulted to 0 and only assertions that happened
        // to expect 0 could pass.
        let Some((label, rest)) = line.trim_start().split_once("  x=") else {
            continue;
        };
        if label.is_empty() {
            continue;
        }
        let fields = format!("x={rest}");
        let mut info = BoxInfo {
            ..BoxInfo::default()
        };
        if let Some(quoted) = label.strip_prefix('"') {
            info.tag = "#text".to_string();
            info.text = Some(unescape(quoted.strip_suffix('"').unwrap_or(quoted)));
        } else {
            let (tag, classes) = match label.split_once('.') {
                Some((tag, rest)) => (tag, rest.split('.').map(str::to_string).collect()),
                None => (label, Vec::new()),
            };
            info.tag = tag.to_string();
            info.classes = classes;
        }
        for token in fields.split_whitespace() {
            let Some((key, value)) = token.split_once('=') else {
                continue;
            };
            match key {
                "x" => info.x = parse_int(value),
                "y" => info.y = parse_int(value),
                "w" => info.width = parse_int(value),
                "h" => info.height = parse_int(value),
                "margin" => info.margin = parse_edges(value),
                "padding" => info.padding = parse_edges(value),
                "border" => info.border = parse_edges(value),
                "font" => info.font_size = parse_int(value),
                "color" => info.color = parse_color(value),
                _ => {}
            }
        }
        if info.tag == "#text" {
            info.text_width = info.width;
        }
        out.push(info);
    }
    out
}

/// Parse an `i32` field, panicking with the raw text when the dump is malformed.
fn parse_int(value: &str) -> i32 {
    value
        .parse()
        .unwrap_or_else(|_| panic!("unparsable integer {value:?} in the box dump"))
}

/// Parse a `t,r,b,l` field.
fn parse_edges(value: &str) -> [i32; 4] {
    let parts: Vec<i32> = value.split(',').map(parse_int).collect();
    assert_eq!(parts.len(), 4, "expected four edges in {value:?}");
    [parts[0], parts[1], parts[2], parts[3]]
}

/// Parse `#rrggbb`.
fn parse_color(value: &str) -> (u8, u8, u8) {
    let hex = value.strip_prefix('#').unwrap_or(value);
    let value = u32::from_str_radix(hex, 16)
        .unwrap_or_else(|_| panic!("unparsable colour {value:?} in the box dump"));
    (
        ((value >> 16) & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        (value & 0xFF) as u8,
    )
}

/// Undo the escaping `layout/tree.rs` applies to text runs.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Find the first box whose tag equals `tag`.
pub fn find<'a>(boxes: &'a [BoxInfo], tag: &str) -> &'a BoxInfo {
    boxes
        .iter()
        .find(|item| item.tag == tag)
        .unwrap_or_else(|| panic!("no box with tag {tag:?} in {boxes:#?}"))
}

/// All boxes whose tag equals `tag`.
pub fn filter<'a>(boxes: &'a [BoxInfo], tag: &str) -> Vec<&'a BoxInfo> {
    boxes.iter().filter(|item| item.tag == tag).collect()
}

// ---------------------------------------------------------------------------
// Paint tier (Phase 3)
// ---------------------------------------------------------------------------
