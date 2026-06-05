//! Integration tests: spawn the built binary and assert on stdout/stderr/exit.
//!
//! The exit-code contract is the public API, so we test it end-to-end rather
//! than through internals. Tests that actually launch a window need a display;
//! those skip cleanly on a headless box (a Linux CI runner without `xvfb`).
//! The usage test needs no window and runs everywhere.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Path to the binary cargo built for this test run.
const BIN: &str = env!("CARGO_BIN_EXE_webview");

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Can we open a window here? macOS runners can; a Linux box needs a display
/// server (real, or `xvfb` exporting `DISPLAY`).
fn has_display() -> bool {
    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
        return true;
    }
    std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

fn skip_unless_display(test: &str) -> bool {
    if has_display() {
        return false;
    }
    eprintln!("skipping {test}: no display available (set DISPLAY or run under xvfb)");
    true
}

#[test]
fn resolve_prints_json_and_exits_zero() {
    if skip_unless_display("resolve_prints_json_and_exits_zero") {
        return;
    }
    let out = Command::new(BIN)
        .arg(fixture("resolve.html"))
        .arg("--timeout-ms")
        .arg("10000")
        .output()
        .expect("spawn webview");

    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "{\"ok\":1}");
}

#[test]
fn reject_prints_message_to_stderr_and_exits_one() {
    if skip_unless_display("reject_prints_message_to_stderr_and_exits_one") {
        return;
    }
    let out = Command::new(BIN)
        .arg(fixture("reject.html"))
        .arg("--timeout-ms")
        .arg("10000")
        .output()
        .expect("spawn webview");

    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty(), "stdout should be empty on reject");
    assert_eq!(String::from_utf8_lossy(&out.stderr).trim(), "nope");
}

#[test]
fn timeout_exits_three() {
    if skip_unless_display("timeout_exits_three") {
        return;
    }
    let out = Command::new(BIN)
        .arg(fixture("never.html"))
        .arg("--timeout-ms")
        .arg("400")
        .output()
        .expect("spawn webview");

    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty());
}

#[test]
fn resolve_via_stdin_works() {
    if skip_unless_display("resolve_via_stdin_works") {
        return;
    }
    let mut child = Command::new(BIN)
        .arg("--timeout-ms")
        .arg("10000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn webview");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"<script>window.webview.resolve(['a','b'])</script>")
        .unwrap();

    let out = child.wait_with_output().expect("wait");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "[\"a\",\"b\"]");
}

#[test]
fn no_input_exits_sixty_four() {
    // No window needed: empty stdin + no path -> usage error.
    let out = Command::new(BIN)
        .stdin(Stdio::null())
        .output()
        .expect("spawn webview");

    assert_eq!(out.status.code(), Some(64));
    assert!(out.stdout.is_empty());
    assert!(!out.stderr.is_empty(), "usage message expected on stderr");
}
