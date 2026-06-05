//! webview — a webview CLI for agents and humans.
//!
//! Render the HTML the caller provides, give the page one channel to send a
//! result back (`window.webview.resolve` / `.reject`), print that result, and
//! exit. The Rust side never parses or validates the result — the page does
//! `JSON.stringify` and the binary prints the string verbatim.

mod assets;
mod bridge;
mod cli;
mod icon;
mod input;
mod run;

use cli::Cli;
use input::{resolve_from_env, InputError};
use run::exit;

fn main() {
    let args = Cli::parse_args();

    let load = match resolve_from_env(args.path.clone()) {
        Ok(load) => load,
        Err(InputError::NoInput) => {
            eprintln!(
                "webview: no HTML to render. Pass a file path or http(s) URL, or pipe HTML on stdin.\n\
                 Try 'webview --help' for usage."
            );
            std::process::exit(exit::USAGE);
        }
        Err(InputError::StdinRead(e)) => {
            eprintln!("webview: failed to read stdin: {e}");
            std::process::exit(exit::USAGE);
        }
    };

    run::run(&args, load);
}
