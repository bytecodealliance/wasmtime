#!/bin/bash

# Format all sources using rustfmt.

# Exit immediately on errors.
set -e

cd $(dirname "$0")
src=$(pwd)

# Make sure we can find rustfmt.
export PATH="$PATH:$HOME/.cargo/bin"

for crate in $(find "$src" -name Cargo.toml); do
    cd $(dirname "$crate")
    cargo fmt -- "$@"
done
