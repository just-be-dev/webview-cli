//! The result bridge.
//!
//! The whole protocol is: the page calls `window.webview.resolve(v)` or
//! `window.webview.reject(e)`, which posts a single tagged string over the
//! IPC channel. The Rust side never parses JSON — it splits the tag off the
//! front and passes the rest through verbatim. The result's *shape* is the
//! caller's concern.

/// Injected before page load. Defines the one channel the page talks back on.
///
/// `resolve` stringifies its argument (so the binary prints valid JSON, with
/// `undefined`/no-arg becoming `null`); `reject` coerces to a string. The
/// `"ok:"` / `"err:"` prefixes are the entire wire format.
pub const BRIDGE: &str = r#"
  window.webview = {
    resolve: (v) => window.ipc.postMessage("ok:"  + JSON.stringify(v ?? null)),
    reject:  (e) => window.ipc.postMessage("err:" + String(e)),
  };
"#;

/// What the event loop reacts to. `Resolve`/`Reject` carry the page's payload
/// verbatim; `Timeout` and the window-close case carry nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    Resolve(String),
    Reject(String),
    Timeout,
}

/// Parse one IPC message body into an event.
///
/// Returns `None` for anything that isn't a recognized `ok:`/`err:` message so
/// the IPC handler can simply ignore stray traffic. The first recognized
/// message wins; the caller exits immediately after.
pub fn parse_message(body: &str) -> Option<AppEvent> {
    match body.split_once(':') {
        Some(("ok", rest)) => Some(AppEvent::Resolve(rest.to_string())),
        Some(("err", rest)) => Some(AppEvent::Reject(rest.to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_resolve() {
        assert_eq!(
            parse_message("ok:{\"ok\":1}"),
            Some(AppEvent::Resolve("{\"ok\":1}".to_string()))
        );
    }

    #[test]
    fn parses_reject() {
        assert_eq!(
            parse_message("err:nope"),
            Some(AppEvent::Reject("nope".to_string()))
        );
    }

    #[test]
    fn resolve_payload_may_contain_colons() {
        // split_once stops at the first colon, so JSON with colons is intact.
        assert_eq!(
            parse_message("ok:{\"a\":\"b:c\"}"),
            Some(AppEvent::Resolve("{\"a\":\"b:c\"}".to_string()))
        );
    }

    #[test]
    fn empty_resolve_is_a_resolve_with_empty_payload() {
        assert_eq!(parse_message("ok:"), Some(AppEvent::Resolve(String::new())));
    }

    #[test]
    fn unrecognized_messages_are_ignored() {
        assert_eq!(parse_message("garbage"), None);
        assert_eq!(parse_message("info:something"), None);
        assert_eq!(parse_message(""), None);
    }
}
