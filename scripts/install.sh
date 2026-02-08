#!/usr/bin/env sh
set -eu

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required. Install Rust from https://rustup.rs/" >&2
  exit 1
fi

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

echo "Installing skepac..."
cargo install --path "$ROOT_DIR/skepac"

echo "Installing skeparun..."
cargo install --path "$ROOT_DIR/skeparun"

echo "Done. Ensure ~/.cargo/bin is on PATH."
