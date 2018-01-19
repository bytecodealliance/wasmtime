#!/bin/bash

# This is the test script for testing the no_std configuration of
# packages which support it.

# Exit immediately on errors.
set -e

# Repository top-level directory.
cd $(dirname "$0")
topdir=$(pwd)

function banner() {
    echo "======  $@  ======"
}

# Test those packages which have no_std support.
LIBS="cretonne frontend wasm native"
cd "$topdir"
for LIB in $LIBS
do
    banner "Rust unit tests in $LIB"
    cd "lib/$LIB"
    cargo test --features no_std
    cd "$topdir"
done

banner "OK"
