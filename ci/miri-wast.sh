#!/bin/bash

# Helper script to execute a `*.wast` test in Miri. This is only lightly used on
# CI and is provided here to assist with development of anything that ends up
# using unsafe for example.
#
# Example usage is:
#
#   ./ci/miri-wast.sh ./tests/spec_testsuite/br_if.wast
#
# extra flags to this script are passed to `cargo run wast` which means they
# must be suitable flags for the `wast` subcommand.

set -ex

REPO="$(dirname $0)/.."
CARGO_TOML="$REPO/Cargo.toml"
MIRI_WAST="$REPO/target/miri-wast"

rm -rf "$MIRI_WAST"
mkdir -p "$MIRI_WAST"

cargo run --manifest-path "$CARGO_TOML" -- wast --target pulley64 --precompile-save "$MIRI_WAST" "$@" \
  -O memory-reservation=$((1 << 20)) \
  -O memory-guard-size=0 \
  -O signals-based-traps=n \
  -O memory-init-cow=n \
  -W function-references,gc

MIRIFLAGS="$MIRIFLAGS -Zmiri-disable-isolation -Zmiri-permissive-provenance" \
cargo miri run --manifest-path "$CARGO_TOML" -- \
  wast -Ccache=n --target pulley64 --precompile-load "$MIRI_WAST" "$@" \
  -O memory-init-cow=n \
  -W function-references,gc \
  --ignore-error-messages
