#!/bin/bash
set -euo pipefail

# This is the test script for testing the no_std configuration of
# packages which support it.

# Repository top-level directory.
topdir=$(dirname "$0")
cd "$topdir"

function banner {
    echo "======  $*  ======"
}

# Test those packages which have no_std support.
LIBS="cranelift-codegen cranelift-frontend cranelift-wasm \
cranelift-native cranelift-preopt cranelift-module \
cranelift-entity cranelift-bforest cranelift-umbrella"
for LIB in $LIBS; do
    banner "Rust unit tests in $LIB"
    pushd "$LIB" >/dev/null

    # Test with just "core" enabled.
    cargo +nightly test --no-default-features --features "core all-arch"

    # Test with "core" and "std" enabled at the same time.
    cargo +nightly test --features "core all-arch"

    popd >/dev/null
done

banner "OK"
