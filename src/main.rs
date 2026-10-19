//! `swiftrender` binary: read one HTML file, lay it out, and emit one artifact (`SPEC.md` §9).
//!
//! `tree` goes to stdout by default; the three image/structural artifacts default to the input
//! path with their own extension (`fixtures/hello.html` → `fixtures/hello.png`). `--out`
//! overrides either rule, and a failed run never leaves a partial artifact behind.

use std::path::PathBuf;
use std::process::ExitCode;

use swiftrender::cli::{self, CliError, Emit, Options};
use swiftrender::css::{explain, parse::Selector, Css};
use swiftrender::html;
use swiftrender::layout;
use swiftrender::paint;

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
    let mut warnings = document.warnings.clone();
    let (width, height) = options.size;
    let css = Css::from_document(&document, &mut warnings);

    // §9.4: `--explain` answers a question about styles, so it runs before layout — there is no
    // reason to lay the document out to explain why a declaration lost.
    if let Some(query) = &options.explain {
        let selector =
            Selector::parse(query).expect("the CLI rejects selectors outside the subset (§9.2)");
        let report = explain::explain(&document, &css, &selector);
        print_warnings(&options, &warnings);
        return write_output(options.out.as_deref(), report.as_bytes());
    }

    let tree = layout::layout_document(&document, &css, width, height, &mut warnings);
    print_warnings(&options, &warnings);

    let bytes = match options.emit {
        Emit::Tree => layout::dump_tree(&tree).into_bytes(),
        Emit::Ppm => paint::ppm::encode(&paint::paint_tree(&tree)),
        Emit::Png => paint::png::encode(&paint::paint_tree(&tree)),
        Emit::Svg => paint::svg::encode(&tree).into_bytes(),
    };

    write_output(default_output_path(&options).as_deref(), &bytes)
}

/// Where the artifact should go when `--out` is absent (`SPEC.md` §9.1).
///
/// `tree` and `--explain` are textual reports, so they stay on stdout; the three artifacts are
/// written beside the input as `<stem>.<ext>`, which is what `swiftrender fixtures/hello.html
/// --emit png` promises in `ROADMAP.md`.
fn default_output_path(options: &Options) -> Option<String> {
    if options.explain.is_some() || options.emit == Emit::Tree {
        return options.out.clone();
    }
    options.out.clone().or_else(|| {
        let mut path = PathBuf::from(&options.input);
        path.set_extension(options.emit.extension());
        Some(path.to_string_lossy().into_owned())
    })
}

/// Print collected warnings to stderr when `--verbose` is set (§9.3).
fn print_warnings(options: &Options, warnings: &[String]) {
    if options.verbose {
        for warning in warnings {
            eprintln!("swiftrender: warning: {warning}");
        }
    }
}

/// Write `bytes` to `path`, or to stdout when no path was given.
fn write_output(path: Option<&str>, bytes: &[u8]) -> ExitCode {
    match path {
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
