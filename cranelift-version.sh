#!/bin/bash
set -euo pipefail

# This is a convenience script for maintainers changing a cranelift
# dependencies versions. To use, bump the version number below, run the
# script.

topdir=$(dirname "$0")
cd "$topdir"

# All the cranelift-* crates have the same version number
version="0.44.0"

# Update all of the Cargo.toml files.
echo "Updating crate versions to $version"
for crate in . wasmtime-* fuzz misc/wasmtime-*; do
    # Update the version number of this crate to $version.
    sed -i.bk -e "/^cranelift-/s/\"[^\"]*\"/\"$version\"/" \
        "$crate/Cargo.toml"

    # Update the required version number of any cranelift* dependencies.
    sed -i.bk -e "/^cranelift-/s/version = \"[^\"]*\"/version = \"$version\"/" \
        "$crate/Cargo.toml"
done
