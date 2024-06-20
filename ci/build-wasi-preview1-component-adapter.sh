#!/usr/bin/env bash
set -ex

build_adapter="cargo build -p wasi-preview1-component-adapter --target wasm32-unknown-unknown"
verify="cargo run -p verify-component-adapter --"

debug="target/wasm32-unknown-unknown/debug/wasi_snapshot_preview1.wasm"
release="target/wasm32-unknown-unknown/release/wasi_snapshot_preview1.wasm"

# Debug build, default features (reactor)
$build_adapter
$verify $debug

# Debug build, command
$build_adapter --no-default-features --features command
$verify $debug

# Release build, command
$build_adapter --release --no-default-features --features command
$verify $release
wasm-tools metadata add --name "wasi_preview1_component_adapter.command.adapter:${VERSION}" $release \
  -o target/wasm32-unknown-unknown/release/wasi_snapshot_preview1.command.wasm

# Release build, default features (reactor)
$build_adapter --release
$verify $release
wasm-tools metadata add --name "wasi_preview1_component_adapter.reactor.adapter:${VERSION}" $release \
  -o target/wasm32-unknown-unknown/release/wasi_snapshot_preview1.reactor.wasm

# Release build, proxy
$build_adapter --release --no-default-features --features proxy
$verify $release
wasm-tools metadata add --name "wasi_preview1_component_adapter.proxy.adapter:${VERSION}" $release \
  -o target/wasm32-unknown-unknown/release/wasi_snapshot_preview1.proxy.wasm
