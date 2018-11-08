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
LIBS="codegen frontend wasm native preopt module simplejit umbrella"
for LIB in $LIBS; do
    banner "Rust unit tests in $LIB"
    pushd "lib/$LIB" >/dev/null

    # Test with just "core" enabled.
    cargo +nightly test --no-default-features --features core

    # Test with "core" and "std" enabled at the same time.
    cargo +nightly test --features core

    popd >/dev/null
done

banner "OK"
