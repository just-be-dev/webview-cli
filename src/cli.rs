//! Argument parsing.
//!
//! The flag surface is deliberately tiny: everything about the *interaction*
//! lives in the HTML the caller provides, so the only flags here shape the
//! window or bound the wait. Nothing parses or validates the page's result.

use clap::Parser;

/// Shown only for `--help` (the long form). `-h` stays terse with the one-line
/// `about` above.
const LONG_ABOUT: &str = "\
Render HTML in a native webview, print the single result the page sends back, then exit.

The HTML comes from a FILE argument, or from stdin when piped in. The page reports its
result over the JavaScript bridge that webview injects as `window.webview`:

  window.webview.resolve(value)   print `value` to stdout, exit 0
  window.webview.reject(reason)   print `reason` to stderr, exit 1

webview never parses or validates the result \u{2014} whatever the page passes is printed
verbatim, so the caller decides how to interpret it.";

/// Examples + the exit-code table (the table is part of the public API; keep it
/// in sync with `run::exit`). Appended after the options on `--help`.
const AFTER_LONG_HELP: &str = "\
Examples:
  webview page.html                     render a file and wait for a result
  cat page.html | webview               render HTML piped on stdin
  webview page.html --timeout-ms 5000   give up after 5 seconds
  webview page.html --title Pick --width 480 --height 320

Exit codes:
  0    the page called resolve(value)
  1    the page called reject(reason)
  2    the window was closed before the page settled
  3    --timeout-ms elapsed before the page settled
  64   usage error (no HTML on stdin or as a file argument)";

/// Parsed command-line arguments.
///
/// The short `about` (for `-h`) comes from the crate description in
/// `Cargo.toml` via the bare `about` attribute, so there's a single source of
/// truth; the long `--help` text lives in `LONG_ABOUT` / `AFTER_LONG_HELP`.
#[derive(Parser, Debug)]
#[command(
    name = "webview",
    version,
    about,
    long_about = LONG_ABOUT,
    after_long_help = AFTER_LONG_HELP,
)]
pub struct Cli {
    /// HTML file to render. Omit to read HTML from stdin.
    #[arg(value_name = "FILE")]
    pub path: Option<std::path::PathBuf>,

    /// Window title.
    #[arg(long, default_value = "webview")]
    pub title: String,

    /// Window width in logical pixels.
    #[arg(long, default_value_t = 800)]
    pub width: u32,

    /// Window height in logical pixels.
    #[arg(long, default_value_t = 600)]
    pub height: u32,

    /// Open developer tools on launch (when the build supports it).
    #[arg(long)]
    pub devtools: bool,

    /// Path to an image to show as the Dock icon while the window is open
    /// (macOS only; ignored elsewhere).
    #[arg(long, value_name = "PATH")]
    pub icon: Option<std::path::PathBuf>,

    /// Exit with code 3 if the page hasn't settled within this many milliseconds.
    #[arg(long, value_name = "MS")]
    pub timeout_ms: Option<u64>,
}

impl Cli {
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
