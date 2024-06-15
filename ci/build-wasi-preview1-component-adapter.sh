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

# The adapter's version is the hash of the last commit that touched the adapter
# source files
VERSION=$(git log -1 --pretty="format:%H" -- $( \
  git ls-files -- \
    'crates/wasi-preview1-component-adapter/**' \
    ':!:crates/wasi-preview1-component-adapter/provider/**' \
))

compare() {
  input=$1
  flavor=$2
  $verify $input
  name=wasi_snapshot_preview1.$flavor.wasm
  dst=$(dirname $input)/$name
  reference=crates/wasi-preview1-component-adapter/provider/artefacts/$name
  wasm-tools metadata add --name "wasi_preview1_component_adapter.$flavor.adapter:${VERSION}" $input \
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
