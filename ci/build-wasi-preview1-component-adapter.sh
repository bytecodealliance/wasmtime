#!/usr/bin/env bash
set -ex

build_adapter="cargo build -p wasi-preview1-component-adapter --target wasm32-unknown-unknown"
verify="cargo run -p verify-component-adapter --"

debug="target/wasm32-unknown-unknown/debug/wasi_snapshot_preview1.wasm"
release="target/wasm32-unknown-unknown/release/wasi_snapshot_preview1.wasm"

# The rust version that the adapter is built with is the crate's MSRV
RUST_VERSION=$( \
  grep '^rust-version\s*=' crates/wasi-preview1-component-adapter/Cargo.toml | \
  sed 's/rust-version.*=.*\"\(.*\)\"/\1/' \
)

rustup toolchain install $RUST_VERSION --profile minimal
rustup target add wasm32-wasi wasm32-unknown-unknown --toolchain $RUST_VERSION

export RUSTUP_TOOLCHAIN=$RUST_VERSION

cargo --version

# Debug build, default features (reactor)
$build_adapter
$verify $debug

# Debug build, command
$build_adapter --no-default-features --features command
$verify $debug

compare() {
  input=$1
  flavor=$2
  $verify $input
  name=wasi_snapshot_preview1.$flavor.wasm
  dst=$(dirname $input)/$name
  reference=crates/wasi-preview1-component-adapter/provider/artefacts/$name
  wasm-tools metadata add --name "wasi_preview1_component_adapter.$flavor.adapter" $input \
    -o $dst
  set +x
  if [ "$BLESS" = "1" ]; then
    cp $dst $reference
  elif ! cmp -s $dst $reference; then
    echo "Reference copy of adapter is not the same as the generated copy of"
    echo "the adapter"
    echo ""
    echo "  reference copy: $reference"
    echo "      built copy: $dst"
    echo ""
    echo "To automatically update the reference copy set \`BLESS=1\` in the"
    echo "environment"
    diff -u <(wasm-tools print $reference) <(wasm-tools print $dst)
    exit 1
  else
    echo "Reference copy of adapter matches local copy"
  fi
  set -x
}

# Release build, command
$build_adapter --release --no-default-features --features command
compare $release command

# Release build, default features (reactor)
$build_adapter --release
compare $release reactor

# Release build, proxy
$build_adapter --release --no-default-features --features proxy
compare $release proxy
