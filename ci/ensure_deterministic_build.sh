#!/bin/bash

# This script makes sure that the meta crate deterministically generate files
# with a high probability.
# The current directory must be set to the repository's root.
#
# We set SKIP_ISLE=1 to skip ISLE generation, because it depends on files
# in-tree (cranelift/codegen/.../*.isle) but these are not available when we
# switch to different working directories during this test.

set -e

BUILD_SCRIPT=$(find -wholename "./target/debug/build/cranelift-codegen-*/build-script-build")

# First, run the script to generate a reference comparison.
rm -rf /tmp/reference
mkdir /tmp/reference
SKIP_ISLE=1 OUT_DIR=/tmp/reference TARGET=x86_64 CARGO_PKG_VERSION=0.1.0 CARGO_MANIFEST_DIR=/tmp $BUILD_SCRIPT

# To make sure the build script doesn't depend on the current directory, we'll
# change the current working directory on every iteration. Make this easy to
# reproduce this locally by first copying the target/ directory into an initial
# temporary directory (and not move and lose the local clone's content).
rm -rf /tmp/src0
mkdir /tmp/src0

echo Copying target directory...
cp -r ./target /tmp/src0/target
cd /tmp/src0
echo "Done, starting loop."

# Then, repeatedly make sure that the output is the same.
for i in {1..20}
do
    # Move to a different directory, as explained above.
    rm -rf /tmp/src$i
    mkdir /tmp/src$i
    mv ./* /tmp/src$i
    cd /tmp/src$i

    rm -rf /tmp/try
    mkdir /tmp/try
    SKIP_ISLE=1 OUT_DIR=/tmp/try TARGET=x86_64 CARGO_PKG_VERSION=0.1.0 CARGO_MANIFEST_DIR=/tmp/src$i $BUILD_SCRIPT
    diff -qr /tmp/reference /tmp/try
done
