#!/bin/bash

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
