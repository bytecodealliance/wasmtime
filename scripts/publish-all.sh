#!/bin/bash
set -euo pipefail

# This is a convenience script for maintainers publishing a new version of
# Wasmtime to crates.io. To use, bump the version number below, run the
# script, and then run the commands that the script prints.

topdir=$(dirname "$0")/..
cd "$topdir"

# All the wasmtime-* crates have the same version number
version="0.9.0"

# Update the version numbers of the crates to $version.
echo "Updating crate versions to $version"
find -name Cargo.toml \
    -not -path ./crates/wasi-common/WASI/tools/witx/Cargo.toml \
    -exec sed -i.bk -e "s/^version = \"[[:digit:]].*/version = \"$version\"/" {} \;

# Update our local Cargo.lock (not checked in).
cargo update
scripts/test-all.sh

# Commands needed to publish.
#
# Note that libraries need to be published in topological order.

echo git checkout -b bump-version-to-$version
echo git commit -a -m "\"Bump version to $version"\"
echo git tag v$version
echo git push origin bump-version-to-$version
echo "# Don't forget to click the above link to open a pull-request!"
echo git push origin v$version
for cargo_toml in \
    crates/wasi-common/wasi-common-cbindgen/Cargo.toml \
    crates/wasi-common/winx/Cargo.toml \
    crates/wasi-common/wig/Cargo.toml \
    crates/wasi-common/Cargo.toml \
    crates/lightbeam/Cargo.toml \
    crates/environ/Cargo.toml \
    crates/obj/Cargo.toml \
    crates/runtime/Cargo.toml \
    crates/debug/Cargo.toml \
    crates/jit/Cargo.toml \
    crates/wasi-c/Cargo.toml \
    crates/api/Cargo.toml \
    crates/wasi/Cargo.toml \
    crates/wast/Cargo.toml \
    crates/interface-types/Cargo.toml \
    crates/misc/py/Cargo.toml \
    crates/misc/rust/macro/Cargo.toml \
    crates/misc/rust/Cargo.toml \
; do
    version=""
    case $cargo_toml in
        crates/lightbeam/Cargo.toml) version=" +nightly" ;;
        crates/misc/py/Cargo.toml) version=" +nightly" ;;
    esac

    echo cargo$version publish --manifest-path "$cargo_toml"

    # Sleep for a few seconds to allow the server to update the index.
    # https://internals.rust-lang.org/t/changes-to-how-crates-io-handles-index-updates/9608
    echo sleep 10
done
