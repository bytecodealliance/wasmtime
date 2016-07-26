#!/bin/bash

# This is the top-level test script:
#
# - Build documentation for Rust code in 'src/tools/target/doc'.
# - Run unit tests for all Rust crates.
# - Make a debug build of all crates.
# - Make a release build of cton-util.
# - Run file-level tests with the release build of cton-util.
#
# All tests run by this script should be passing at all times.

# Exit immediately on errors.
set -e

# Repository top-level directory.
cd $(dirname "$0")
topdir=$(pwd)

PKGS="libcretonne libreader tools"
echo ====== Rust unit tests and debug builds ======
for PKG in $PKGS
do
    pushd $topdir/src/$PKG
    cargo test
    cargo build
    popd
done

# Build cton-util for parser testing.
echo ====== Rust release build and documentation ======
cd "$topdir/src/tools"
cargo doc
cargo build --release

export CTONUTIL="$topdir/src/tools/target/release/cton-util"

# Run the parser tests.
echo ====== Parser tests ======
cd "$topdir/tests"
parser/run.sh
cfg/run.sh

echo ====== OK ======
