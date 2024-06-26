#!/usr/bin/env bash
set -ex

# These flags reduce binary size by a combined 4.6k
export CARGO_PROFILE_RELEASE_LTO=fat
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="$RUSTFLAGS -Ctarget-feature=+bulk-memory"

build_adapter="cargo build -p wasi-preview1-component-adapter --target wasm32-unknown-unknown"
verify="cargo run -p verify-component-adapter --"

debug="target/wasm32-unknown-unknown/debug/wasi_snapshot_preview1.wasm"
release="target/wasm32-unknown-unknown/release/wasi_snapshot_preview1.wasm"

# Debug build, default features (reactor)
$build_adapter
$verify $debug

build() {
  input=$1
  flavor=$2
  $verify $input
  name=wasi_snapshot_preview1.$flavor.wasm
  dst=$(dirname $input)/$name
  provider=crates/wasi-preview1-component-adapter/provider/artefacts/$name
  wasm-tools metadata add --name "wasi_preview1_component_adapter.$flavor.adapter" $input \
    -o $dst
  cp $dst $provider
}

# Debug build, command
$build_adapter --no-default-features --features command
$verify $debug

# Release build, command
$build_adapter --release --no-default-features --features command
build $release command

# Release build, default features (reactor)
$build_adapter --release
build $release reactor

# Release build, proxy
$build_adapter --release --no-default-features --features proxy
build $release proxy

# Add the adapter provider to the workspace
cp crates/wasi-preview1-component-adapter/provider/Cargo.toml.in crates/wasi-preview1-component-adapter/provider/Cargo.toml
sed -i '/"crates\/wasi-preview1-component-adapter",/a\ \ "crates\/wasi-preview1-component-adapter\/provider",' Cargo.toml

cargo fmt -p wasi-preview1-component-adapter-provider -- --check
cargo check -p wasi-preview1-component-adapter-provider
cargo clippy -p wasi-preview1-component-adapter-provider
