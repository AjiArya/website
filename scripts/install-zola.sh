#!/usr/bin/env bash
# Install a pinned Zola binary into .tools/zola for reproducible local builds.
set -euo pipefail

VERSION="${ZOLA_VERSION:-0.23.6}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/.tools/zola"

if [[ -x "$BIN" ]] && "$BIN" --version 2>/dev/null | grep -q " $VERSION"; then
  echo "Zola $VERSION already installed at $BIN"
  exit 0
fi

mkdir -p "$ROOT/.tools"

case "$(uname -s):$(uname -m)" in
  Linux:x86_64)
    target="x86_64-unknown-linux-gnu"
    expected_sha="8f5132b3522412d04e395e0b25f6d68613ad272a873e54a2b3ebf664873024a4"
    ;;
  Linux:aarch64 | Linux:arm64)
    target="aarch64-unknown-linux-gnu"
    expected_sha="266448fffbf7c7004ca399d0e76dd699541771096d8a42aede98cebe2a029d02"
    ;;
  Darwin:x86_64)
    target="x86_64-apple-darwin"
    expected_sha="79a4d0ab51a4d863c068e6e594c6fce36f0aa17429a414ea63066f5910d14460"
    ;;
  Darwin:arm64)
    target="aarch64-apple-darwin"
    expected_sha="cbffbd29b3f59c3f52633507c8cb945a7a02d8b1399b43b235f5932912297aa3"
    ;;
  *)
    echo "Unsupported platform: $(uname -s) $(uname -m)" >&2
    echo "Install Zola $VERSION manually and put it on PATH, or set ZOLA_BIN." >&2
    exit 1
    ;;
esac

url="https://github.com/getzola/zola/releases/download/v${VERSION}/zola-v${VERSION}-${target}.tar.gz"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

echo "Downloading Zola $VERSION for $target..."
curl -fsSL "$url" -o "$tmp"

expected_sha="${ZOLA_SHA256:-$expected_sha}"
if command -v sha256sum >/dev/null 2>&1; then
  actual_sha="$(sha256sum "$tmp" | awk '{print $1}')"
else
  actual_sha="$(shasum -a 256 "$tmp" | awk '{print $1}')"
fi
if [[ "$actual_sha" != "$expected_sha" ]]; then
  echo "Checksum mismatch for $url" >&2
  echo "expected: $expected_sha" >&2
  echo "actual:   $actual_sha" >&2
  exit 1
fi

tar -xzf "$tmp" -C "$ROOT/.tools"
chmod +x "$BIN"
"$BIN" --version
