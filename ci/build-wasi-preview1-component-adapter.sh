#!/usr/bin/env bash
set -ex

BASEDIR=$(dirname "$0")
cd ${BASEDIR}/../crates/wasi-preview1-component-adapter/

# Debug build, default features (reactor)
cargo build --target wasm32-unknown-unknown
cargo run -p verify-component-adapter -- target/wasm32-unknown-unknown/debug/wasi_preview1_component_adapter.wasm

# Debug build, command
cargo build --target wasm32-unknown-unknown --no-default-features --features command
cargo run -p verify-component-adapter -- target/wasm32-unknown-unknown/debug/wasi_preview1_component_adapter.wasm

# Release build, command
cargo build --target wasm32-unknown-unknown --release --no-default-features --features command
cargo run -p verify-component-adapter -- target/wasm32-unknown-unknown/release/wasi_preview1_component_adapter.wasm
wasm-tools metadata add --name "wasi_preview1_component_adapter.command.adapter:${VERSION}" target/wasm32-unknown-unknown/release/wasi_preview1_component_adapter.wasm -o target/wasm32-unknown-unknown/release/wasi_preview1_component_adapter.command.wasm

# Release build, default features (reactor)
cargo build --target wasm32-unknown-unknown --release
cargo run -p verify-component-adapter -- ./target/wasm32-unknown-unknown/release/wasi_preview1_component_adapter.wasm
wasm-tools metadata add --name "wasi_preview1_component_adapter.reactor.adapter:${VERSION}" target/wasm32-unknown-unknown/release/wasi_preview1_component_adapter.wasm -o target/wasm32-unknown-unknown/release/wasi_preview1_component_adapter.reactor.wasm
