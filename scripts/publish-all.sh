#!/bin/bash
set -euo pipefail

# This is a convenience script for maintainers publishing a new version of
# Wasmtime to crates.io. To use, bump the version number below, run the
# script, and then run the commands that the script prints.

topdir=$(dirname "$0")/..
cd "$topdir"

# All the wasmtime-* crates have the same version number
version="0.2.0"

# Update the version numbers of the crates to $version.
echo "Updating crate versions to $version"
find -name Cargo.toml -exec sed -i.bk -e "s/^version = .*/version = \"$version\"/" {} \;

# Update our local Cargo.lock (not checked in).
cargo update
scripts/test-all.sh

# Commands needed to publish.
#
# Note that libraries need to be published in topological order.

echo git commit -a -m "\"Bump version to $version"\"
echo git tag v$version
echo git push
echo git push origin v$version
echo "find -name Cargo.toml -exec scripts/cargo-chill.sh publish --manifest-path {} \\;"
