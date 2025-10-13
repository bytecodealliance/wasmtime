#!/usr/bin/env bash

set -ex
cd "$(dirname $0)"
cargo build --target wasm32-wasi --release
cp ../../target/wasm32-wasi/release/uap_bench.wasm ../uap_bench.control.wasm
cargo build --target wasm32-wasi --release --features wizer
cd ../..
cargo run --all-features -- --allow-wasi target/wasm32-wasi/release/uap_bench.wasm -o benches/uap_bench.wizer.wasm
