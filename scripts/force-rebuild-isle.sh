#!/bin/sh
#
# This script rebuilds all ISLE generated source that is checked in, even if
# the source has not changed relative to the manifests.
#
# This is useful when one is developing the ISLE compiler itself; otherwise,
# changing the compiler does not automatically change the generated code, even
# if the `rebuild-isle` feature is specified.

set -e

# Remove the manifests (which contain hashes of ISLE source) to force the build
# script to regenerate all backends.
rm -f cranelift/codegen/src/isa/*/lower/isle/generated_code.manifest

# `cargo check` will both invoke the build script to rebuild the backends, and
# check that the output is valid Rust. We specify `all-arch` here to include
# all backends.
cargo check -p cranelift-codegen --features rebuild-isle,all-arch
