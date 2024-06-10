#!/usr/bin/env bash

# Usage:
#
#     wasmtime-serve-rps.sh [WASMTIME-FLAGS] path/to/wasi-http-component.wasm
#
# For a basic WASI HTTP component, check out
# https://github.com/sunfishcode/hello-wasi-http
#
# You must have the `hey` tool installed on your `$PATH`. It is available in at
# least the `apt` and `brew` package managers, as well as a binary download via
# its github page: https://github.com/rakyll/hey

set -e

repo_dir="$(dirname $0)/.."
cargo_toml="$repo_dir/Cargo.toml"
target_dir="$CARGO_TARGET_DIR"
if [[ "$target_dir" == "" ]]; then
    target_dir="$repo_dir/target"
fi

# Build Wasmtime.
cargo build --manifest-path "$cargo_toml" --release -p wasmtime-cli

# Spawn `wasmtime serve` in the background.
"$target_dir/release/wasmtime" serve "$@" &
pid=$!

# Give it a second to print its diagnostic information and get the server up and
# running.
sleep 1

echo 'Running `wasmtime serve` in background as pid '"$pid"

# Benchmark the server!
echo "Benchmarking for 10 seconds..."
hey -z 10s http://0.0.0.0:8080/

kill "$pid"
