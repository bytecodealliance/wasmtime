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

set -ex

# First compile the C implementation of the platform symbols that will be
# required by our embedding. This is the `embedding/wasmtime-platform.c` file.
# The header file used is generated from Rust source code with the `cbindgen`
# utility which can be installed with:
#
#   cargo install cbindgen
#
# which ensures that Rust & C agree on types and such.
cbindgen ../../crates/wasmtime/src/runtime/vm/sys/custom/capi.rs \
    --config embedding/cbindgen.toml > embedding/wasmtime-platform.h
clang -shared -O2 -o libwasmtime-platform.so ./embedding/wasmtime-platform.c \
  -D_GNU_SOURCE

# Next the embedding itself is built.
#
# Note that this builds the embedding as a static library, here
# `libembedding.a`. This embedding is then turned into a dynamic library for the
# host platform using `cc` afterwards. The `*-unknown-none` targets themselves
# don't support dynamic libraries so this is a bit of a dance to get around the
# fact that we're pretending this examples in't being compiled for linux.
cargo build \
  --manifest-path embedding/Cargo.toml \
  --target $target \
  --release
cc \
  -Wl,--gc-sections \
  -Wl,--whole-archive \
  ../../target/$target/release/libembedding.a \
  -Wl,--no-whole-archive \
  -shared \
  -o libembedding.so

# The final step here is running the host, in the current directory, which will
# load the embedding and execute it.
cargo run --release -- $target
