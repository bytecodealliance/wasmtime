#!/bin/bash
set -euo pipefail

# This is a convenience script for maintainers publishing a new version of
# Wasmtime to crates.io. To use, bump the version number below, run the
# script, and then run the commands that the script prints.

topdir=$(dirname "$0")
cd "$topdir"

# All the wasmtime-* crates have the same version number
version="0.1.0"

# Update all of the Cargo.toml files.
#
# The main Cargo.toml in the top-level directory is the wasmtime crate which we don't publish.
echo "Updating crate versions to $version"
for crate in . wasmtime-*; do
    # Update the version number of this crate to $version.
    sed -i.bk -e "s/^version = .*/version = \"$version\"/" \
        "$crate/Cargo.toml"

    # Update the required version number of any wasmtime* dependencies.
    sed -i.bk -e "/^wasmtime/s/version = \"[^\"]*\"/version = \"$version\"/" \
        "$crate/Cargo.toml"
done

# Update our local Cargo.lock (not checked in).
cargo update
./test-all.sh

# Commands needed to publish.
#
# Note that libraries need to be published in topological order.

echo git commit -a -m "\"Bump version to $version"\"
echo git push
for crate in \
    wasmtime-environ
do
    echo cargo publish --manifest-path "$crate/Cargo.toml"
done
