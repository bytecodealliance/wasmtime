#!/bin/bash
set -euo pipefail

# This is the test script for testing the no_std configuration of
# packages which support it.

# Repository top-level directory.
cd $(dirname "$0")
topdir=$(pwd)

function banner() {
    echo "======  $@  ======"
}

# Test those packages which have no_std support.
LIBS="codegen frontend wasm native module simplejit umbrella"
cd "$topdir"
for LIB in $LIBS
do
    banner "Rust unit tests in $LIB"
    cd "lib/$LIB"

    # Test with just "core" enabled.
    cargo +nightly test --no-default-features --features core

    # Test with "core" and "std" enabled at the same time.
    cargo +nightly test --features core

    cd "$topdir"
done

banner "OK"
