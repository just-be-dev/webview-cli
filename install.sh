#!/bin/sh
# webview-cli installer.
#
# Downloads the prebuilt `webview` binary for your platform from a GitHub
# release and installs it into a bin directory on your PATH.
#
#   curl -fsSL https://raw.githubusercontent.com/just-be-dev/webview-cli/main/install.sh | sh
#
# Environment overrides:
#   VERSION      release tag to install (default: latest, e.g. v0.1.0)
#   INSTALL_DIR  where to put the binary (default: ~/.local/bin)
set -eu

REPO="just-be-dev/webview-cli"
BIN="webview"
VERSION="${VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

err() {
	printf 'install: %s\n' "$1" >&2
	exit 1
}

need() {
	command -v "$1" >/dev/null 2>&1 || err "required command not found: $1"
}

# Pick a downloader.
if command -v curl >/dev/null 2>&1; then
	dl() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
	dl() { wget -qO "$2" "$1"; }
else
	err "need curl or wget to download"
fi

# Map uname output to the release asset platform suffix.
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
	Darwin) os_name="macos" ;;
	Linux) os_name="linux" ;;
	*) err "unsupported OS: $os (Windows users: download from the Releases page)" ;;
esac
case "$arch" in
	arm64 | aarch64) arch_name="arm64" ;;
	x86_64 | amd64) arch_name="x64" ;;
	*) err "unsupported architecture: $arch" ;;
esac
platform="${os_name}-${arch_name}"
asset="${BIN}-${platform}"

# Resolve the download base for the requested version.
if [ "$VERSION" = "latest" ]; then
	base="https://github.com/${REPO}/releases/latest/download"
else
	base="https://github.com/${REPO}/releases/download/${VERSION}"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

printf 'install: downloading %s (%s)...\n' "$asset" "$VERSION" >&2
dl "${base}/${asset}" "${tmp}/${BIN}" || err "download failed: ${base}/${asset}"

# Verify against SHA256SUMS if we can fetch it and have a checksum tool.
if dl "${base}/SHA256SUMS" "${tmp}/SHA256SUMS" 2>/dev/null; then
	if command -v sha256sum >/dev/null 2>&1; then
		sum="$(sha256sum "${tmp}/${BIN}" | awk '{print $1}')"
	elif command -v shasum >/dev/null 2>&1; then
		sum="$(shasum -a 256 "${tmp}/${BIN}" | awk '{print $1}')"
	else
		sum=""
	fi
	if [ -n "$sum" ]; then
		want="$(grep " ${asset}\$" "${tmp}/SHA256SUMS" | awk '{print $1}')"
		if [ -n "$want" ] && [ "$sum" != "$want" ]; then
			err "checksum mismatch for ${asset} (expected ${want}, got ${sum})"
		fi
		printf 'install: checksum verified\n' >&2
	fi
fi

mkdir -p "$INSTALL_DIR"
chmod +x "${tmp}/${BIN}"
mv "${tmp}/${BIN}" "${INSTALL_DIR}/${BIN}"
printf 'install: installed %s to %s\n' "$BIN" "${INSTALL_DIR}/${BIN}" >&2

case ":${PATH}:" in
	*":${INSTALL_DIR}:"*) ;;
	*) printf 'install: note: %s is not on your PATH; add it to use `%s` directly.\n' "$INSTALL_DIR" "$BIN" >&2 ;;
esac
