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

PKGS="cretonne cretonne-reader cretonne-tools cretonne-frontend cretonne-wasm \
      filecheck "
cd "$topdir"
for PKG in $PKGS
do
    banner "Rust $PKG unit tests"
    cargo test -p $PKG
done

# Build cton-util for parser testing.
cd "$topdir"
banner "Rust documentation"
echo "open $topdir/target/doc/cretonne/index.html"
cargo doc
banner "Rust release build"
cargo build --release

export CTONUTIL="$topdir/target/release/cton-util"

cd "$topdir"
banner "File tests"
"$CTONUTIL" test filetests docs

banner "OK"
