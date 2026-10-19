//! `swiftrender` binary: read one HTML file, build the tree, and describe it (`SPEC.md` §9).
//!
//! Phase 1 supports `--emit tree` only. `ppm`, `png` and `svg` parse correctly but exit `1`
//! with an explicit message rather than pretending to render: honest failure is more useful
//! than an empty file.

use std::process::ExitCode;

use swiftrender::cli::{self, CliError, Emit, Options};
use swiftrender::html;

/// Exit code for a usage error (`SPEC.md` §9.2).
const EXIT_USAGE: u8 = 2;
/// Exit code for an I/O failure or an unimplemented artifact (`SPEC.md` §9.2).
const EXIT_FAILURE: u8 = 1;

/// Entry point: parse arguments, run the pipeline, map the result to an exit code.
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    run(&args)
}

/// Run one invocation and return the process exit code.
fn run(args: &[String]) -> ExitCode {
    let options = match cli::parse(args) {
        Ok(options) => options,
        Err(error) => {
            report_usage(&error);
            return ExitCode::from(EXIT_USAGE);
        }
    };

    let source = match std::fs::read_to_string(&options.input) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("swiftrender: cannot read {}: {error}", options.input);
            return ExitCode::from(EXIT_FAILURE);
        }
    };

    if source.len() > swiftrender::MAX_INPUT_BYTES {
        eprintln!(
            "swiftrender: input is {} bytes, over the {} byte limit",
            source.len(),
            swiftrender::MAX_INPUT_BYTES
        );
        return ExitCode::from(EXIT_FAILURE);
    }

    let document = html::parse(&source);
    if options.verbose {
        for warning in &document.warnings {
            eprintln!("swiftrender: warning: {warning}");
        }
    }

    let (width, height) = options.size;
    let text = match options.emit {
        Emit::Tree => html::dump::dump(&document, width, height),
        other => {
            eprintln!(
                "swiftrender: --emit {} is not implemented yet; run with --emit tree",
                other.name()
            );
            return ExitCode::from(EXIT_FAILURE);
        }
    };

    write_output(&options, text.as_bytes())
}

/// Write `bytes` to `--out`, or to stdout when no path was given.
fn write_output(options: &Options, bytes: &[u8]) -> ExitCode {
    match &options.out {
        Some(path) => match std::fs::write(path, bytes) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("swiftrender: cannot write {path}: {error}");
                ExitCode::from(EXIT_FAILURE)
            }
        },
        None => {
            use std::io::Write;
            match std::io::stdout().write_all(bytes) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("swiftrender: cannot write to stdout: {error}");
                    ExitCode::from(EXIT_FAILURE)
                }
            }
        }
    }
}

/// Print a usage error and the usage line to stderr.
fn report_usage(error: &CliError) {
    eprintln!("swiftrender: {error}");
    eprintln!("{}", cli::USAGE);
}
