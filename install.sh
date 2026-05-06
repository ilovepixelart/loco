#!/usr/bin/env sh
# loco installer — fetches the latest release binary for your platform.
#
#   curl -sSfL https://raw.githubusercontent.com/ilovepixelart/loco/main/install.sh | sh
#
# Override install dir:    LOCO_INSTALL_DIR=~/.local/bin sh install.sh
# Pin to a specific tag:   LOCO_VERSION=v1.0.0 sh install.sh

set -eu

REPO="ilovepixelart/loco"
INSTALL_DIR="${LOCO_INSTALL_DIR:-/usr/local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin)
    case "$arch" in
      arm64)  target="aarch64-apple-darwin" ;;
      x86_64) target="x86_64-apple-darwin" ;;
      *) echo "loco: unsupported macOS arch: $arch" >&2; exit 1 ;;
    esac
    ;;
  Linux)
    case "$arch" in
      aarch64|arm64) target="aarch64-unknown-linux-musl" ;;
      x86_64)        target="x86_64-unknown-linux-musl" ;;
      *) echo "loco: unsupported Linux arch: $arch" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "loco: unsupported OS: $os" >&2
    echo "loco: for Windows, download the binary directly from https://github.com/$REPO/releases/latest" >&2
    exit 1
    ;;
esac

if [ -n "${LOCO_VERSION:-}" ]; then
  tag="$LOCO_VERSION"
else
  tag="$(curl -fsS "https://api.github.com/repos/$REPO/releases/latest" \
         | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)"
  if [ -z "${tag:-}" ]; then
    echo "loco: could not resolve latest release tag" >&2
    exit 1
  fi
fi

url="https://github.com/$REPO/releases/download/$tag/loco-$target"

echo "loco: downloading $tag ($target)"
tmp="$(mktemp)"
curl -fsSL "$url" -o "$tmp"
chmod +x "$tmp"

if [ -w "$INSTALL_DIR" ]; then
  mv "$tmp" "$INSTALL_DIR/loco"
else
  echo "loco: $INSTALL_DIR is not writable — using sudo"
  sudo mv "$tmp" "$INSTALL_DIR/loco"
fi

echo "loco: installed to $INSTALL_DIR/loco"
"$INSTALL_DIR/loco" --help >/dev/null && echo "loco: verified."
