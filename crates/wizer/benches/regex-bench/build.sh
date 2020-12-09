#!/usr/bin/env bash

set -ex
cd "$(dirname $0)"
cargo build --target wasm32-wasi --release
cp ../../target/wasm32-wasi/release/regex_bench.wasm ../regex_bench.control.wasm
cargo build --target wasm32-wasi --release --features wizer
cd ../..
cargo run --all-features -- --allow-wasi target/wasm32-wasi/release/regex_bench.wasm -o benches/regex_bench.wizer.wasm
