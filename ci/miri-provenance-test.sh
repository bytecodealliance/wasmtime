#!/bin/bash

# This is a small script to assist in running the `pulley_provenance_test` test
# located at `tests/all/pulley.rs`. The goal of this script is to use the native
# host to compile the wasm module in question to avoid needing to run Cranelift
# under MIRI. That enables much faster iteration on the test here.

set -ex

cargo run --no-default-features --features compile,pulley,wat,gc-drc,component-model \
  compile --target pulley64 ./tests/all/pulley_provenance_test.wat \
  -o tests/all/pulley_provenance_test.cwasm \
  -O memory-reservation=$((1 << 20)) \
  -O memory-guard-size=0 \
  -O signals-based-traps=n \
  -W function-references

MIRIFLAGS="$MIRIFLAGS -Zmiri-disable-isolation -Zmiri-permissive-provenance" \
  cargo miri test --test all -- \
    --ignored pulley_provenance_test "$@"
