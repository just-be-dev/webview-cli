# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-06-04

### Added

- Load remote pages by passing an `http://` or `https://` URL as the positional
  argument, alongside the existing file-path and piped-stdin inputs. URLs are
  detected case-insensitively; anything else is treated as a file. Piped stdin
  continues to take precedence over both.

## [0.1.0]

### Added

- Initial release: a one-shot webview CLI that renders HTML (from a file or
  piped on stdin), gives the page a `window.webview.resolve` / `.reject`
  bridge, prints the single result, and exits with a documented exit code.
- Window-shaping flags: `--title`, `--width`, `--height`, `--devtools`,
  `--icon` (macOS Dock icon), and `--timeout-ms`.

[0.2.0]: https://github.com/just-be-dev/webview-cli/releases/tag/v0.2.0
[0.1.0]: https://github.com/just-be-dev/webview-cli/releases/tag/v0.1.0
