#!/bin/bash
set -euo pipefail

# This is a convenience script for maintainers publishing a new version of
# Cranelift to crates.io. To use, bump the version number below, run the
# script, and then run the commands that the script prints.

topdir=$(dirname "$0")
cd "$topdir"

# All the cranelift-* crates have the same version number
version="0.54.0"

# Update all of the Cargo.toml files.
#
# The main Cargo.toml in the top-level directory is the cranelift-tools crate which we don't publish.
echo "Updating crate versions to $version"
for crate in . cranelift-* cranelift-codegen/shared cranelift-codegen/meta; do
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

echo git checkout -b bump-version-to-$version
echo git commit -a -m "\"Bump version to $version"\"
echo git tag v$version
echo git push origin bump-version-to-$version
echo "# Don't forget to click the above link to open a pull-request!"
echo git push origin v$version
for crate in \
    entity bforest codegen/shared codegen/meta codegen frontend native \
    preopt \
    reader wasm module \
    faerie umbrella simplejit object
do
    echo cargo publish --manifest-path "cranelift-$crate/Cargo.toml"

    # Sleep for a few seconds to allow the server to update the index.
    # https://internals.rust-lang.org/t/changes-to-how-crates-io-handles-index-updates/9608
    echo sleep 10
done
echo
echo "echo \"#\""
echo "echo \"# Don't forget to click the above link to open a pull-request!\""
echo "echo \"#\""
