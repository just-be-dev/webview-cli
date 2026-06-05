# webview

A webview CLI. It opens a native window, renders the HTML you give it,
gives the page **one** channel to send a result back, prints that result, and
exits. That's the whole tool.

Any caller — a shell, an agent, Python, Node — integrates the same way: spawn
the process, write HTML, read the result off stdout.

The page does `JSON.stringify`; the binary prints that string **verbatim**. The
Rust side never parses or validates the result — its shape is entirely the
caller's concern.

## Install

**Install script** (macOS / Linux) — grabs the right prebuilt binary for your
platform from the latest GitHub release:

```bash
curl -fsSL https://raw.githubusercontent.com/just-be-dev/webview-cli/main/install.sh | sh
```

**Prebuilt binaries** — download from the
[Releases page](https://github.com/just-be-dev/webview-cli/releases). Each
release ships `webview-<platform>` for `macos-arm64`, `linux-x64`,
`linux-arm64`, and `windows-x64`, alongside `SHA256SUMS`.

**Cargo** — if you have a Rust toolchain:

```bash
cargo install webview-cli      # installs the `webview` binary
```

**From source:**

```bash
mise run build                 # -> target/release/webview
# or
cargo build --release
```

On Linux you need the WebKitGTK headers to build (or to run the binary):
`libwebkit2gtk-4.1-dev libgtk-3-dev`.

## Usage

```bash
echo '<html>…</html>' | webview            # HTML piped on stdin
webview ./page.html                         # or a file path
webview ./page.html --title T --width 900 --height 700 --devtools --icon ./icon.png --timeout-ms 60000
```

Input precedence: non-empty piped stdin wins; otherwise the path argument;
otherwise it's a usage error. File pages are served over a custom origin so
relative CSS/JS/images and `fetch` resolve (and so they load at all under
WKWebView).

### Flags

| Flag           | Default   | Meaning                                      |
| -------------- | --------- | -------------------------------------------- |
| `--title`      | `webview` | Window title.                                |
| `--width`      | `800`     | Window width (logical px).                   |
| `--height`     | `600`     | Window height (logical px).                  |
| `--devtools`   | off       | Open dev tools on launch.                    |
| `--icon`       | none      | Image to show as the Dock icon (macOS only). |
| `--timeout-ms` | none      | Exit `3` if the page hasn't settled in time. |

Everything else lives in the HTML — there are no other flags by design.

## The bridge

The page talks back through one injected object:

```js
window.webview.resolve(value); // any JSON-serializable value
window.webview.reject(error); // string or Error
```

The first `resolve`/`reject` wins; the process exits immediately after.

## Exit codes — this table _is_ the public API

| Outcome                    | stdout              | stderr  | exit |
| -------------------------- | ------------------- | ------- | ---- |
| page called `resolve(v)`   | `JSON.stringify(v)` | —       | 0    |
| page called `reject(e)`    | —                   | message | 1    |
| user closed window first   | (empty)             | —       | 2    |
| `--timeout-ms` elapsed     | (empty)             | —       | 3    |
| bad usage (no input, etc.) | (empty)             | usage   | 64   |

## End-to-end example

A tiny confirmation prompt that returns `true`/`false`:

```bash
cat <<'HTML' | webview --title "Confirm" --width 360 --height 160
<!doctype html>
<body style="font:14px system-ui;display:grid;place-content:center;gap:12px">
  <p>Delete everything?</p>
  <div>
    <button onclick="webview.resolve(true)">Yes</button>
    <button onclick="webview.resolve(false)">No</button>
  </div>
</body>
HTML
# prints `true` or `false`; exit 0
```

## From an agent — the same call, three ways

No SDK: spawn the binary, write HTML to stdin, read stdout.

**Shell**

```bash
result=$(echo "$html" | webview --timeout-ms 60000)
```

**Python**

```python
import subprocess
out = subprocess.run(["webview", "--timeout-ms", "60000"],
                     input=html, capture_output=True, text=True)
result = out.stdout                     # JSON string; parse if you want
```

**Node**

```js
import { spawnSync } from "node:child_process";
const out = spawnSync("webview", ["--timeout-ms", "60000"], { input: html });
const result = out.stdout.toString(); // JSON string
```

## The one boundary

`webview` does "show something, get one answer back" — it isn't a live,
two-way session. When you need multiple steps, put them all in one page (a
wizard, several screens in one document) and only call `resolve` at the very
end. Keep the interaction in the HTML.

## Development

```bash
mise run test          # cargo test (window-launch tests skip without a display)
mise run lint          # cargo clippy -D warnings
mise run format        # cargo fmt
mise run typecheck     # cargo check --all-targets
mise run check         # all of the above
```

Source layout:

| File        | Concern                                                   |
| ----------- | --------------------------------------------------------- |
| `main.rs`   | orchestration: parse args, resolve input, run             |
| `cli.rs`    | clap arg struct + usage                                   |
| `input.rs`  | stdin-vs-path resolution → a `Load` enum                  |
| `bridge.rs` | the `BRIDGE` JS + `AppEvent` + message parsing            |
| `assets.rs` | custom-protocol file server (MIME, path-traversal safety) |
| `icon.rs`   | runtime Dock-icon swap for `--icon` (macOS; no-op else)   |
| `run.rs`    | window + webview build, event loop, exit codes            |

## Releasing

Pushing a `v*` tag (e.g. `v0.1.0`) triggers the release workflow, which builds
all four platform binaries, attaches them plus `SHA256SUMS` to a GitHub
Release, and publishes the crate to crates.io. See
[`.github/workflows/release.yml`](.github/workflows/release.yml).

## License

[MIT](LICENSE) © Justin Bennett
