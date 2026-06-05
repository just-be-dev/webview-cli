//! Serving file pages over a custom protocol.
//!
//! WKWebView won't load `file://` URLs without an explicit read-access grant
//! (it uses `loadRequest`, not `loadFileURL:allowingReadAccessToURL:`), so a
//! plain `file://` page silently never loads. Serving the page's directory
//! over a custom scheme sidesteps that *and* gives the page a real origin, so
//! relative assets and `fetch` resolve. This is the plan's M5, promoted to the
//! default path because file pages don't work without it.

use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

use wry::http::{header::CONTENT_TYPE, Request, Response};

/// Guess a content type from a file extension. A small, dependency-free table —
/// enough for the assets a one-shot page references.
pub fn content_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("html") | Some("htm") => "text/html",
        Some("css") => "text/css",
        Some("js") | Some("mjs") => "text/javascript",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        Some("wasm") => "application/wasm",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
}

/// Percent-decode a path. Minimal `%XX` handling so filenames with spaces and
/// other escaped characters resolve, without pulling in a dependency.
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Resolve a request path (e.g. `/sub/app.css`) to a file under `root`,
/// refusing anything that escapes `root`.
///
/// `root` must already be canonical. We reject `..` and absolute components
/// *before* touching the filesystem, then confirm the canonicalized result is
/// still under `root` (which also defeats symlink escapes).
pub fn resolve_under_root(root: &Path, req_path: &str) -> Option<PathBuf> {
    let rel = percent_decode(req_path.trim_start_matches('/'));
    let rel = if rel.is_empty() { "index.html" } else { &rel };

    let candidate = Path::new(rel);
    for comp in candidate.components() {
        match comp {
            Component::Normal(_) => {}
            // Anything that could climb out of root or re-anchor the path.
            _ => return None,
        }
    }

    let joined = root.join(candidate);
    let canonical = std::fs::canonicalize(&joined).ok()?;
    canonical.starts_with(root).then_some(canonical)
}

/// Serve one custom-protocol request out of `root`.
pub fn serve(root: &Path, request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    match resolve_under_root(root, request.uri().path()) {
        Some(file) => match std::fs::read(&file) {
            Ok(bytes) => Response::builder()
                .header(CONTENT_TYPE, content_type(&file))
                .body(Cow::Owned(bytes))
                .unwrap_or_else(|_| not_found()),
            Err(_) => not_found(),
        },
        None => not_found(),
    }
}

fn not_found() -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(404)
        .body(Cow::Borrowed(&b"not found"[..]))
        .expect("static 404 response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn content_type_is_case_insensitive() {
        assert_eq!(content_type(Path::new("a.CSS")), "text/css");
        assert_eq!(content_type(Path::new("a.Js")), "text/javascript");
        assert_eq!(content_type(Path::new("noext")), "application/octet-stream");
    }

    #[test]
    fn percent_decode_handles_spaces_and_passthrough() {
        assert_eq!(percent_decode("my%20file.html"), "my file.html");
        assert_eq!(percent_decode("plain.css"), "plain.css");
        // Malformed escapes are left untouched.
        assert_eq!(percent_decode("100%done"), "100%done");
    }

    #[test]
    fn traversal_attempts_are_rejected() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap();
        assert!(resolve_under_root(&root, "/../Cargo.toml").is_none());
        assert!(resolve_under_root(&root, "/../../etc/passwd").is_none());
    }

    #[test]
    fn real_file_under_root_resolves() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .canonicalize()
            .unwrap();
        let got = resolve_under_root(&root, "/resolve.html");
        assert_eq!(got, Some(root.join("resolve.html")));
    }
}
