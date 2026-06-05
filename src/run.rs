//! Window + webview construction and the event loop.
//!
//! `EventLoop::run` never returns, so the clean way out of a CLI is to call
//! `std::process::exit` from inside the loop the moment an `AppEvent` arrives —
//! no `ControlFlow::Exit` / `run_return` juggling. The first event wins.

use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

use crate::assets;
use crate::bridge::{parse_message, AppEvent, BRIDGE};
use crate::cli::Cli;
use crate::input::Load;

/// The custom scheme file pages are served under (see `assets`).
const FILE_SCHEME: &str = "wv";

/// Exit codes — this table *is* the public API (see README).
pub mod exit {
    /// page called `resolve(v)` — stdout carries the JSON.
    pub const RESOLVE: i32 = 0;
    /// page called `reject(e)` — stderr carries the message.
    pub const REJECT: i32 = 1;
    /// user closed the window before the page settled.
    pub const CLOSED: i32 = 2;
    /// `--timeout-ms` elapsed before the page settled.
    pub const TIMEOUT: i32 = 3;
    /// bad usage (no input, etc.).
    pub const USAGE: i32 = 64;
}

/// Build the window + webview and run the event loop. Never returns: it exits
/// the process from inside the loop.
pub fn run(cli: &Cli, load: Load) -> ! {
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // The event loop has created the NSApplication by now, so the Dock icon
    // can be swapped before the window appears.
    if let Some(icon) = &cli.icon {
        crate::icon::set_app_icon(icon);
    }

    let window = WindowBuilder::new()
        .with_title(&cli.title)
        .with_inner_size(LogicalSize::new(cli.width, cli.height))
        .build(&event_loop)
        .expect("failed to create window");

    let ipc_proxy = proxy.clone();
    let mut builder = WebViewBuilder::new()
        .with_initialization_script(BRIDGE)
        .with_devtools(cli.devtools)
        .with_ipc_handler(move |req| {
            // `req.body()` is the verbatim string the page posted. We split the
            // tag off the front and forward the payload untouched.
            if let Some(ev) = parse_message(req.body().as_str()) {
                let _ = ipc_proxy.send_event(ev);
            }
        });

    builder = match &load {
        Load::Html(html) => builder.with_html(html),
        Load::Url(url) => builder.with_url(url),
        Load::File(path) => {
            // Serve the page's directory over a custom scheme so it loads at
            // all (WKWebView) and gets a real origin for relative assets.
            let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
            let root = canonical
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("/"));
            let filename = canonical
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("index.html")
                .to_string();

            let serve_root = root.clone();
            builder
                .with_custom_protocol(FILE_SCHEME.to_string(), move |_id, request| {
                    assets::serve(&serve_root, &request)
                })
                .with_url(format!(
                    "{FILE_SCHEME}://localhost/{}",
                    url_encode(&filename)
                ))
        }
    };

    let _webview = builder.build(&window).expect("failed to create webview");

    // Optional timeout: a thread that sleeps, then nudges the loop. The first
    // event (resolve/reject/close) still wins if it arrives first.
    if let Some(ms) = cli.timeout_ms {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(ms));
            let _ = proxy.send_event(AppEvent::Timeout);
        });
    }

    event_loop.run(move |event, _, control_flow| {
        // Keep the webview alive for the lifetime of the loop.
        let _ = &_webview;
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(AppEvent::Resolve(v)) => {
                println!("{v}");
                std::process::exit(exit::RESOLVE);
            }
            Event::UserEvent(AppEvent::Reject(e)) => {
                eprintln!("{e}");
                std::process::exit(exit::REJECT);
            }
            Event::UserEvent(AppEvent::Timeout) => {
                std::process::exit(exit::TIMEOUT);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                std::process::exit(exit::CLOSED);
            }
            _ => {}
        }
    });
}

/// Percent-encode the characters in a single path segment that would otherwise
/// break a URL. Minimal — filenames rarely need more than space-escaping.
fn url_encode(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for b in segment.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
