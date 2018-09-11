#!/bin/bash
set -euo pipefail

# This is a convenience script for maintainers publishing a new version of
# Cranelift to crates.io. To use, bump the version number below, run the
# script, and then run the commands that the script prints.

topdir=$(dirname "$0")
cd "$topdir"

# All the cranelift-* crates have the same version number
version="0.21.1"

# Update all of the Cargo.toml files.
#
# The main Cargo.toml in the top-level directory is the cranelift-tools crate which we don't publish.
echo "Updating crate versions to $version"
for crate in . lib/* lib/codegen/meta; do
    # Update the version number of this crate to $version.
    sed -i.bk -e "s/^version = .*/version = \"$version\"/" \
        "$crate/Cargo.toml"

    # Update the required version number of any cranelift* dependencies.
    sed -i.bk -e "/^cranelift/s/version = \"[^\"]*\"/version = \"$version\"/" \
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
    entity bforest codegen/meta codegen frontend native \
    reader wasm module \
    faerie umbrella simplejit
do
    echo cargo publish --manifest-path "lib/$crate/Cargo.toml"
done
