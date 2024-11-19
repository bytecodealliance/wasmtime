#!/bin/sh

# An example script to build and run the `min-platform` example by building both
# the embedding itself as well as the example host which will run it.
#
# This script takes a single argument which is a path to a Rust target json
# file. Example targets are `x86_64-unknown-none` or `aarch64-unknown-none`.
#
# This script must be executed with the current-working-directory as
# `examples/min-platform`.

target=$1
if [ "$target" = "" ]; then
  echo "Usage: $0 <target>"
  exit 1
fi

REPO_DIR=$(dirname $0)/../..
HOST_DIR=$REPO_DIR/examples/min-platform
EMBEDDING_DIR=$HOST_DIR/embedding

set -ex

if [ "$WASMTIME_SIGNALS_BASED_TRAPS" = "1" ]; then
  cflags="$cflags -DWASMTIME_SIGNALS_BASED_TRAPS"
  features="$features,signals-based-traps"
fi

# First compile the C implementation of the platform symbols that will be
# required by our embedding. This is the `embedding/wasmtime-platform.c` file.
# The header file used is generated from Rust source code with the `cbindgen`
# utility which can be installed with:
#
#   cargo install cbindgen
#
# which ensures that Rust & C agree on types and such.
cbindgen "$REPO_DIR/crates/wasmtime/src/runtime/vm/sys/custom/capi.rs" \
    --config "$EMBEDDING_DIR/cbindgen.toml" > "$EMBEDDING_DIR/wasmtime-platform.h"
clang -shared -O2 -o "$HOST_DIR/libwasmtime-platform.so" "$EMBEDDING_DIR/wasmtime-platform.c" \
  -D_GNU_SOURCE $cflags

# Next the embedding itself is built.
#
# Note that this builds the embedding as a static library, here
# `libembedding.a`. This embedding is then turned into a dynamic library for the
# host platform using `cc` afterwards. The `*-unknown-none` targets themselves
# don't support dynamic libraries so this is a bit of a dance to get around the
# fact that we're pretending this examples in't being compiled for linux.
cargo build \
  --manifest-path $EMBEDDING_DIR/Cargo.toml \
  --target $target \
  --features "$features" \
  --release
cc \
  -Wl,--gc-sections \
  -Wl,--whole-archive \
  "$REPO_DIR/target/$target/release/libembedding.a" \
  -Wl,--no-whole-archive \
  -shared \
  -o "$HOST_DIR/libembedding.so"

# The final step here is running the host, in the current directory, which will
# load the embedding and execute it.
cargo run --manifest-path "$HOST_DIR/Cargo.toml" --release --features "$features" -- \
  "$target" \
  "$HOST_DIR/libembedding.so" \
  "$HOST_DIR/libwasmtime-platform.so"
