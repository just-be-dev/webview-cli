//! Input resolution: where does the HTML come from?
//!
//! Precedence:
//!   1. stdin is not a TTY (something piped in) -> read to EOF as inline HTML.
//!   2. a positional path was given            -> load that file.
//!   3. neither                                 -> usage error (exit 64).

use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

/// What the webview should load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Load {
    /// Inline HTML (about:blank origin).
    Html(String),
    /// A file on disk (file:// origin, with read access to its parent dir).
    File(PathBuf),
}

/// Why we couldn't resolve any input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    /// No HTML piped on stdin and no path argument given.
    NoInput,
    /// stdin was piped but reading it failed.
    StdinRead(String),
}

/// Resolve the input from stdin-is-a-tty state and an optional path.
///
/// `stdin_is_tty` is injected so this is unit-testable without a real terminal;
/// `read_stdin` likewise defers the actual read so tests don't touch fd 0.
pub fn resolve<F>(
    stdin_is_tty: bool,
    path: Option<PathBuf>,
    read_stdin: F,
) -> Result<Load, InputError>
where
    F: FnOnce() -> io::Result<String>,
{
    // 1. Piped stdin wins. A non-TTY stdin means a caller streamed HTML in.
    //    Empty/whitespace-only input doesn't count — that's `< /dev/null`, not
    //    a page — so we fall through to a path argument or a usage error.
    if !stdin_is_tty {
        let html = read_stdin().map_err(|e| InputError::StdinRead(e.to_string()))?;
        if !html.trim().is_empty() {
            return Ok(Load::Html(html));
        }
    }

    // 2. Otherwise fall back to a file path.
    if let Some(p) = path {
        return Ok(Load::File(p));
    }

    // 3. Nothing to render.
    Err(InputError::NoInput)
}

/// Resolve input from the real process environment.
pub fn resolve_from_env(path: Option<PathBuf>) -> Result<Load, InputError> {
    let stdin_is_tty = io::stdin().is_terminal();
    resolve(stdin_is_tty, path, || {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piped_stdin_is_read_as_inline_html() {
        let got = resolve(false, None, || Ok("<h1>hi</h1>".to_string()));
        assert_eq!(got, Ok(Load::Html("<h1>hi</h1>".to_string())));
    }

    #[test]
    fn piped_stdin_takes_precedence_over_path() {
        let got = resolve(false, Some(PathBuf::from("page.html")), || {
            Ok("<h1>piped</h1>".to_string())
        });
        assert_eq!(got, Ok(Load::Html("<h1>piped</h1>".to_string())));
    }

    #[test]
    fn tty_with_path_loads_the_file() {
        let got = resolve(true, Some(PathBuf::from("page.html")), || {
            panic!("stdin should not be read when it's a tty")
        });
        assert_eq!(got, Ok(Load::File(PathBuf::from("page.html"))));
    }

    #[test]
    fn empty_stdin_falls_through_to_path() {
        let got = resolve(
            false,
            Some(PathBuf::from("page.html")),
            || Ok(String::new()),
        );
        assert_eq!(got, Ok(Load::File(PathBuf::from("page.html"))));
    }

    #[test]
    fn empty_stdin_without_path_is_no_input() {
        let got = resolve(false, None, || Ok("   \n".to_string()));
        assert_eq!(got, Err(InputError::NoInput));
    }

    #[test]
    fn tty_without_path_is_no_input() {
        let got = resolve(true, None, || panic!("stdin should not be read"));
        assert_eq!(got, Err(InputError::NoInput));
    }

    #[test]
    fn stdin_read_failure_is_surfaced() {
        let got = resolve(false, None, || Err(io::Error::other("boom")));
        assert!(matches!(got, Err(InputError::StdinRead(_))));
    }
}
