#!/bin/bash
set -euo pipefail

# This is a convenience script for maintainers publishing a new version of
# Cranelift to crates.io. To use, first bump the version number by running the
# `scripts/bump-cranelift-version.sh` script, then run this script, and run the
# commands that it prints.
#
# Don't forget to push a git tag for this release!

topdir=$(dirname "$0")/..
cd "$topdir"

# Commands needed to publish.
#
# Note that libraries need to be published in topological order.

for crate in \
    entity \
    bforest \
    codegen/shared \
    codegen/meta \
    codegen \
    frontend \
    native \
    preopt \
    reader \
    wasm \
    module \
    faerie \
    umbrella \
    simplejit \
    object
do
    echo cargo publish --manifest-path "cranelift/$crate/Cargo.toml"

    # Sleep for a few seconds to allow the server to update the index.
    # https://internals.rust-lang.org/t/changes-to-how-crates-io-handles-index-updates/9608
    echo sleep 30
done

echo git tag cranelift-v$(grep version cranelift/Cargo.toml | head -n 1 | cut -d '"' -f 2)
