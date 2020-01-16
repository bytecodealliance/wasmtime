#!/bin/bash
set -euo pipefail

# This is a convenience script for maintainers changing a cranelift
# dependencies versions. To use, bump the version number below, run the
# script.

topdir=$(dirname "$0")/..
cd "$topdir"

# All the cranelift-* crates have the same version number
version="0.55"

# Update all of the Cargo.toml files.
echo "Updating crate versions to $version"
for toml in Cargo.toml crates/*/Cargo.toml crates/misc/*/Cargo.toml fuzz/Cargo.toml; do
    # Update the version number of this crate to $version.
    sed -i.bk -e "/^cranelift-/s/\"[^\"]*\"/\"$version\"/" \
        "$toml"

    # Update the required version number of any cranelift* dependencies.
    sed -i.bk -e "/^cranelift-/s/version = \"[^\"]*\"/version = \"$version\"/" \
        "$toml"
done
