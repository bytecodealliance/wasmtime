#!/bin/bash
set -euo pipefail

# This is a convenience script for maintainers publishing a new version of
# Wasmtime to crates.io. To use, first bump the Wasmtime versions with
# `scripts/bump-wasmtime-version.sh` and Cranelift versions with
# `scripts/bump-cranelift-version.sh`, then run this script, and run the
# commands that it prints.

topdir=$(dirname "$0")/..
cd "$topdir"

# Publishing Wasmtime requires publishing any local Cranelift changes.
scripts/publish-cranelift.sh

# Commands needed to publish.
#
# Note that libraries need to be published in topological order.

for cargo_toml in \
    crates/wasi-common/winx/Cargo.toml \
    crates/wasi-common/yanix/Cargo.toml \
    crates/wasi-common/wig/Cargo.toml \
    crates/wiggle/crates/runtime/Cargo.toml \
    crates/wiggle/crates/generate/Cargo.toml \
    crates/wiggle/crates/test/Cargo.toml \
    crates/wiggle/Cargo.toml \
    crates/wasi-common/Cargo.toml \
    crates/lightbeam/Cargo.toml \
    crates/environ/Cargo.toml \
    crates/obj/Cargo.toml \
    crates/runtime/Cargo.toml \
    crates/profiling/Cargo.toml \
    crates/debug/Cargo.toml \
    crates/jit/Cargo.toml \
    crates/api/Cargo.toml \
    crates/wasi/Cargo.toml \
    crates/wast/Cargo.toml \
    crates/misc/py/Cargo.toml \
    crates/misc/rust/macro/Cargo.toml \
    crates/misc/rust/Cargo.toml \
    Cargo.toml \
; do
    version=""
    case $cargo_toml in
        crates/lightbeam/Cargo.toml) version=" +nightly" ;;
        crates/misc/py/Cargo.toml) version=" +nightly" ;;
    esac

    echo cargo$version publish --manifest-path "$cargo_toml"

    # Sleep for a few seconds to allow the server to update the index.
    # https://internals.rust-lang.org/t/changes-to-how-crates-io-handles-index-updates/9608
    echo sleep 20
done

echo echo git tag v$(grep "version =" Cargo.toml | head -n 1 | cut -d '"' -f 2)
