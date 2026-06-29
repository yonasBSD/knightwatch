#!/usr/bin/env bash
# Installer for knightwatch.
# Usage:
#   curl -LsSf https://github.com/YofaGh/knightwatch/releases/latest/download/install.sh | sh
#   curl -LsSf .../install.sh | sh -s -- --version 1.0.17
set -euo pipefail

REPO="YofaGh/knightwatch"
BIN_NAME="knightwatch"
VERSION="latest"
INSTALL_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"

while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --install-dir) INSTALL_DIR="$2"; shift 2 ;;
    *) echo "Unknown argument: $1" >&2; exit 1 ;;
  esac
done

uname_os="$(uname -s)"
uname_arch="$(uname -m)"

case "$uname_os" in
  Linux) os="unknown-linux-gnu" ;;
  Darwin) os="apple-darwin" ;;
  *) echo "Unsupported OS: $uname_os" >&2; exit 1 ;;
esac

case "$uname_arch" in
  x86_64|amd64) arch="x86_64" ;;
  arm64|aarch64) arch="aarch64" ;;
  *) echo "Unsupported architecture: $uname_arch" >&2; exit 1 ;;
esac

target="${arch}-${os}"
archive="${BIN_NAME}-${target}.tar.gz"

if [ "$VERSION" = "latest" ]; then
  url="https://github.com/${REPO}/releases/latest/download/${archive}"
else
  url="https://github.com/${REPO}/releases/download/${VERSION}/${archive}"
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

echo "Downloading ${url}"
curl --proto '=https' --tlsv1.2 -LsSf "$url" -o "${tmpdir}/${archive}"

tar -xzf "${tmpdir}/${archive}" -C "$tmpdir"

stage_dir="${tmpdir}/${BIN_NAME}-${target}"
mkdir -p "$INSTALL_DIR"
install -m 755 "${stage_dir}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"

echo "Installed ${BIN_NAME} to ${INSTALL_DIR}/${BIN_NAME}"
case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *) echo "Note: ${INSTALL_DIR} is not on your PATH. Add it to your shell profile." ;;
esac
