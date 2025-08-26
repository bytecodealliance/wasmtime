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

rm -rf ./miri-wast
mkdir ./miri-wast
cargo run -- wast --target pulley64 --precompile-save ./miri-wast "$@" \
  -O memory-reservation=$((1 << 20)) \
  -O memory-guard-size=0 \
  -O signals-based-traps=n \
  -O memory-init-cow=n

MIRIFLAGS="$MIRIFLAGS -Zmiri-disable-isolation -Zmiri-permissive-provenance" \
  cargo miri run -- wast -Ccache=n --target pulley64 --precompile-load ./miri-wast "$@" \
  -O memory-init-cow=n
