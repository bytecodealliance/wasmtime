#! /bin/bash

set -e

RUST_BINDINGS="crates/wasi-crypto/spec/implementations/bindings/rust"
pushd "$RUST_BINDINGS"
cargo build --release --target=wasm32-wasi
popd

cargo run --features wasi-crypto -- run "$RUST_BINDINGS/target/wasm32-wasi/release/wasi-crypto-guest.wasm" --wasi-modules=experimental-wasi-crypto
