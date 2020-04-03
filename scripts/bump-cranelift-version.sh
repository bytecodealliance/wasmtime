#!/bin/bash
set -euo pipefail

# This is a convenience script for maintainers changing a cranelift
# dependencies versions. To use, bump the version number below, run the
# script.

topdir=$(dirname "$0")/..
cd "$topdir"

# All the cranelift-* crates have the same version number
version="0.61.0"

# Update all of the Cargo.toml files.
echo "Updating crate versions to $version"
for toml in cranelift/Cargo.toml cranelift/*/Cargo.toml cranelift/*/*/Cargo.toml; do
    # Update the version number of this crate to $version.
    sed -i.bk -e "/^version = /s/\"[^\"]*\"/\"$version\"/" \
        "$toml"
done

# Update the required version numbers of path dependencies.
find -name Cargo.toml \
    -not -path ./crates/wasi-common/WASI/tools/witx/Cargo.toml \
    -exec sed -i.bk \
        -e "/^cranelift/s/version = \"[^\"]*\"/version = \"$version\"/" \
        {} \;

# Update the Cargo.lock file for the new versions.
cargo update
