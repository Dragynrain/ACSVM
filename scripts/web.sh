#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Builds or serves the browser viewer with trunk (https://trunk-rs.github.io).
#
#   scripts/web.sh serve     # dev server on http://127.0.0.1:8080 with reload
#   scripts/web.sh build     # optimized static site in target/web/
#
# Needs: rustup target add wasm32-unknown-unknown; cargo install trunk --locked
# trunk fetches the matching wasm-bindgen and wasm-opt itself on first use.

set -eu
cd "$(dirname "$0")/../crates/acs-viewer"

command -v trunk >/dev/null 2>&1 || {
  echo "trunk not found: cargo install trunk --locked" >&2
  exit 1
}

case "${1:-serve}" in
  serve) exec trunk serve ;;
  build) exec trunk build --release ;;
  *) echo "usage: $0 [serve|build]" >&2; exit 2 ;;
esac
