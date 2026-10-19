//! Command-line interface (`SPEC.md` §9).
//!
//! Parsing is deliberately hand-written and total: every malformed invocation produces a
//! [`CliError`], which the binary turns into exit code `2`. Nothing here touches the
//! filesystem, so the parser is trivially testable.

use std::fmt;

/// The artifact a run should produce (`--emit`, `SPEC.md` §1.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emit {
    /// Debug dump of the tree (DOM in Phase 1, box tree from Phase 2).
    Tree,
    /// Binary PPM (`P6`), `SPEC.md` §8.2.
    Ppm,
    /// Truecolour PNG with a hand-rolled stored-deflate IDAT, `SPEC.md` §8.3.
    Png,
    /// Structural SVG dump, `SPEC.md` §8.4.
    Svg,
}

impl Emit {
    /// Parse the value of `--emit`.
    ///
    /// Returns [`CliError::BadEmit`] for anything outside the four documented names.
    pub fn parse(value: &str) -> Result<Emit, CliError> {
        match value {
            "tree" => Ok(Emit::Tree),
            "ppm" => Ok(Emit::Ppm),
            "png" => Ok(Emit::Png),
            "svg" => Ok(Emit::Svg),
            other => Err(CliError::BadEmit(other.to_string())),
        }
    }

    /// File extension used when `--out` is absent.
    pub fn extension(self) -> &'static str {
        match self {
            Emit::Tree => "txt",
            Emit::Ppm => "ppm",
            Emit::Png => "png",
            Emit::Svg => "svg",
        }
    }

    /// Human-readable name, used in error messages.
    pub fn name(self) -> &'static str {
        match self {
            Emit::Tree => "tree",
            Emit::Ppm => "ppm",
            Emit::Png => "png",
            Emit::Svg => "svg",
        }
    }
}

/// A fully parsed invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Path to the input HTML file (positional argument).
    pub input: String,
    /// Requested artifact (`--emit`, default [`Emit::Png`] per §9.1).
    pub emit: Emit,
    /// Explicit output path (`--out`); `None` means stdout for `tree`, else `<stem>.<ext>`.
    pub out: Option<String>,
    /// Canvas size in pixels (`--size`, default 800×600).
    pub size: (u32, u32),
    /// Whether warnings should be printed to stderr (`--verbose`, §9.3).
    pub verbose: bool,
}

/// Usage errors. All of these map to exit code `2` (`SPEC.md` §9.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    /// No positional argument was supplied.
    MissingInput,
    /// A flag that requires a value was at the end of the argument list.
    MissingValue(String),
    /// A flag not defined by the CLI contract.
    UnknownFlag(String),
    /// `--emit` was given a name outside the four supported artifacts.
    BadEmit(String),
    /// `--size` was not of the form `WxH`, or a dimension was zero or unparsable.
    BadSize(String),
    /// A second positional argument appeared.
    UnexpectedArgument(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::MissingInput => write!(f, "missing input file"),
            CliError::MissingValue(flag) => write!(f, "{flag} requires a value"),
            CliError::UnknownFlag(flag) => write!(f, "unknown flag {flag}"),
            CliError::BadEmit(value) => {
                write!(
                    f,
                    "unknown --emit value {value:?} (expected tree, ppm, png or svg)"
                )
            }
            CliError::BadSize(value) => {
                write!(f, "bad --size value {value:?} (expected WxH, e.g. 800x600)")
            }
            CliError::UnexpectedArgument(value) => write!(f, "unexpected argument {value:?}"),
        }
    }
}

/// Usage line printed alongside every [`CliError`].
pub const USAGE: &str = "usage: swiftrender <INPUT.html> [--emit tree|ppm|png|svg] [--out PATH] [--size WxH] [--verbose]";

/// Parse `args`, which must exclude `argv[0]`.
///
/// # Examples
///
/// ```
/// use swiftrender::cli::{parse, Emit};
/// let opts = parse(&["in.html".to_string(), "--emit".to_string(), "tree".to_string()]).unwrap();
/// assert_eq!(opts.emit, Emit::Tree);
/// ```
pub fn parse(args: &[String]) -> Result<Options, CliError> {
    let mut opts = Options {
        input: String::new(),
        emit: Emit::Png,
        out: None,
        size: crate::DEFAULT_SIZE,
        verbose: false,
    };
    let mut input: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--emit" => {
                let value = value_of(args, i, "--emit")?;
                opts.emit = Emit::parse(value)?;
                i += 2;
            }
            "--out" => {
                let value = value_of(args, i, "--out")?;
                opts.out = Some(value.to_string());
                i += 2;
            }
            "--size" => {
                let value = value_of(args, i, "--size")?;
                opts.size = parse_size(value)?;
                i += 2;
            }
            "--verbose" => {
                opts.verbose = true;
                i += 1;
            }
            "--" => {
                i += 1;
                while i < args.len() {
                    if input.is_some() {
                        return Err(CliError::UnexpectedArgument(args[i].clone()));
                    }
                    input = Some(args[i].clone());
                    i += 1;
                }
            }
            flag if flag.starts_with("--") => return Err(CliError::UnknownFlag(flag.to_string())),
            positional => {
                if input.is_some() {
                    return Err(CliError::UnexpectedArgument(positional.to_string()));
                }
                input = Some(positional.to_string());
                i += 1;
            }
        }
    }

    opts.input = input.ok_or(CliError::MissingInput)?;
    Ok(opts)
}

/// Fetch the value following the flag at `index`, or report it as missing.
fn value_of<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, CliError> {
    match args.get(index + 1) {
        Some(value) => Ok(value.as_str()),
        None => Err(CliError::MissingValue(flag.to_string())),
    }
}

/// Parse a `WxH` size specification; both dimensions must be non-zero.
fn parse_size(value: &str) -> Result<(u32, u32), CliError> {
    let (w, h) = value
        .split_once(['x', 'X'])
        .ok_or_else(|| CliError::BadSize(value.to_string()))?;
    let w: u32 = w
        .parse()
        .map_err(|_| CliError::BadSize(value.to_string()))?;
    let h: u32 = h
        .parse()
        .map_err(|_| CliError::BadSize(value.to_string()))?;
    if w == 0 || h == 0 {
        return Err(CliError::BadSize(value.to_string()));
    }
    Ok((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn defaults_are_png_and_800x600() {
        let opts = parse(&args(&["a.html"])).unwrap();
        assert_eq!(opts.emit, Emit::Png);
        assert_eq!(opts.size, (800, 600));
        assert!(!opts.verbose);
    }

    #[test]
    fn flags_may_precede_the_input() {
        let opts = parse(&args(&["--verbose", "--size", "10x20", "a.html"])).unwrap();
        assert_eq!(opts.size, (10, 20));
        assert_eq!(opts.input, "a.html");
    }

    #[test]
    fn bad_size_and_bad_emit_are_usage_errors() {
        assert_eq!(
            parse(&args(&["a", "--size", "0x10"])),
            Err(CliError::BadSize("0x10".into()))
        );
        assert_eq!(
            parse(&args(&["a", "--emit", "jpeg"])),
            Err(CliError::BadEmit("jpeg".into()))
        );
    }
}
