#!/bin/sh
# Builds the FreeCandy7 plug-in (every effect in one module) to dist/freecandy7.wasm, ready for
# Edit > Preferences > Plug-ins. Needs the wasm32-unknown-unknown target:
# `rustup target add wasm32-unknown-unknown`.
set -eu
cd "$(dirname "$0")"
# Keep compiler diagnostic paths independent of the build machine.
RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$HOME=FreeCandy7" \
    cargo build --release --target wasm32-unknown-unknown
mkdir -p dist
cp target/wasm32-unknown-unknown/release/freecandy7.wasm dist/freecandy7.wasm
echo "built dist/freecandy7.wasm ($(wc -c < dist/freecandy7.wasm) bytes)"
