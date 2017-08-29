#!/bin/bash
set -euo pipefail

# This is the top-level test script:
#
# - Build documentation for Rust code in 'src/tools/target/doc'.
# - Run unit tests for all Rust crates.
# - Make a debug build of all crates.
# - Make a release build of cton-util.
# - Run file-level tests with the release build of cton-util.
#
# All tests run by this script should be passing at all times.

# Disable generation of .pyc files because they cause trouble for vendoring
# scripts, and this is a build step that isn't run very often anyway.
export PYTHONDONTWRITEBYTECODE=1

# Repository top-level directory.
cd $(dirname "$0")
topdir=$(pwd)

function banner() {
    echo "======  $@  ======"
}

# Run rustfmt if we have it.
if $topdir/check-rustfmt.sh; then
    banner "Rust formatting"
    $topdir/format-all.sh --write-mode=diff
fi

# Check if any Python files have changed since we last checked them.
tsfile=$topdir/target/meta-checked
if [ -f $tsfile ]; then
    needcheck=$(find $topdir/lib/cretonne/meta -name '*.py' -newer $tsfile)
else
    needcheck=yes
fi
if [ -n "$needcheck" ]; then
    banner "$(python --version 2>&1), $(python3 --version 2>&1)"
    $topdir/lib/cretonne/meta/check.sh
    touch $tsfile || echo no target directory
fi

cd "$topdir"

# Make sure the code builds in debug mode.
banner "Rust debug build"
cargo build

# Make sure the code builds in release mode, and run the unit tests. We run
# these in release mode for speed, but note that the top-level Cargo.toml file
# does enable debug assertions in release builds.
banner "Rust release build and unit tests"
cargo test --all --release

# Make sure the documentation builds.
banner "Rust documentation: $topdir/target/doc/cretonne/index.html"
cargo doc

banner "OK"
