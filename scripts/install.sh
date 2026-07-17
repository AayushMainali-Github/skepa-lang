#!/usr/bin/env sh
set -eu

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required. Install Rust from https://rustup.rs/" >&2
  exit 1
fi

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"

echo "Installing skepac..."
cargo install --path "$ROOT_DIR/skepac"

echo "Building native runtime library..."
cargo build --release -p skepart --manifest-path "$ROOT_DIR/Cargo.toml"

if [ ! -f "$ROOT_DIR/target/release/libskepart.a" ]; then
  echo "expected $ROOT_DIR/target/release/libskepart.a after release build" >&2
  exit 1
fi

mkdir -p "$BIN_DIR"
cp "$ROOT_DIR/target/release/libskepart.a" "$BIN_DIR/"

echo "Done. Ensure $BIN_DIR is on PATH."
echo "build-native will find libskepart.a beside skepac (or via SKEPA_RUNTIME_DIR)."
