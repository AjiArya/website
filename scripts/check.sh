#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test

if command -v node >/dev/null 2>&1; then
  node --check static/js/site.js
  node --check static/js/diagram.js
fi

cargo run -- build
