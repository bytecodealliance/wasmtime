#!/bin/bash

# Format all sources using rustfmt.

# Exit immediately on errors.
set -e

cd $(dirname "$0")
src=$(pwd)

for crate in $(find "$src" -name Cargo.toml); do
    cd $(dirname "$crate")
    cargo fmt
done
