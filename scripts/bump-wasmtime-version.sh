#!/bin/bash
set -euo pipefail

# This is a convenience script for maintainers publishing a new version of
# Wasmtime to crates.io. To use, bump the version number below, run the
# script, and then run the commands that the script prints.

topdir=$(dirname "$0")/..
cd "$topdir"

# All the wasmtime-* crates have the same version number
version="0.10.0"

# Update the version numbers of the crates to $version.
echo "Updating crate versions to $version"
find -name Cargo.toml \
    -not -path ./crates/wasi-common/wig/WASI/tools/witx/Cargo.toml \
    -exec sed -i.bk -e "s/^version = \"[[:digit:]].*/version = \"$version\"/" {} \;

# Update the required version numbers of path dependencies.
find -name Cargo.toml \
    -not -path ./crates/wasi-common/wig/WASI/tools/witx/Cargo.toml \
    -exec sed -i.bk \
        -e "/\> *= *{.*\<path *= *\"/s/version = \"[^\"]*\"/version = \"$version\"/" \
        {} \;

cargo build
