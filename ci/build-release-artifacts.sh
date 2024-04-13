#!/bin/bash

# A script to build the release artifacts of Wasmtime into the `target`
# directory. For now this is the CLI and the C API. Note that this script only
# produces the artifacts through Cargo and doesn't package things up. That's
# intended for the `build-tarballs.sh` script.
#
# This script takes a Rust target as its first input and optionally a parameter
# afterwards which can be "-min" to indicate that a minimal build should be
# produced with as many features as possible stripped out.

set -ex

build=$1
target=$2

# Default build flags for release artifacts. Leave debugging for
# builds-from-source which have richer information anyway, and additionally the
# CLI won't benefit from catching unwinds and neither will the C API so use
# panic=abort in both situations.
export CARGO_PROFILE_RELEASE_STRIP=debuginfo
export CARGO_PROFILE_RELEASE_PANIC=abort

if [[ "$build" = *-min ]]; then
  # Configure a whole bunch of compile-time options which help reduce the size
  # of the binary artifact produced.
  export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
  export RUSTFLAGS=-Zlocation-detail=none
  export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
  export CARGO_PROFILE_RELEASE_LTO=true
  flags="-Zbuild-std=std,panic_abort --no-default-features -Zbuild-std-features=std_detect_dlsym_getauxval"
  flags="$flags --features disable-logging"
else
  # For release builds the CLI is built a bit more feature-ful than the Cargo
  # defaults to provide artifacts that can do as much as possible.
  bin_flags="--features all-arch,component-model"
fi

cargo build --release $flags --target $target -p wasmtime-cli $bin_flags --features run
cargo build --release $flags --target $target -p wasmtime-c-api
