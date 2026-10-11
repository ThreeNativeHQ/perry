#!/usr/bin/env bash
# Usage: scripts/prd582-runtime-wasm.sh
# Builds perry-runtime for wasm32-wasip1 as a reactor module and runs k_math against it from a
# second Wasm module that imports the runtime directly (no JS per value op). Needs the
# wasm32-wasip1 rust-std and clang with the wasm32 target.
set -euo pipefail
cd "$(dirname "$0")/.."
out=${CARGO_TARGET_DIR:-target}
cargo rustc -p perry-runtime --release --no-default-features --target wasm32-wasip1 \
    --crate-type cdylib -j 12 -- -C link-arg=--allow-undefined -C link-arg=--export-table -C link-arg=--growable-table -Z wasi-exec-model=reactor
rt="$out/wasm32-wasip1/release/perry_runtime.wasm"
tmp=$(mktemp -d "${TMPDIR:-/tmp}/prd582-runtime-wasm.XXXXXX")
clang --target=wasm32 -O2 -nostdlib -Wl,--no-entry -Wl,--allow-undefined \
    -o "$tmp/rt_math.wasm" scripts/prd582-runtime-wasm/rt_math.c
node --no-warnings scripts/prd582-runtime-wasm/run.mjs "$rt" "$tmp/rt_math.wasm"
