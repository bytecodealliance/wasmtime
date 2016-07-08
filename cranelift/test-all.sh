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

# Run cargo from the src/tools directory which includes all our crates for
# building cton-util.
cd "$topdir/src/tools"
PKGS="-p cretonne -p cretonne-reader -p cretonne-tools"
echo ====== Rust unit tests and debug build ======
cargo test $PKGS
cargo build $PKGS
cargo doc

echo ====== Rust release build ======
cargo build --release

export CTONUTIL="$topdir/src/tools/target/release/cton-util"

# Run the parser tests.
echo ====== Parser tests ======
cd "$topdir/tests"
parser/run.sh

echo ====== OK ======
