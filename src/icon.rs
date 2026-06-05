//! Setting the app's Dock icon at runtime (macOS only).
//!
//! A CLI-launched webview shows a generic Dock icon. macOS reads that icon from
//! the running `NSApplication`, so `--icon` swaps it by handing AppKit an
//! `NSImage` loaded from the given path. The icon is cosmetic, so a bad path
//! warns rather than failing the run. Other platforms have no equivalent
//! runtime hook here, so this is a no-op (with a one-line note).

use std::path::Path;

#[cfg(target_os = "macos")]
pub fn set_app_icon(path: &Path) {
    use objc2::{AnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSString;

    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("webview: --icon ignored (not on the main thread)");
        return;
    };

    let ns_path = NSString::from_str(&path.to_string_lossy());
    match NSImage::initWithContentsOfFile(NSImage::alloc(), &ns_path) {
        // SAFETY: standard AppKit call, on the main thread (proven by `mtm`).
        Some(image) => unsafe {
            NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image));
        },
        None => eprintln!("webview: could not load icon from {}", path.display()),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_app_icon(_path: &Path) {
    eprintln!("webview: --icon is only supported on macOS; ignoring");
}
