#!/bin/bash
set -euo pipefail

# This is a convenience script for maintainers publishing a new version of
# Wasmtime to crates.io. To use, bump the version number below, run the
# script, and then run the commands that the script prints.

topdir=$(dirname "$0")/..
cd "$topdir"

# All the wasmtime-* crates have the same version number
short_version="19"
version="0.$short_version.0"

# Update the version numbers of the crates to $version. Skip crates with
# a version of "0.0.0", which are unpublished.
echo "Updating crate versions to $version"
find crates -name Cargo.toml \
    -not -path crates/wasi-common/WASI/tools/witx/Cargo.toml \
    -exec sed -i.bk -e "s/^version = \"[.*[^0.].*\"$/version = \"$version\"/" {} \;

# Updat the top-level Cargo.toml too.
sed -i.bk -e "s/^version = \"[.*[^0.].*\"$/version = \"$version\"/" Cargo.toml

# Update the required version numbers of path dependencies.
find -name Cargo.toml \
    -not -path ./crates/wasi-common/WASI/tools/witx/Cargo.toml \
    -exec sed -i.bk \
    -e "/^\(wasmtime\|wiggle\)/s/version = \"[^\"]*\"/version = \"$version\"/" \
    {} \;
find -name Cargo.toml \
    -not -path ./crates/wasi-common/WASI/tools/witx/Cargo.toml \
    -exec sed -i.bk \
    -e "/^\(wasi-common\|wig\|yanix\|winx\|lightbeam\) = /s/version = \"[^\"]*\"/version = \"$version\"/" \
    {} \;

find crates -type f -print0 | xargs -0 sed -i \
    "s/wasi-common-[0-9][0-9]*/wasi-common-$short_version/"
find crates -type f -print0 | xargs -0 sed -i \
    "s/DEP_WASI_COMMON_[0-9][0-9]*/DEP_WASI_COMMON_${short_version}/"

# Update the Cargo.lock files for the new versions.
cargo update
cd crates/test-programs/wasi-tests
cargo update
cd - >/dev/null
