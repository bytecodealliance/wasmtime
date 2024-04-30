#!/bin/sh

# An example script to build and run the `min-platform` example by building both
# the embedding itself as well as the example host which will run it.
#
# This script takes a single argument which is a path to a Rust target json
# file. Examples are provided in `embedding/*.json`.
#
# This script must be executed with the current-working-directory as
# `examples/min-platform`.

target=$1
if [ "$target" = "" ]; then
  echo "Usage: $0 <target-json-file>"
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
    --lang C \
    --cpp-compat > embedding/wasmtime-platform.h
clang -shared -O2 -o libwasmtime-platform.so ./embedding/wasmtime-platform.c \
  -D_GNU_SOURCE

# Next the embedding itself is built. Points of note here:
#
# * `RUSTC_BOOTSTRAP_SYNTHETIC_TARGET=1` - this "fools" the Rust standard
#   library to falling back to an "unsupported" implementation of primitives by
#   default but without marking the standard library as requiring
#   `feature(restricted_std)`. This is probably something that should be
#   coordinated with upstream rust-lang/rust and get better support.
# * `--cfg=wasmtime_custom_platform` - this flag indicates to Wasmtime that the
#   minimal platform support is being opted into.
# * `-Zbuild-std=std,panic_abort` - this is a nightly Cargo feature to build the
#   Rust standard library from source.
#
# The final artifacts will be placed in Cargo's standard target directory.
RUSTC_BOOTSTRAP_SYNTHETIC_TARGET=1 \
RUSTFLAGS="--cfg=wasmtime_custom_platform" \
  cargo build -Zbuild-std=std,panic_abort \
    --manifest-path embedding/Cargo.toml \
    --target $target \
    --release

# The final step here is running the host, in the current directory, which will
# load the embedding and execute it.
cargo run --release -- $target
